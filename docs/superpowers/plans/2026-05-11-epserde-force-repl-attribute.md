# `#[epserde(force_repl(...))]` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a struct/enum-level `#[epserde(force_repl(T, U, ...))]` attribute to the `Epserde` derive macro that forces named type parameters to be treated as transitively replaceable in `Self::DeserType<'a>` and `Self::SerType`.

**Architecture:** Localized to `epserde-derive/src/lib.rs`. Four changes: (1) parse a new attribute arm into `EpserdeAttrs`; (2) validate the named idents and plumb them into `EpserdeContext`; (3) introduce a `type_contains_any` helper and change `gen_eps_deser_method_call` to dispatch on type-containment instead of single-segment match; (4) update the struct and enum impl generators to compute a unified `repl_params = natural ∪ force_repl` upfront. Failure mode for a violated contract is a compile error in the derived `_deser_eps_inner` body.

**Tech Stack:** Rust 2024 (MSRV 1.85), `syn` 2 / `quote` 1 (proc-macro crate), `trybuild` for compile-fail tests.

**Spec:** `docs/superpowers/specs/2026-05-11-epserde-enforce-repl-attribute-design.md`

---

## Task 1: Parse the `force_repl(...)` attribute

**Files:**

- Modify: `epserde-derive/src/lib.rs` (the `EpserdeAttrs` struct and `parse_epserde_attrs` function)

- [ ] **Step 1: Add the field to `EpserdeAttrs`**

Locate the `EpserdeAttrs` struct definition (around line 159). Add a new field at the bottom of the struct:

```rust
/// Parsed epserde attributes.
struct EpserdeAttrs {
    /// Whether the type has `#[repr(C)]`.
    is_repr_c: bool,
    /// Whether `#[epserde(zero_copy)]` or `#[epserde(zero_copy)]` was specified.
    is_zero_copy: bool,
    /// Whether `#[epserde(deep_copy)]` or `#[epserde_deep_copy]` was specified.
    is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl.
    deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl.
    ser_bounds: Vec<WherePredicate>,
    /// Whether old-style `#[epserde(zero_copy)]` was used.
    deprecated_zero_copy: bool,
    /// Whether old-style `#[epserde_deep_copy]` was used.
    deprecated_deep_copy: bool,
    /// Type-parameter idents listed in `#[epserde(force_repl(...))]`.
    force_repl: Vec<syn::Ident>,
}
```

- [ ] **Step 2: Initialize and parse the field**

In `parse_epserde_attrs`, locate the block that initializes the locals (around line 183). Add `force_repl` next to the others:

```rust
    let mut is_zero_copy = false;
    let mut is_deep_copy = false;
    let mut deser_bounds = Vec::new();
    let mut ser_bounds = Vec::new();
    let mut deprecated_zero_copy = false;
    let mut deprecated_deep_copy = false;
    let mut force_repl: Vec<syn::Ident> = Vec::new();
```

In the same function, find the nested-meta walk (the `attr.parse_nested_meta(|meta| { ... })` block). Locate the branch that handles `bound` (it starts with `} else if meta.path.is_ident("bound") {`). Add a new branch immediately _before_ the final `else` branch:

```rust
                } else if meta.path.is_ident("force_repl") {
                    meta.parse_nested_meta(|inner| {
                        let ident = inner.path.require_ident()?.clone();
                        force_repl.push(ident);
                        Ok(())
                    })
                } else {
                    Err(meta.error("expected `zero_copy`, `deep_copy`, `bound`, or `force_repl`"))
                }
```

Note: replace the existing error message in the `else` branch ("expected `zero_copy`, `deep_copy`, or `bound`") with the new one listing `force_repl`. The project convention is to use backticks (not single quotes) around code-like names in error messages (see commit `433d7a4` "No more single quotation marks in error messages"); apply the same convention everywhere in this plan.

- [ ] **Step 3: Return the parsed value**

At the bottom of `parse_epserde_attrs`, find the `Ok(EpserdeAttrs { ... })` block and add the new field:

```rust
    Ok(EpserdeAttrs {
        is_repr_c,
        is_zero_copy,
        is_deep_copy,
        deser_bounds,
        ser_bounds,
        deprecated_zero_copy,
        deprecated_deep_copy,
        force_repl,
    })
```

- [ ] **Step 4: Build and verify nothing regressed**

Run: `cargo build --all-features`
Expected: clean build with no warnings related to the new field. The existing test suite should still compile.

- [ ] **Step 5: Commit**

```bash
git add epserde-derive/src/lib.rs
git commit -m "$(cat <<'EOF'
Parse force_repl attribute in EpserdeAttrs

Adds parsing of #[epserde(force_repl(T, U, ...))] into a
Vec<syn::Ident> on EpserdeAttrs. No behavior change yet; the parsed
list is unused until subsequent tasks plumb it through EpserdeContext.
EOF
)"
```

---

## Task 2: Validate `force_repl` idents and plumb into `EpserdeContext`

**Files:**

- Modify: `epserde-derive/src/lib.rs` (the `EpserdeContext` struct, `epserde_derive` entry point)

- [ ] **Step 1: Add the field to `EpserdeContext`**

Locate `EpserdeContext` (around line 490). Add a new field at the bottom:

```rust
struct EpserdeContext<'a> {
    /// The original derive input.
    derive_input: &'a DeriveInput,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_params: Vec<&'a syn::Ident>,
    /// Identifiers of type parameters as a set.
    type_params: HashSet<&'a syn::Ident>,
    /// Generics for the `impl` clause as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics_for_impl: ImplGenerics<'a>,
    /// Generics for the type as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics_for_type: TypeGenerics<'a>,
    /// The where clause for the type being derived.
    where_clause: &'a WhereClause,
    /// Whether the type has `#[repr(C)]`
    is_repr_c: bool,
    /// Whether the type has `#[epserde(zero_copy)]`
    is_zero_copy: bool,
    /// Whether the type has `#[epserde(deep_copy)]`
    is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl from
    /// `#[epserde(bound(deser = "..."))]`.
    deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl from
    /// `#[epserde(bound(ser = "..."))]`.
    ser_bounds: Vec<WherePredicate>,
    /// Type-parameter idents listed in `#[epserde(force_repl(...))]`,
    /// validated against `type_params`.
    force_repl: Vec<syn::Ident>,
}
```

- [ ] **Step 2: Validate after parsing**

In `epserde_derive` (around line 1030), after the call to `get_type_const_params` and before constructing `EpserdeContext`, insert validation. The relevant section currently looks like:

```rust
    let (type_const_params, type_params, const_params) = match get_type_const_params(&derive_input)
    {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    emit_deprecation_warnings(&attrs, &derive_input.ident);
```

Add validation immediately after the `emit_deprecation_warnings` call:

```rust
    emit_deprecation_warnings(&attrs, &derive_input.ident);

    // Validate `force_repl` idents: each must be a declared type parameter.
    for ident in &attrs.force_repl {
        if !type_params.contains(ident) {
            return syn::Error::new_spanned(
                ident,
                format!(
                    "`{}` is not a generic type parameter of this item",
                    ident
                ),
            )
            .to_compile_error()
            .into();
        }
    }

    // `force_repl` is incompatible with zero-copy types.
    if attrs.is_zero_copy && !attrs.force_repl.is_empty() {
        return syn::Error::new_spanned(
            &attrs.force_repl[0],
            "`force_repl` cannot be used with zero-copy types",
        )
        .to_compile_error()
        .into();
    }
```

- [ ] **Step 3: Plumb the validated list into `EpserdeContext`**

In the same function, update the `EpserdeContext { ... }` literal to include the new field:

```rust
    let ctx = EpserdeContext {
        derive_input: &derive_input,
        type_const_params,
        type_params,
        generics_for_impl,
        generics_for_type,
        where_clause,
        is_repr_c: attrs.is_repr_c,
        is_zero_copy: attrs.is_zero_copy,
        is_deep_copy: attrs.is_deep_copy,
        deser_bounds: attrs.deser_bounds,
        ser_bounds: attrs.ser_bounds,
        force_repl: attrs.force_repl,
    };
```

- [ ] **Step 4: Build**

Run: `cargo build --all-features`
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add epserde-derive/src/lib.rs
git commit -m "$(cat <<'EOF'
Validate force_repl idents and store in EpserdeContext

Rejects 'force_repl(X)' when X is not a generic type parameter and
rejects 'force_repl' on zero-copy types. The validated list is stored
on EpserdeContext for use by the struct/enum impl generators in the
next task.
EOF
)"
```

---

## Task 3: Add `type_contains_any` helper and change field dispatch

**Files:**

- Modify: `epserde-derive/src/lib.rs` (add helper, modify `gen_eps_deser_method_call`)

- [ ] **Step 1: Add the `type_contains_any` helper**

Insert the new function immediately _after_ the existing `get_ident` function (around line 66):

```rust
/// Returns `true` if `ty` syntactically contains any identifier in `params`
/// at any position (path segment, type argument, tuple element, etc.).
///
/// Used to decide whether a field should be ε-copy deserialized: a field
/// whose type mentions a replaceable parameter must be ε-copy deserialized
/// so that the result's type matches the corresponding slot in the parent's
/// substituted `DeserType<'_>`.
///
/// Recurses into the variants of [`syn::Type`] that epserde supports:
/// `Path`, `Tuple`, `Array`, `Slice`, `Paren`, and `Group`. All other
/// variants return `false`.
fn type_contains_any(ty: &syn::Type, params: &HashSet<&syn::Ident>) -> bool {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            for segment in &path.segments {
                if params.contains(&segment.ident) {
                    return true;
                }
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    for arg in &ab.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            if type_contains_any(t, params) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        syn::Type::Tuple(t) => t.elems.iter().any(|e| type_contains_any(e, params)),
        syn::Type::Array(a) => type_contains_any(&a.elem, params),
        syn::Type::Slice(s) => type_contains_any(&s.elem, params),
        syn::Type::Paren(p) => type_contains_any(&p.elem, params),
        syn::Type::Group(g) => type_contains_any(&g.elem, params),
        _ => false,
    }
}
```

- [ ] **Step 2: Change `gen_eps_deser_method_call` to use the helper**

Locate `gen_eps_deser_method_call` (around line 77). Rename its third parameter from `type_params` to `repl_params` (clarifying intent) and replace the single-segment check with a containment check. The full function becomes:

```rust
/// Generates a method call for field ε-copy deserialization.
///
/// Takes care of choosing `_deser_eps_inner` or `_deser_full_inner`
/// depending on whether the field type mentions a replaceable parameter,
/// and uses the special method `_deser_eps_inner_special` for
/// `PhantomDeserData`.
///
/// The type of `field_name` is [`proc_macro2::TokenStream`] because it
/// can be either an identifier (for named fields) or an index (for
/// unnamed fields).
fn gen_eps_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    repl_params: &HashSet<&syn::Ident>,
) -> proc_macro2::TokenStream {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments,
        },
    }) = field_type
    {
        // This is a pretty weak check, as a user could define its own
        // PhantomDeserData, but it should be good enough in practice
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: unsafe { <#field_type>::_deser_eps_inner_special(backend)? });
            }
        }
    }

    // If the field type mentions any replaceable parameter we proceed
    // with ε-copy deserialization; otherwise full-copy.
    if type_contains_any(field_type, repl_params) {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_eps_inner(backend)? })
    } else {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_full_inner(backend)? })
    }
}
```

- [ ] **Step 3: Build**

The change in Step 2 affects callers (struct and enum impl generators), which currently pass `&ctx.type_params` or `&all_repl_params`. These callers will still compile because the parameter type didn't change — only its name and semantic meaning. We fix the callers in Tasks 4 and 5.

Run: `cargo build --all-features`
Expected: clean build. The existing test suite _will_ change behavior subtly — see the note below — but should still pass because every shape it currently tests is preserved under the new dispatch.

Note on behavior: with this task's change alone (before Tasks 4–5), the struct case will start passing `&ctx.type_params` as `repl_params`, meaning _any_ field type that mentions any type parameter (not just a direct single-segment one) will switch to `_deser_eps_inner`. This is the relaxation we want and lifts the implicit "appears twice" invariant. Tasks 4–5 narrow this from "any type param" to "naturally replaceable + `force_repl`".

- [ ] **Step 4: Run tests**

Run: `cargo test -- --skip fail`
Expected: PASS. If anything fails, the failure is in a struct or enum where a non-replaceable parameter appears inside a field's generic args; investigate before continuing.

- [ ] **Step 5: Commit**

```bash
git add epserde-derive/src/lib.rs
git commit -m "$(cat <<'EOF'
Dispatch field ε-deserialization by type containment

Adds 'type_contains_any', recursively scanning a syn::Type for any
identifier in a given set. Replaces the existing single-segment check
in 'gen_eps_deser_method_call' so that field types mentioning a
replaceable parameter (in any position) are routed through
'_deser_eps_inner'. Caller updates follow in subsequent tasks.
EOF
)"
```

---

## Task 4: Compute unified `repl_params` in struct impl

**Files:**

- Modify: `epserde-derive/src/lib.rs` (the `gen_epserde_struct_impl` function)

- [ ] **Step 1: Refactor the struct impl to a two-pass shape**

Locate `gen_epserde_struct_impl` (around line 521). Replace the top of the function — the loop that collects `field_names`, `field_types`, `method_calls`, and `repl_params` — with a two-pass version that builds the unified `repl_params` first, then iterates fields a second time to generate `method_calls`.

Before:

```rust
fn gen_epserde_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];
    let mut repl_params = HashSet::new();

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;

        // We look for type parameters that are types of fields
        if let Some(field_type_id) = get_ident(field_type) {
            if ctx.type_params.contains(field_type_id) {
                repl_params.insert(field_type_id);
            }
        }

        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            &ctx.type_params,
        ));

        field_names.push(field_name);
        field_types.push(field_type);
    }
```

After:

```rust
fn gen_epserde_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    // Pass 1: compute the set of replaceable parameters, unioning the
    // naturally-detected ones (single-segment type-param fields) with the
    // user-declared 'force_repl' idents.
    let mut repl_params: HashSet<&syn::Ident> = HashSet::new();
    for field in s.fields.iter() {
        if let Some(field_type_id) = get_ident(&field.ty) {
            if ctx.type_params.contains(field_type_id) {
                repl_params.insert(field_type_id);
            }
        }
    }
    for ident in &ctx.force_repl {
        repl_params.insert(ident);
    }

    // Pass 2: gather field metadata and generate the per-field ε-deser
    // method calls using the unified 'repl_params'.
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;

        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            &repl_params,
        ));

        field_names.push(field_name);
        field_types.push(field_type);
    }
```

Leave the rest of `gen_epserde_struct_impl` unchanged. `gen_generics_for_deser_type`, `gen_generics_for_ser_type`, and `bound_ser_deser_types` already consume `&repl_params`, so they will pick up forced params automatically.

- [ ] **Step 2: Build**

Run: `cargo build --all-features`
Expected: clean build.

- [ ] **Step 3: Run tests**

Run: `cargo test -- --skip fail`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add epserde-derive/src/lib.rs
git commit -m "$(cat <<'EOF'
Unify natural and force_repl params for struct dispatch

Splits the struct impl generator into two passes: the first computes
'repl_params = natural ∪ force_repl', the second uses it to dispatch
field ε-deserialization. Existing call sites of
'gen_generics_for_deser_type', 'gen_generics_for_ser_type', and
'bound_ser_deser_types' pick up the union without further changes.
EOF
)"
```

---

## Task 5: Compute unified `repl_params` in enum impl

**Files:**

- Modify: `epserde-derive/src/lib.rs` (the `gen_epserde_enum_impl` function)

- [ ] **Step 1: Pre-compute `repl_params` before iterating variants**

Locate `gen_epserde_enum_impl` (around line 696). Find the initialization of `all_repl_params`:

```rust
    // Type parameters that are types of some fields in some variant
    let mut all_repl_params = HashSet::new();
```

Replace those two lines with a pre-pass that computes the unified set across all variants:

```rust
    // Type parameters that are types of some fields in some variant,
    // unioned with the user-declared 'force_repl' idents.
    let mut all_repl_params: HashSet<&syn::Ident> = HashSet::new();
    for variant in &e.variants {
        for field in variant.fields.iter() {
            if let Some(field_type_id) = get_ident(&field.ty) {
                if ctx.type_params.contains(field_type_id) {
                    all_repl_params.insert(field_type_id);
                }
            }
        }
    }
    for ident in &ctx.force_repl {
        all_repl_params.insert(ident);
    }
```

- [ ] **Step 2: Remove the incremental updates inside variant loops**

Inside the variant-processing loop, find the two occurrences of:

```rust
                for field in &fields.named {
                    // It's a named field
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = &field.ty;

                    // We look for type parameters that are types of fields
                    if let Some(field_type_id) = get_ident(field_type) {
                        if ctx.type_params.contains(field_type_id) {
                            all_repl_params.insert(field_type_id);
                        }
                    }

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
                    ));
```

and:

```rust
                for (field_idx, field) in fields.unnamed.iter().enumerate() {
                    let field_name = syn::Index::from(field_idx);
                    let field_type = &field.ty;

                    // We look for type parameters that are types of fields
                    if let Some(field_type_id) = get_ident(field_type) {
                        if ctx.type_params.contains(field_type_id) {
                            all_repl_params.insert(field_type_id);
                        }
                    }
```

In each, remove the inner `if let Some(field_type_id) = ... { ... }` block — the unified set is already populated. The two loop bodies become:

For the named-fields case:

```rust
                for field in &fields.named {
                    // It's a named field
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = &field.ty;

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
                    ));
                    field_names.push(quote! { #field_name });
                    field_types.push(field_type);
                }
```

For the unnamed-fields case:

```rust
                for (field_idx, field) in fields.unnamed.iter().enumerate() {
                    let field_name = syn::Index::from(field_idx);
                    let field_type = &field.ty;

                    field_indices.push(
                        syn::Ident::new(&format!("v{}", field_idx), proc_macro2::Span::call_site())
                            .to_token_stream(),
                    );

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
                    ));
                    field_types.push(field_type);
                    field_names_in_arm.push(field_name);
                }
```

- [ ] **Step 3: Build**

Run: `cargo build --all-features`
Expected: clean build.

- [ ] **Step 4: Run tests**

Run: `cargo test -- --skip fail`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add epserde-derive/src/lib.rs
git commit -m "$(cat <<'EOF'
Unify natural and force_repl params for enum dispatch

Pre-computes 'all_repl_params' across every variant and unions in the
user-declared 'force_repl' idents before the variant-generation loop.
Removes the incremental updates that previously made dispatch
order-sensitive across variants.
EOF
)"
```

---

## Task 6: Positive integration test — basic wrapper case

**Files:**

- Create: `epserde/tests/test_force_repl.rs`

- [ ] **Step 1: Write the test file**

Create `epserde/tests/test_force_repl.rs` with the following content:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// A naturally-replaceable wrapper: T appears as a direct field, so its
// `DeserType<'a>` substitutes T transitively for any T.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct A<T>(T);

// T does *not* appear as a direct field, only inside A<T>. Without
// 'force_repl(T)', T would be non-replaceable in B and the ε-copy
// deserialized form would keep T as-is.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct B<T>(A<T>);

#[test]
fn test_force_repl_wrapper() -> anyhow::Result<()> {
    let original: B<Vec<u32>> = B(A(vec![1, 2, 3, 4]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    // Full-copy round-trip.
    cursor.set_position(0);
    let full = unsafe { <B<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // ε-copy round-trip: inner Vec<u32> must come back as &[u32].
    let eps = unsafe { <B<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner_slice: &[u32] = eps.0.0;
    assert_eq!([1u32, 2, 3, 4].as_slice(), inner_slice);

    Ok(())
}
```

- [ ] **Step 2: Run the new test**

Run: `cargo test --test test_force_repl`
Expected: PASS. The crucial assertion is the `let inner_slice: &[u32] = eps.0.0;` line — it fails to compile if `B<Vec<u32>>::DeserType<'_>` is not `B<&[u32]>`.

- [ ] **Step 3: Commit**

```bash
git add epserde/tests/test_force_repl.rs
git commit -m "$(cat <<'EOF'
Add wrapper-case round-trip test for force_repl

Tests that B<T>(A<T>) with #[epserde(force_repl(T))] makes T
transitively replaceable in B, so that B<Vec<u32>>'s ε-copy deserialized
form's inner field has type &[u32] instead of Vec<u32>.
EOF
)"
```

---

## Task 7: More positive tests — mixed-position, bounded, idempotency, enum

**Files:**

- Modify: `epserde/tests/test_force_repl.rs`

- [ ] **Step 1: Add mixed-position test**

Append to `epserde/tests/test_force_repl.rs`:

```rust
// T appears both as a direct field *and* through a wrapper. Without
// 'force_repl(T)' this is rejected because the generated code's
// 'DeserType<'_>' would have inconsistent slots. With 'force_repl(T)'
// both slots are substituted uniformly.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct Mixed<T>(T, A<T>);

#[test]
fn test_force_repl_mixed_position() -> anyhow::Result<()> {
    let original: Mixed<Vec<u32>> = Mixed(vec![10, 20], A(vec![30, 40, 50]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Mixed<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Mixed<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let direct: &[u32] = eps.0;
    let through_a: &[u32] = eps.1.0;
    assert_eq!([10u32, 20].as_slice(), direct);
    assert_eq!([30u32, 40, 50].as_slice(), through_a);

    Ok(())
}
```

- [ ] **Step 2: Add bounded-parameter test**

Append:

```rust
// 'force_repl' on a parameter with trait bounds must propagate those
// bounds onto the substituted form ('DeserType<'_, T>: Clone').
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct Bounded<T: Clone>(A<T>);

#[test]
fn test_force_repl_bounded() -> anyhow::Result<()> {
    let original: Bounded<Vec<u32>> = Bounded(A(vec![7, 8, 9]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Bounded<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Bounded<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.0.0;
    // Exercise the propagated Clone bound on DeserType<'_, T>.
    let _cloned = inner;
    assert_eq!([7u32, 8, 9].as_slice(), inner);

    Ok(())
}
```

- [ ] **Step 3: Add idempotency test**

Append:

```rust
// 'force_repl(T)' on a parameter that is already naturally replaceable
// is a no-op: the derived code must behave identically to A<T> above.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct Redundant<T>(T);

#[test]
fn test_force_repl_redundant() -> anyhow::Result<()> {
    let original: Redundant<Vec<u32>> = Redundant(vec![100, 200]);
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Redundant<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Redundant<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.0;
    assert_eq!([100u32, 200].as_slice(), inner);

    Ok(())
}
```

- [ ] **Step 4: Add enum test**

Append:

```rust
// Forced replaceability works on enum parameters across all variant
// shapes (unit, unnamed, named).
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
enum E<T> {
    Empty,
    Single(A<T>),
    Named { value: A<T> },
}

#[test]
fn test_force_repl_enum() -> anyhow::Result<()> {
    let original: E<Vec<u32>> = E::Single(A(vec![5, 6, 7]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <E<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <E<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    match eps {
        E::Single(a) => {
            let inner: &[u32] = a.0;
            assert_eq!([5u32, 6, 7].as_slice(), inner);
        }
        _ => panic!("expected E::Single variant"),
    }

    Ok(())
}
```

- [ ] **Step 5: Run all the new tests**

Run: `cargo test --test test_force_repl`
Expected: all four new tests PASS.

- [ ] **Step 6: Run the full test suite to catch regressions**

Run: `cargo test -- --skip fail`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add epserde/tests/test_force_repl.rs
git commit -m "$(cat <<'EOF'
Add mixed-position, bounded, redundant, and enum force_repl tests

Covers the four remaining positive paths from the spec: a parameter
appearing both directly and through a wrapper, a bounded forced
parameter, a redundant force_repl on a naturally-replaceable
parameter, and forced replaceability on an enum across all variant
shapes.
EOF
)"
```

---

## Task 8: Compile-fail tests for `force_repl`

**Files:**

- Create: `epserde/tests/fail/force_repl_unknown_param.rs`
- Create: `epserde/tests/fail/force_repl_unknown_param.stderr`
- Create: `epserde/tests/fail/force_repl_on_zero_copy.rs`
- Create: `epserde/tests/fail/force_repl_on_zero_copy.stderr`

- [ ] **Step 1: Write the unknown-param fail case**

Create `epserde/tests/fail/force_repl_unknown_param.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde)]
#[epserde(force_repl(X))]
struct G<T>(T);

fn main() {
    let _ = G::<u32>(0);
}
```

- [ ] **Step 2: Write the zero-copy fail case**

Create `epserde/tests/fail/force_repl_on_zero_copy.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[epserde(force_repl(T))]
#[repr(C)]
struct H<T: Copy>(T);

fn main() {
    let _ = H::<u32>(0);
}
```

- [ ] **Step 3: Generate the `.stderr` files**

`trybuild` records the expected compiler output in `.stderr` files. The easiest way is to run with `TRYBUILD=overwrite`, which materializes them, then commit the recorded output:

Run: `TRYBUILD=overwrite cargo test fail`
Expected: PASS. The two new fail tests get fresh `.stderr` files in `epserde/tests/fail/`.

- [ ] **Step 4: Inspect the generated `.stderr` files**

Open `epserde/tests/fail/force_repl_unknown_param.stderr` and `epserde/tests/fail/force_repl_on_zero_copy.stderr` and confirm they contain, respectively, the messages emitted by the validation in Task 2:

- For `force_repl_unknown_param.stderr`: must mention `` `X` is not a generic type parameter of this item ``.
- For `force_repl_on_zero_copy.stderr`: must mention `` `force_repl` cannot be used with zero-copy types ``.

If either file is missing the expected message, the error in Task 2 was either spelled differently or never reached. Fix and re-run with `TRYBUILD=overwrite`.

- [ ] **Step 5: Run `fail` tests in the normal mode**

Run: `cargo test fail`
Expected: PASS. The `.stderr` files now contain the recorded output, so the run no longer needs `TRYBUILD=overwrite`.

- [ ] **Step 6: Commit**

```bash
git add epserde/tests/fail/force_repl_unknown_param.rs \
        epserde/tests/fail/force_repl_unknown_param.stderr \
        epserde/tests/fail/force_repl_on_zero_copy.rs \
        epserde/tests/fail/force_repl_on_zero_copy.stderr
git commit -m "$(cat <<'EOF'
Add trybuild compile-fail tests for force_repl misuse

Covers two validation paths from the design spec: 'force_repl(X)'
where X is not a declared type parameter, and 'force_repl' on a
zero-copy type. Recorded .stderr output captures the diagnostic
messages emitted by the validation in Task 2.
EOF
)"
```

---

## Task 9: Documentation

**Files:**

- Modify: `epserde-derive/src/lib.rs` (doc comment on `#[derive(Epserde)]`)
- Modify: `epserde/src/lib.rs` (crate-level prose near `PhantomDeserData`)
- Modify: `CLAUDE.md` (Key Invariants section)

- [ ] **Step 1: Locate the existing derive macro doc comment**

In `epserde-derive/src/lib.rs`, find the doc comment block immediately preceding `#[proc_macro_derive(Epserde, ...)]` (around line 1029). It documents the existing attributes — `zero_copy`, `deep_copy`, `bound(...)`. Skim it once to match the existing style and indentation.

- [ ] **Step 2: Append `force_repl` documentation**

Insert a new section into that doc comment, after the existing description of `bound(...)`. The exact text to add:

````rust
/// # `force_repl` attribute
///
/// `#[epserde(force_repl(T, U, ...))]` forces the named type parameters to be
/// replaceable in `Self::DeserType<'_>` and `Self::SerType`, even if they do
/// not appear as a direct field type.
///
/// The attribute lifts two related restrictions:
/// - a type parameter that appears only inside a wrapper (e.g. `Vec<T>`
///   in a field of type `A<Vec<T>>`) can still be substituted in the
///   ε-copy deserialized form;
/// - a type parameter can appear both as a direct field type and as a
///   parameter of another field's type.
///
/// In both cases, every field type that mentions a forced parameter must
/// substitute it transitively in its own `DeserType<'_>` and `SerType`.
/// This is a contract on the user; standard library wrappers (`Vec<T>`,
/// `Box<T>`, `Option<T>`, tuples, arrays) and `Epserde`-derived types
/// satisfy it for their naturally-replaceable parameters. A violated
/// contract produces a compile error in the generated `_deser_eps_inner`
/// body — no silent miscompilation.
///
/// `force_repl` is rejected on zero-copy types and on idents that do
/// not name a generic type parameter of the annotated item. Listing a
/// naturally-replaceable parameter is allowed (no-op).
///
/// Example:
///
/// ```ignore
/// #[derive(Epserde)]
/// struct A<T>(T);
///
/// #[derive(Epserde)]
/// #[epserde(force_repl(T))]
/// struct B<T>(A<T>);
/// ```
````

- [ ] **Step 3: Add crate-level prose in `epserde/src/lib.rs`**

In `epserde/src/lib.rs`, find the existing crate-level documentation discussing `PhantomDeserData` (search for the heading or paragraph about that helper). Append a new top-level section discussing `force_repl`. The exact text:

```rust
//! ## Forcing transitive replaceability with `force_repl`
//!
//! By default, a generic type parameter `T` of an `Epserde`-derived
//! struct or enum is substituted with `T::DeserType<'_>` in
//! `Self::DeserType<'_>` only if `T` appears as the exact type of one of
//! the item's fields. If `T` only appears through a wrapper — for
//! example, in `struct B<T>(A<Vec<T>>)` — then `B<…>::DeserType<'_>`
//! keeps `T` unchanged, and the ε-copy deserialized form does not benefit
//! from `T`'s own ε-copy form.
//!
//! The struct/enum-level attribute `#[epserde(force_repl(T, U, …))]`
//! lets you opt every named parameter into transitive replaceability.
//! The asserted contract is that every field type containing the
//! parameter substitutes it transitively in its own `DeserType<'_>` and
//! `SerType`; standard library wrappers and `Epserde`-derived types
//! satisfy this automatically. See the documentation of `#[derive(Epserde)]`
//! for the precise rules and failure modes.
```

- [ ] **Step 4: Update `CLAUDE.md`**

Open `CLAUDE.md`. In the "Key Invariants" section, find the line:

```
- A replaceable type parameter must not appear both as a field type and as a parameter of another field type
```

Replace it with:

```
- A replaceable type parameter must not appear both as a field type and as a parameter of another field type, unless the type is annotated with `#[epserde(force_repl(T))]`
```

- [ ] **Step 5: Verify docs build cleanly**

Run: `cargo doc --no-deps --all-features`
Expected: PASS, no warnings about broken intra-doc links or unclosed code fences.

- [ ] **Step 6: Run the full check suite**

Run these in parallel (or sequentially if you prefer): `cargo fmt -- --check`, `cargo clippy --all-features`, `cargo test -- --skip fail`, `cargo test fail`.
Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add epserde-derive/src/lib.rs epserde/src/lib.rs CLAUDE.md
git commit -m "$(cat <<'EOF'
Document the force_repl attribute

Adds documentation on '#[derive(Epserde)]' describing the attribute's
semantics, the user contract for field types, the failure mode, and an
example. Adds matching crate-level prose in 'epserde/src/lib.rs' and
updates the corresponding 'Key Invariants' entry in 'CLAUDE.md' to note
the new opt-out.
EOF
)"
```

---

## Final verification

- [ ] **Run the full CI matrix locally**

Run: `cargo build --all-features && cargo fmt -- --check && cargo clippy --all-features && cargo test -- --skip fail && cargo test fail`

Expected: all four steps PASS.

- [ ] **Manual sanity check of the motivating example**

Confirm by inspection (no extra test required) that the file `epserde/tests/test_force_repl.rs` exercises a `struct B<T>(A<T>)` with `#[epserde(force_repl(T))]` and asserts `&[u32]` (not `Vec<u32>`) is recovered after ε-copy deserialization.

If both checks pass, the implementation matches the spec.
