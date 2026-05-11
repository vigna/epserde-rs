# Field-level `#[epserde(force_repl)]` and `#[epserde(force_irrepl)]` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the struct-level `#[epserde(force_repl(T, U, …))]` attribute with two symmetric **field-level** attributes — `#[epserde(force_repl)]` (lifts a wrapper field's parameter occurrences into the replaceable set) and `#[epserde(force_irrepl)]` (flips a direct-generic field's occurrence into the irreplaceable set and full-dispatches it) — whose semantics are expressed directly via the README's "replaceable / irreplaceable" parameter classification, fixing the architectural mismatch the struct-level design introduces.

**Architecture:** All work is localized to `epserde-derive/src/lib.rs` for the derive logic, plus test/doc files for migration. The core change is replacing the current `type_contains_any`-driven dispatch widening with (a) a per-field marker that lifts the irreplaceability contribution of its field's parameter occurrences, and (b) a small classifier that walks every field's type once, computes the replaceable/irreplaceable sets, and either emits a clean `syn::Error` for conflicts or returns the replaceable set the rest of the macro consumes. Substitution and bounds then operate on the classifier's output exactly as today's natural-repl rule does.

**Tech Stack:** Rust 2024 (MSRV 1.85), `syn` 2 / `quote` 1 (proc-macro), `trybuild` for compile-fail fixtures.

**Spec:** `docs/superpowers/specs/2026-05-11-field-level-force-repl-design.md`

**Comment-style note:** `//` line comments use plain text (no backticks). `///` doc comments and markdown freely use backticks. Apply throughout.

**Branch note:** Work happens on `review-force-repl` (currently checked out). The plan does NOT touch the historical struct-level spec/plan docs (`docs/superpowers/specs/2026-05-11-epserde-force-repl-attribute-design.md`, `docs/superpowers/plans/2026-05-11-epserde-force-repl-attribute.md`) — they stay as a record of the abandoned design.

---

## Task 1: Parse field-level `#[epserde(force_repl)]` and `#[epserde(force_irrepl)]`

**Files:**
- Modify: `epserde-derive/src/lib.rs` (introduce small helpers that read the per-field attributes)

- [ ] **Step 1: Add the helpers**

Insert immediately after the existing `get_ident` function in `epserde-derive/src/lib.rs`:

```rust
/// Field-level marker state for the new symmetric attributes.
///
/// `Default` means "neither marker present" — the field follows the
/// default classification and dispatch rules.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum FieldMarker {
    #[default]
    None,
    /// `#[epserde(force_repl)]` — contributes wrapper occurrences to
    /// replaceability; dispatch flips to `_deser_eps_inner`.
    ForceRepl,
    /// `#[epserde(force_irrepl)]` — contributes a direct (single-segment
    /// generic) occurrence to irreplaceability; dispatch flips to
    /// `_deser_full_inner`.
    ForceIrrepl,
}

/// Reads `#[epserde(force_repl)]` / `#[epserde(force_irrepl)]` off a field.
/// The two markers are mutually exclusive on the same field; validation
/// for that (and for argument shape) lives in Task 7.
fn field_marker(field: &syn::Field) -> FieldMarker {
    let mut result = FieldMarker::None;
    for attr in &field.attrs {
        if !attr.meta.path().is_ident("epserde") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("force_repl") {
                result = FieldMarker::ForceRepl;
            } else if meta.path.is_ident("force_irrepl") {
                result = FieldMarker::ForceIrrepl;
            }
            Ok(())
        });
    }
    result
}
```

- [ ] **Step 2: Build**

Run: `cargo build --all-features` from `/Users/vigna/git/epserde-rs`.
Expected: clean build. The helper is unused so far — a `dead_code` warning is acceptable and will go away when Task 2 consumes it.

- [ ] **Step 3: Existing tests still pass**

Run: `cargo test -- --skip fail`
Expected: PASS (helper is unused, no behavioural change).

- [ ] **Step 4: Commit**

Stage and surface diff for review. **Do not commit without explicit user approval.** When approved:

```bash
git -C /Users/vigna/git/epserde-rs add epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Parse field-level force_repl and force_irrepl markers

Adds a FieldMarker enum and a field_marker helper that reads either
#[epserde(force_repl)] or #[epserde(force_irrepl)] off a field. Used
by the upcoming classifier and dispatch wiring; not yet acted upon.
The old struct-level force_repl(...) parsing remains in place for now.
EOF
)"
```

---

## Task 2: Build the classifier

**Files:**
- Modify: `epserde-derive/src/lib.rs` (add the classifier function next to `type_contains_any`)

- [ ] **Step 1: Add the classifier**

Insert the function immediately after `type_contains_any` in `epserde-derive/src/lib.rs`:

```rust
/// Per-field classification record produced by `classify_repl_params`.
///
/// One entry per generic type parameter of the struct/enum being derived.
/// The same parameter may show up replaceable (from one field) and
/// irreplaceable (from another) — the caller detects this as a conflict.
struct ParamClassification<'a> {
    /// The parameter being classified.
    ident: &'a syn::Ident,
    /// Set of field names where the parameter appears in a position that
    /// contributes to replaceability. Used for diagnostic messages.
    replaceable_in: Vec<proc_macro2::TokenStream>,
    /// Set of field names where the parameter appears in a position that
    /// contributes to irreplaceability. Used for diagnostic messages.
    irreplaceable_in: Vec<proc_macro2::TokenStream>,
}

/// Walks every field's type and classifies each generic parameter's
/// occurrences as replaceable, irreplaceable, or neither (inside
/// PhantomData<…> or absent). Returns one record per generic parameter,
/// in declaration order.
///
/// The walker treats PhantomData<…> as a barrier: occurrences inside it
/// (at any depth) contribute to neither classification.
///
/// Marker handling at the direct (single-segment) field level:
/// - `None` (default) → contributes to replaceable.
/// - `ForceIrrepl` → contributes to irreplaceable (the marker exists to
///   override the natural-repl default).
/// - `ForceRepl` on a direct field → contributes to replaceable (same as
///   default; the marker is a silent no-op there).
///
/// Marker handling at the type-argument level:
/// - `None` → contributes to irreplaceable.
/// - `ForceRepl` → contributes to replaceable.
/// - `ForceIrrepl` is rejected at validation time on non-direct fields
///   (Task 7), so the walker does not need to handle that combination.
fn classify_repl_params<'a>(
    type_params: &[&'a syn::Ident],
    fields: &[(proc_macro2::TokenStream, &syn::Type, FieldMarker)],
) -> Vec<ParamClassification<'a>> {
    let mut out: Vec<ParamClassification<'a>> = type_params
        .iter()
        .map(|p| ParamClassification {
            ident: p,
            replaceable_in: Vec::new(),
            irreplaceable_in: Vec::new(),
        })
        .collect();

    for (field_name, field_type, marker) in fields {
        // A field whose type is exactly a single-segment generic adds the
        // parameter to one of the buckets per the marker.
        if let Some(ident) = get_ident(field_type) {
            if let Some(rec) = out.iter_mut().find(|r| r.ident == ident) {
                match marker {
                    FieldMarker::ForceIrrepl => rec.irreplaceable_in.push(field_name.clone()),
                    _ => rec.replaceable_in.push(field_name.clone()),
                }
                continue;
            }
        }
        // Otherwise walk the field's type, classifying each generic-ident
        // occurrence. ForceRepl-marked fields contribute to replaceable;
        // unmarked fields contribute to irreplaceable. Inside PhantomData<…>
        // nothing is recorded.
        let field_is_marked = matches!(marker, FieldMarker::ForceRepl);
        collect_occurrences(
            field_type, field_is_marked, field_name, type_params, &mut out, false,
        );
    }

    out
}

/// Recursive helper for `classify_repl_params`. `inside_phantom` becomes
/// true when the walk descends into the type arguments of a PhantomData
/// path segment; while it is true, no occurrences are recorded.
fn collect_occurrences<'a>(
    ty: &syn::Type,
    field_marked: bool,
    field_name: &proc_macro2::TokenStream,
    type_params: &[&'a syn::Ident],
    out: &mut [ParamClassification<'a>],
    inside_phantom: bool,
) {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            for segment in &path.segments {
                let segment_is_phantom = segment.ident == "PhantomData";
                // Record this segment's ident if it is a struct generic and
                // appears as a path on its own — but only at the path's first
                // segment with no qualifier, to avoid B::Word style false hits.
                // We do not record idents here because the top-level call
                // already handles the single-segment-direct case before
                // recursing.
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let descend_inside_phantom = inside_phantom || segment_is_phantom;
                    for arg in &ab.args {
                        match arg {
                            syn::GenericArgument::Type(t) => {
                                // If t is a bare single-segment generic ident,
                                // record this position; otherwise recurse into t.
                                let bare = get_ident(t).and_then(|id| {
                                    type_params.iter().find(|p| **p == id).copied()
                                });
                                if let Some(p_ident) = bare {
                                    if !descend_inside_phantom {
                                        let rec = out
                                            .iter_mut()
                                            .find(|r| r.ident == p_ident)
                                            .expect("ident is in type_params");
                                        if field_marked {
                                            rec.replaceable_in.push(field_name.clone());
                                        } else {
                                            rec.irreplaceable_in.push(field_name.clone());
                                        }
                                    }
                                } else {
                                    collect_occurrences(
                                        t,
                                        field_marked,
                                        field_name,
                                        type_params,
                                        out,
                                        descend_inside_phantom,
                                    );
                                }
                            }
                            syn::GenericArgument::AssocType(a) => {
                                collect_occurrences(
                                    &a.ty,
                                    field_marked,
                                    field_name,
                                    type_params,
                                    out,
                                    descend_inside_phantom,
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        syn::Type::Tuple(t) => {
            for e in &t.elems {
                collect_occurrences(
                    e, field_marked, field_name, type_params, out, inside_phantom,
                );
            }
        }
        syn::Type::Array(a) => collect_occurrences(
            &a.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        syn::Type::Slice(s) => collect_occurrences(
            &s.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        syn::Type::Paren(p) => collect_occurrences(
            &p.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        syn::Type::Group(g) => collect_occurrences(
            &g.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        _ => {}
    }
}
```

- [ ] **Step 2: Build**

Run: `cargo build --all-features`
Expected: clean build (helpers unused — `dead_code` warnings expected, resolved in Task 3).

- [ ] **Step 3: Existing tests still pass**

Run: `cargo test -- --skip fail`
Expected: PASS.

- [ ] **Step 4: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Add field-level force_repl classifier

Introduces classify_repl_params and the recursive collect_occurrences
helper. The classifier walks every field's type, records each generic
parameter's occurrences as either replaceable or irreplaceable, and
treats PhantomData<…> as a barrier whose interior is ignored. Output
is consumed in the next task; the function is unused so far.
EOF
)"
```

---

## Task 3: Wire classifier into the struct impl generator

**Files:**
- Modify: `epserde-derive/src/lib.rs` (the `gen_epserde_struct_impl` function)

- [ ] **Step 1: Replace the natural-repl scan with a classifier call**

Locate `gen_epserde_struct_impl`. The current top of the function builds `repl_params` from a per-field scan (single-segment-param detection plus union with `ctx.force_repl`). Replace that whole prefix with:

```rust
fn gen_epserde_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    // Per-field metadata: (display name, type, marker). The marker is
    // consumed by the classifier and by per-field dispatch.
    let fields_info: Vec<(proc_macro2::TokenStream, &syn::Type, FieldMarker)> = s
        .fields
        .iter()
        .enumerate()
        .map(|(idx, field)| (get_field_name(field, idx), &field.ty, field_marker(field)))
        .collect();

    // Classify each generic parameter's occurrences.
    let classifications =
        classify_repl_params(&ctx.type_const_params, &fields_info);

    // Emit conflict diagnostic in Task 6; for now, just compute repl_params.
    let repl_params: HashSet<&syn::Ident> = classifications
        .iter()
        .filter(|c| !c.replaceable_in.is_empty())
        .map(|c| c.ident)
        .collect();

    // ... (rest of the function: per-field method calls, where clauses, impls)
```

Then in the per-field method-call generation loop (further down in the function), use `fields_info`'s `is_marked` flag:

```rust
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];
    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;
        let marker = field_marker(field);
        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            &repl_params,
            marker,
        ));
        field_names.push(field_name);
        field_types.push(field_type);
    }
```

Note `gen_eps_deser_method_call` gains a fourth `marker: FieldMarker` argument — its signature changes in Task 4. For now, you can pass `marker` even though the function doesn't yet read it; Task 4 will start consuming it.

- [ ] **Step 2: Add `marker` parameter to `gen_eps_deser_method_call`**

Update the signature:

```rust
fn gen_eps_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    repl_params: &HashSet<&syn::Ident>,
    _marker: FieldMarker,  // wired up in Task 4
) -> proc_macro2::TokenStream {
    // ... existing body unchanged for now
}
```

Underscore-prefix marks it as known-unused (no warning).

- [ ] **Step 3: Build**

Run: `cargo build --all-features`
Expected: clean build.

- [ ] **Step 4: Run existing tests**

Run: `cargo test -- --skip fail`
Expected: PASS. The classifier reproduces today's natural-repl rule for unmarked fields; no struct-level `force_repl(...)` test currently exists that distinguishes the behaviour at this stage (Task 5 handles tests using the struct-level attribute).

If a test fails, the classifier disagrees with the existing natural-repl detection. Investigate before continuing.

- [ ] **Step 5: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Use classifier for repl_params in struct derive

Replaces the inline natural-repl scan in gen_epserde_struct_impl with
a single call to classify_repl_params. The classifier reproduces
today's natural-repl rule for unmarked fields, so existing test
behaviour is preserved. The classifier's output (replaceable set) is
passed unchanged to gen_generics_for_deser_type, gen_generics_for_ser_type,
and bound_ser_deser_types. Field-level force_repl marks are now
collected per field but not yet acted upon at the dispatch site.
EOF
)"
```

---

## Task 4: Rewire dispatch in `gen_eps_deser_method_call`

**Files:**
- Modify: `epserde-derive/src/lib.rs` (the `gen_eps_deser_method_call` function)

- [ ] **Step 1: Update dispatch to use marker + single-segment rule, drop type-containment widening**

Replace the body of `gen_eps_deser_method_call`:

```rust
fn gen_eps_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    repl_params: &HashSet<&syn::Ident>,
    marker: FieldMarker,
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
            // PhantomData<...> is handled natively: emit a literal
            // PhantomData whose generic parameter is inferred from the
            // surrounding Self::DeserType<'a> struct literal.
            if segment.ident == "PhantomData" {
                return syn::parse_quote!(#field_name: ::core::marker::PhantomData);
            }
        }
    }

    // Dispatch:
    // - force_irrepl marker → full-deser, regardless of field shape.
    // - force_repl marker → eps-deser.
    // - No marker, single-segment generic → eps-deser (natural rule).
    // - Otherwise → full-deser.
    let is_natural_repl = get_ident(field_type)
        .map(|id| repl_params.contains(id))
        .unwrap_or(false);
    let use_eps = match marker {
        FieldMarker::ForceIrrepl => false,
        FieldMarker::ForceRepl => true,
        FieldMarker::None => is_natural_repl,
    };

    if use_eps {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_eps_inner(backend)? })
    } else {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_full_inner(backend)? })
    }
}
```

Note the dispatch no longer calls `type_contains_any` — that helper, plus the broader walking for dispatch decisions, is gone. The repl_params set is consulted only to check single-segment-param fields (today's natural-repl check, expressed via `get_ident`).

- [ ] **Step 2: Build**

Run: `cargo build --all-features`
Expected: clean build. The `type_contains_any` helper is now unused; allow the dead-code warning (it gets removed in Task 5).

- [ ] **Step 3: Run existing tests**

Run: `cargo test -- --skip fail`

Expected: tests that depended on the `type_contains_any` widening will now break — specifically the existing struct-level force_repl tests in `epserde/tests/test_force_repl.rs`. Tests using only naturally-replaceable parameters (`test_phantom.rs`, `test_generics.rs`, etc.) should still pass.

Failures in `test_force_repl.rs` are expected at this stage. They are fixed in Task 5 (test rewrite) and Task 6 (remove struct-level machinery).

- [ ] **Step 4: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Rewire field dispatch: marker OR single-segment, no type walking

gen_eps_deser_method_call now picks _deser_eps_inner iff the field
carries #[epserde(force_repl)] or its type is a single-segment struct
generic. The earlier type_contains_any widening is gone; dispatch is
again a local per-field decision. The current type_contains_any helper
is left in place for one more commit and will be removed in Task 5
together with the rest of the struct-level force_repl machinery.

test_force_repl.rs is expected to break at this commit (struct-level
force_repl(T) no longer drives dispatch); it is rewritten against the
field-level surface in Task 5.
EOF
)"
```

---

## Task 5: Rewrite `test_force_repl.rs` and the existing fail fixtures

**Files:**
- Modify: `epserde/tests/test_force_repl.rs`
- Delete: `epserde/tests/fail/force_repl_unknown_param.rs`, `epserde/tests/fail/force_repl_unknown_param.stderr`
- Modify: `epserde/tests/fail/force_repl_on_zero_copy.rs`, `epserde/tests/fail/force_repl_on_zero_copy.stderr`

- [ ] **Step 1: Rewrite `test_force_repl.rs`**

Replace the whole file with the field-level surface:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct A<T>(T);

// T does not appear as a direct field; only inside A<T>. Marking the
// field lifts A<T>'s parameter occurrence into the replaceable set
// while preserving the README's no-overlap invariant.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct B<T> {
    #[epserde(force_repl)]
    inner: A<T>,
}

#[test]
fn test_force_repl_wrapper() -> anyhow::Result<()> {
    let original: B<Vec<u32>> = B { inner: A(vec![1, 2, 3, 4]) };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <B<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <B<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner_slice: &[u32] = eps.inner.0;
    assert_eq!([1u32, 2, 3, 4].as_slice(), inner_slice);

    Ok(())
}

// T appears both as a direct field AND inside A<T>, but the wrapping
// field is marked, so T's occurrence inside A<T> contributes to
// replaceability rather than irreplaceability. No conflict; both
// slots are substituted uniformly in Self::DeserType<'a>.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Mixed<T> {
    direct: T,
    #[epserde(force_repl)]
    wrapped: A<T>,
}

#[test]
fn test_force_repl_mixed_position() -> anyhow::Result<()> {
    let original: Mixed<Vec<u32>> = Mixed {
        direct: vec![10, 20],
        wrapped: A(vec![30, 40, 50]),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Mixed<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Mixed<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let direct: &[u32] = eps.direct;
    let through_wrapper: &[u32] = eps.wrapped.0;
    assert_eq!([10u32, 20].as_slice(), direct);
    assert_eq!([30u32, 40, 50].as_slice(), through_wrapper);

    Ok(())
}

// Marked-field substitution propagates bounds onto the substituted
// form, matching today's natural-repl behaviour for `T: Clone` etc.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Bounded<T: Clone> {
    #[epserde(force_repl)]
    inner: A<T>,
}

#[test]
fn test_force_repl_bounded() -> anyhow::Result<()> {
    let original: Bounded<Vec<u32>> = Bounded { inner: A(vec![7, 8, 9]) };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Bounded<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Bounded<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.inner.0;
    let _cloned = inner;
    assert_eq!([7u32, 8, 9].as_slice(), inner);

    Ok(())
}

// Marker on a single-segment-param field is a no-op (the field would
// already be ε-dispatched by the natural rule). Compiles and round-
// trips identically.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Redundant<T> {
    #[epserde(force_repl)]
    inner: T,
}

#[test]
fn test_force_repl_redundant_on_natural() -> anyhow::Result<()> {
    let original: Redundant<Vec<u32>> = Redundant { inner: vec![100, 200] };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Redundant<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Redundant<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.inner;
    assert_eq!([100u32, 200].as_slice(), inner);

    Ok(())
}

// Marker on a parameterless field is a silent no-op: the field is
// ε-dispatched (returns its type unchanged), no parameter contribution.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct ParameterlessMarker {
    #[epserde(force_repl)]
    inner: u32,
}

#[test]
fn test_force_repl_parameterless_field() -> anyhow::Result<()> {
    let original = ParameterlessMarker { inner: 42 };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { ParameterlessMarker::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { ParameterlessMarker::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(42, eps.inner);

    Ok(())
}

// Force-repl on an enum variant field. The marker lives on the field,
// not on the variant.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
enum E<T> {
    Empty,
    Wrapped(#[epserde(force_repl)] A<T>),
    Named {
        #[epserde(force_repl)]
        value: A<T>,
    },
}

#[test]
fn test_force_repl_enum() -> anyhow::Result<()> {
    let original: E<Vec<u32>> = E::Wrapped(A(vec![5, 6, 7]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <E<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <E<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    match eps {
        E::Wrapped(a) => {
            let inner: &[u32] = a.0;
            assert_eq!([5u32, 6, 7].as_slice(), inner);
        }
        _ => panic!("expected E::Wrapped variant"),
    }

    Ok(())
}

// force_irrepl flips a direct generic field from replaceable to
// irreplaceable AND from eps-dispatch to full-dispatch. This resolves
// the historical "T both as direct field and as a type argument" case
// by pinning T as irreplaceable from both sides.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct ForceIrreplDirect<T> {
    #[epserde(force_irrepl)]
    direct: T,
    wrapped: A<T>,
}

#[test]
fn test_force_irrepl_direct() -> anyhow::Result<()> {
    let original: ForceIrreplDirect<u32> = ForceIrreplDirect {
        direct: 7,
        wrapped: A(11),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <ForceIrreplDirect<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // T is irreplaceable from both fields. Self::DeserType<'a> does NOT
    // substitute T, so the ε-form's `direct` field is plain T, not
    // T::DeserType<'a>. Type-annotated binding pins this.
    let eps = unsafe { <ForceIrreplDirect<u32>>::deserialize_eps(cursor.as_bytes())? };
    let _direct_check: u32 = eps.direct;
    assert_eq!(7, eps.direct);

    Ok(())
}
```

- [ ] **Step 2: Run the test crate**

Run: `cargo test --test test_force_repl`

Expected: all six tests pass. If `test_force_repl_enum` fails because the enum impl generator hasn't been updated, that's surfaced here — fix in Task 6.

- [ ] **Step 3: Delete the unknown-param fail fixture**

Run:

```bash
rm /Users/vigna/git/epserde-rs/epserde/tests/fail/force_repl_unknown_param.rs
rm /Users/vigna/git/epserde-rs/epserde/tests/fail/force_repl_unknown_param.stderr
```

This fixture had no analogue under field-level: there is no parameter name to be unknown.

- [ ] **Step 4: Rewrite the zero-copy fail fixture for field-level**

Open `epserde/tests/fail/force_repl_on_zero_copy.rs`. Replace its contents with:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[repr(C)]
struct H<T: Copy> {
    #[epserde(force_repl)]
    inner: T,
}

fn main() {
    let _ = H::<u32> { inner: 0 };
}
```

Delete the existing `.stderr` so trybuild regenerates it:

```bash
rm /Users/vigna/git/epserde-rs/epserde/tests/fail/force_repl_on_zero_copy.stderr
```

- [ ] **Step 5: Regenerate the zero-copy stderr**

Run: `TRYBUILD=overwrite cargo test fail`

If the wrap-up complains about other tests failing to compile (counter_*, etc.), that's OK — trybuild still materializes new `.stderr` files into `epserde/wip/`. Move the new file into place:

```bash
mv /Users/vigna/git/epserde-rs/epserde/wip/force_repl_on_zero_copy.stderr \
   /Users/vigna/git/epserde-rs/epserde/tests/fail/force_repl_on_zero_copy.stderr
rmdir /Users/vigna/git/epserde-rs/epserde/wip 2>/dev/null || true
```

Inspect the file: it should contain the rejection message for `#[epserde(force_repl)]` on a zero-copy type's field. The exact wording is finalised in Task 6; for now whatever message the derive emits gets captured.

- [ ] **Step 6: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add \
  epserde/tests/test_force_repl.rs \
  epserde/tests/fail/force_repl_on_zero_copy.rs \
  epserde/tests/fail/force_repl_on_zero_copy.stderr
git -C /Users/vigna/git/epserde-rs rm \
  epserde/tests/fail/force_repl_unknown_param.rs \
  epserde/tests/fail/force_repl_unknown_param.stderr
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Rewrite force_repl tests against field-level surface

test_force_repl.rs now exercises six field-level cases: wrapper,
mixed direct + wrapped, bounded parameter, redundant marker on a
natural-repl field, parameterless marked field, enum variant fields.

The force_repl_unknown_param fail fixture is deleted (no parameter
name to be unknown under the field-level surface). The
force_repl_on_zero_copy fixture is rewritten to place the marker on
a field of a zero-copy struct; the .stderr captures the rejection
emitted by the derive's per-field validation.
EOF
)"
```

---

## Task 6: Remove struct-level `force_repl` machinery; add per-field validation

**Files:**
- Modify: `epserde-derive/src/lib.rs` (remove parsing/validation/where-clause injection for struct-level; add per-field validation)

- [ ] **Step 1: Remove `force_repl` from `EpserdeAttrs`**

In `epserde-derive/src/lib.rs`, find the `EpserdeAttrs` struct. Delete the `force_repl: Vec<syn::Ident>` field.

In `parse_epserde_attrs`, delete:
- The `let mut force_repl: Vec<syn::Ident> = Vec::new();` local.
- The `} else if meta.path.is_ident("force_repl") { … }` arm.
- The error message listing `force_repl` (change `"expected \`zero_copy\`, \`deep_copy\`, \`bound\`, or \`force_repl\`"` back to `"expected \`zero_copy\`, \`deep_copy\`, or \`bound\`"`).
- The `force_repl,` field in the final `Ok(EpserdeAttrs { … })` block.

- [ ] **Step 2: Remove `force_repl` from `EpserdeContext`**

Find `EpserdeContext`. Delete the `force_repl: Vec<syn::Ident>` field.

In `epserde_derive` (the proc-macro entry point), delete:
- The validation loop that checks each `attrs.force_repl` ident against `type_params`.
- The validation that rejects `force_repl` on zero-copy types.
- The `force_repl: attrs.force_repl,` line in the `EpserdeContext { … }` literal.

- [ ] **Step 3: Remove the per-param bound injection**

In `gen_epserde_struct_impl`, delete the block that runs:

```rust
if !ctx.is_zero_copy {
    for ident in &ctx.force_repl {
        ser_where_clause.predicates.push(syn::parse_quote!(
            #ident: ::epserde::ser::SerInner
        ));
        deser_where_clause.predicates.push(syn::parse_quote!(
            #ident: ::epserde::deser::DeserInner
        ));
    }
}
```

Do the same in `gen_epserde_enum_impl`.

These bounds were a workaround for the struct-level design. With the classifier, parameters that need substitution come from natural-repl or from marked-field occurrences; the existing `gen_ser_deser_where_clauses` already adds `field_type: SerInner` / `field_type: DeserInner` per field, which (combined with the wrappers' own impl bounds) gives Rust everything it needs.

- [ ] **Step 4: Remove the bound-skip in `gen_ser_deser_where_clauses`**

Locate `gen_ser_deser_where_clauses`. It currently accepts an extra `enforce_repl: &HashSet<&syn::Ident>` argument (renamed from earlier work) and skips emitting `field_type: SerInner`/`DeserInner` for fields whose type contains a forced-repl ident. Revert to the simpler signature without that argument:

```rust
fn gen_ser_deser_where_clauses(
    field_types: &[&syn::Type],
    is_zero_copy: bool,
) -> (WhereClause, WhereClause) {
    let mut ser_where_clause = empty_where_clause();
    let mut deser_where_clause = empty_where_clause();
    for field_type in field_types {
        add_ser_deser_trait_bounds(
            field_type,
            is_zero_copy,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );
    }
    (ser_where_clause, deser_where_clause)
}
```

Update the two call sites (`gen_epserde_struct_impl`, `gen_epserde_enum_impl`) to drop the third argument.

The Rust #152409 shadow that this skip was working around does not arise in the field-level design: marked fields are dispatched via the literal-or-eps-deser path whose return type Rust projects through the impl's `DeserType` definition, not through a where-bound on the field type. The skip-and-substitute workaround is no longer needed.

- [ ] **Step 5: Remove `type_contains_any`**

Delete the entire `type_contains_any` function added in the earlier feature. It is now unused (the dispatch site stopped calling it in Task 4, and the bound-skip was just removed).

- [ ] **Step 6: Add per-field marker validation**

In `epserde_derive` (the proc-macro entry point), immediately after `emit_deprecation_warnings`, add a validation pass that runs over every field in the input and rejects misuse:

```rust
    // Validate the per-field force_repl / force_irrepl markers.
    let validate_field = |field: &syn::Field| -> Result<(), syn::Error> {
        let mut saw_force_repl = false;
        let mut saw_force_irrepl = false;
        for attr in &field.attrs {
            if !attr.meta.path().is_ident("epserde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("force_repl") {
                    if attrs.is_zero_copy {
                        return Err(meta.error(
                            "`force_repl` cannot be applied to a field of a zero-copy type",
                        ));
                    }
                    if meta.input.peek(syn::token::Paren) {
                        return Err(meta.error(
                            "`force_repl` is a field-level marker and takes no arguments; \
                             use `#[epserde(force_repl)]`",
                        ));
                    }
                    saw_force_repl = true;
                } else if meta.path.is_ident("force_irrepl") {
                    if attrs.is_zero_copy {
                        return Err(meta.error(
                            "`force_irrepl` cannot be applied to a field of a zero-copy type",
                        ));
                    }
                    if meta.input.peek(syn::token::Paren) {
                        return Err(meta.error(
                            "`force_irrepl` is a field-level marker and takes no arguments; \
                             use `#[epserde(force_irrepl)]`",
                        ));
                    }
                    saw_force_irrepl = true;
                }
                Ok(())
            })?;
        }
        if saw_force_repl && saw_force_irrepl {
            return Err(syn::Error::new_spanned(
                field,
                "`force_repl` and `force_irrepl` are mutually exclusive on the same field",
            ));
        }
        if saw_force_irrepl && get_ident(&field.ty).is_none() {
            return Err(syn::Error::new_spanned(
                field,
                "`force_irrepl` may only be applied to a field whose type is a single-segment \
                 struct generic; there is no direct parameter occurrence to reclassify here",
            ));
        }
        Ok(())
    };

    let validate_fields = |fields: &syn::Fields| -> Result<(), syn::Error> {
        for field in fields {
            validate_field(field)?;
        }
        Ok(())
    };

    if let Err(e) = match &derive_input.data {
        Data::Struct(s) => validate_fields(&s.fields),
        Data::Enum(e) => {
            e.variants.iter().try_for_each(|v| validate_fields(&v.fields))
        }
        Data::Union(_) => Ok(()),
    } {
        return e.to_compile_error().into();
    }
```

This validation runs once before the impl is generated. Errors are spanned on the offending attribute.

- [ ] **Step 7: Build**

Run: `cargo build --all-features`
Expected: clean build (no `dead_code` warnings).

- [ ] **Step 8: Run all tests**

Run: `cargo test -- --skip fail` then `cargo test fail`.

Expected: all green. test_force_repl.rs passes (Task 5 rewrote it). test_phantom.rs's `test_phantom_data_substitution` passes (T in `data: T` is replaceable, T in PhantomData is ignored, no conflict). test_phantom.rs's `test_not_serializable_in_phantom` passes (D only inside PhantomData, classified as neither, no `D: DeserInner` bound).

If `test_phantom_data_substitution` fails with a "both replaceable and irreplaceable" diagnostic, the PhantomData exception in `collect_occurrences` isn't being honoured — verify the recursion's `inside_phantom` flag is propagated correctly when descending into `PhantomData<…>`'s angle-bracketed arguments.

- [ ] **Step 9: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Remove struct-level force_repl machinery; validate field marker

Tears out:
- EpserdeAttrs::force_repl and its parser arm
- EpserdeContext::force_repl and its validation
- The T: SerInner / T: DeserInner bound injection in struct/enum impls
- The bound-skip in gen_ser_deser_where_clauses (Rust #152409 workaround)
- The type_contains_any helper

Adds per-field validation: rejects force_repl with arguments, rejects
force_repl on a field of a zero-copy struct/enum.
EOF
)"
```

---

## Task 7: Add conflict diagnostic + the `both_repl_and_irrepl` fail fixture

**Files:**
- Modify: `epserde-derive/src/lib.rs` (emit diagnostic when classifier reports a conflict)
- Create: `epserde/tests/fail/both_repl_and_irrepl.rs`, `epserde/tests/fail/both_repl_and_irrepl.stderr`

- [ ] **Step 1: Emit the conflict diagnostic**

In `gen_epserde_struct_impl`, after computing `classifications` and before building `repl_params`, scan for conflicts and short-circuit:

```rust
    // Detect "both replaceable and irreplaceable" conflicts and surface
    // a clear diagnostic before the impl is generated.
    for c in &classifications {
        if !c.replaceable_in.is_empty() && !c.irreplaceable_in.is_empty() {
            let mut msg = format!(
                "type parameter `{}` is both replaceable and irreplaceable",
                c.ident,
            );
            msg.push_str("\nnote: replaceable: appears in field(s) ");
            for (i, fname) in c.replaceable_in.iter().enumerate() {
                if i > 0 { msg.push_str(", "); }
                msg.push_str(&format!("`{}`", fname));
            }
            msg.push_str("\nnote: irreplaceable: appears as a type argument inside unmarked field(s) ");
            for (i, fname) in c.irreplaceable_in.iter().enumerate() {
                if i > 0 { msg.push_str(", "); }
                msg.push_str(&format!("`{}`", fname));
            }
            msg.push_str("\nhelp: add `#[epserde(force_repl)]` to the irreplaceable field(s) if their type substitutes the parameter transitively");
            return syn::Error::new_spanned(c.ident, msg).to_compile_error();
        }
    }
```

Add the same loop at the corresponding location in `gen_epserde_enum_impl`.

- [ ] **Step 2: Build**

Run: `cargo build --all-features`
Expected: clean build.

- [ ] **Step 3: Existing tests still pass**

Run: `cargo test -- --skip fail`
Expected: PASS. The new diagnostic only fires when the classifier records a conflict — no existing test does.

- [ ] **Step 4: Create the conflict fail fixture**

Create `epserde/tests/fail/both_repl_and_irrepl.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: a parameter appearing both as a direct field
 * and inside an unmarked wrapper field is classified as both
 * replaceable and irreplaceable, triggering the derive's conflict
 * diagnostic.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct A<T>(T);

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct BothReplAndIrrepl<T> {
    direct: T,
    wrapped: A<T>,
}

fn main() {
    let _ = BothReplAndIrrepl::<u32> { direct: 0, wrapped: A(0) };
}
```

- [ ] **Step 5: Materialise the `.stderr`**

Run: `TRYBUILD=overwrite cargo test fail`

If the run fails because of trybuild's "successfully created new stderr files" mode, look in `epserde/wip/` for the generated `.stderr` and move it into place:

```bash
mv /Users/vigna/git/epserde-rs/epserde/wip/both_repl_and_irrepl.stderr \
   /Users/vigna/git/epserde-rs/epserde/tests/fail/both_repl_and_irrepl.stderr
rmdir /Users/vigna/git/epserde-rs/epserde/wip 2>/dev/null || true
```

Inspect the file: it should contain the multi-line diagnostic emitted in Step 1.

- [ ] **Step 6: Verify all trybuild fixtures pass**

Run: `cargo test fail`
Expected: PASS — the new fixture is included alongside the existing ones.

- [ ] **Step 7: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add \
  epserde-derive/src/lib.rs \
  epserde/tests/fail/both_repl_and_irrepl.rs \
  epserde/tests/fail/both_repl_and_irrepl.stderr
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Emit diagnostic when a parameter is both replaceable and irreplaceable

The classifier now emits a clear, spanned syn::Error when a generic
parameter ends up classified in both buckets, naming the offending
fields. Adds the both_repl_and_irrepl trybuild fixture to capture the
diagnostic text.
EOF
)"
```

---

## Task 8: Additional fail fixtures (force_repl + force_irrepl misuse)

**Files:**
- Create: `epserde/tests/fail/force_repl_on_item.rs`, `.stderr`
- Create: `epserde/tests/fail/force_repl_with_args.rs`, `.stderr`
- Create: `epserde/tests/fail/force_irrepl_on_non_param.rs`, `.stderr`
- Create: `epserde/tests/fail/force_irrepl_on_zero_copy.rs`, `.stderr`
- Create: `epserde/tests/fail/force_repl_and_irrepl_together.rs`, `.stderr`

- [ ] **Step 1: Create the on-item fail fixture**

Create `epserde/tests/fail/force_repl_on_item.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_repl is a field-level marker. Placing
 * it on the item itself is rejected.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl)]
struct OnItem<T>(T);

fn main() {
    let _ = OnItem::<u32>(0);
}
```

- [ ] **Step 2: Create the with-args fail fixture**

Create `epserde/tests/fail/force_repl_with_args.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_repl as a field marker takes no
 * arguments; using it with arguments is rejected.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct WithArgs<T> {
    #[epserde(force_repl(T))]
    inner: T,
}

fn main() {
    let _ = WithArgs::<u32> { inner: 0 };
}
```

- [ ] **Step 3: Create the force_irrepl on-non-param fixture**

Create `epserde/tests/fail/force_irrepl_on_non_param.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_irrepl is only valid on a field whose
 * type is a single-segment struct generic. Applying it to a concrete
 * wrapper is rejected at derive time.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct OnNonParam<T> {
    #[epserde(force_irrepl)]
    bad: Vec<T>,
}

fn main() {
    let _: OnNonParam<u32> = OnNonParam { bad: vec![] };
}
```

- [ ] **Step 4: Create the force_irrepl on-zero-copy fixture**

Create `epserde/tests/fail/force_irrepl_on_zero_copy.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_irrepl on a field of a zero-copy type
 * is rejected.
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[repr(C)]
struct OnZeroCopy<T: Copy> {
    #[epserde(force_irrepl)]
    inner: T,
}

fn main() {
    let _ = OnZeroCopy::<u32> { inner: 0 };
}
```

- [ ] **Step 5: Create the mutex fixture**

Create `epserde/tests/fail/force_repl_and_irrepl_together.rs`:

```rust
/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: the two markers are mutually exclusive on
 * the same field.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Both<T> {
    #[epserde(force_repl)]
    #[epserde(force_irrepl)]
    inner: T,
}

fn main() {
    let _ = Both::<u32> { inner: 0 };
}
```

- [ ] **Step 6: Run with `TRYBUILD=overwrite` to materialise the `.stderr` files**

Run: `TRYBUILD=overwrite cargo test fail`

Move the generated `.stderr` files from `epserde/wip/` into `epserde/tests/fail/`:

```bash
for name in force_repl_on_item force_repl_with_args \
            force_irrepl_on_non_param force_irrepl_on_zero_copy \
            force_repl_and_irrepl_together; do
  mv "/Users/vigna/git/epserde-rs/epserde/wip/${name}.stderr" \
     "/Users/vigna/git/epserde-rs/epserde/tests/fail/${name}.stderr"
done
rmdir /Users/vigna/git/epserde-rs/epserde/wip 2>/dev/null || true
```

Inspect each `.stderr`: each should contain a clear rejection message tied to the validation arm that fired.

- [ ] **Step 7: Run all trybuild fixtures**

Run: `cargo test fail`
Expected: PASS — every fixture under `tests/fail/` passes.

- [ ] **Step 8: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add \
  epserde/tests/fail/force_repl_on_item.rs \
  epserde/tests/fail/force_repl_on_item.stderr \
  epserde/tests/fail/force_repl_with_args.rs \
  epserde/tests/fail/force_repl_with_args.stderr \
  epserde/tests/fail/force_irrepl_on_non_param.rs \
  epserde/tests/fail/force_irrepl_on_non_param.stderr \
  epserde/tests/fail/force_irrepl_on_zero_copy.rs \
  epserde/tests/fail/force_irrepl_on_zero_copy.stderr \
  epserde/tests/fail/force_repl_and_irrepl_together.rs \
  epserde/tests/fail/force_repl_and_irrepl_together.stderr
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Add compile-fail fixtures for force_repl and force_irrepl misuse

Captures the validation errors emitted by the derive at attribute-
parsing time, before any impl is generated:
- force_repl on the item rather than a field
- force_repl invoked with arguments (struct-level shape)
- force_irrepl on a field whose type is not a single-segment generic
- force_irrepl on a field of a zero-copy type
- force_repl and force_irrepl applied to the same field
EOF
)"
```

---

## Task 9: Update the README

**Files:**
- Modify: `README.md` (workspace root; `epserde/README.md` is a symlink)

- [ ] **Step 1: Rewrite the `force_repl` example section**

In `README.md`, find the section currently titled `## Example: Forcing transitive replaceability with \`force_repl\``. Replace the section body (the example structs, the doctest, and the surrounding prose) with one that uses the field-level marker. Concretely:

```markdown
## Example: Forcing transitive replaceability with `force_repl`

The default behaviour described above means that a type parameter `T` is
substituted with its associated deserialization type only when it appears as
the exact type of one of the item's fields. If `T` is buried inside a wrapper
— for example, in `struct Outer<T>(Inner<T>)` — then `Outer<…>::DeserType<'_>`
keeps `T` unchanged, and the ε-copy deserialized form does not benefit from
`T`'s own ε-copy form.

The field-level attribute `#[epserde(force_repl)]` lifts this default for one
field. Marking a field tells the derive to substitute every type parameter
appearing inside that field's type in `Self::DeserType<'_>` (and in
`Self::SerType`), and to dispatch the field's ε-deserialization through the
wrapper's own `DeserInner` impl:

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq)]
struct Inner<T>(T);

#[derive(Epserde, Debug, PartialEq)]
struct Outer<T> {
    #[epserde(force_repl)]
    inner: Inner<T>,
}

let s: Outer<Vec<isize>> = Outer { inner: Inner(vec![0, 1, 2, 3]) };
let mut file = std::env::temp_dir();
file.push("serialized_force_repl");
unsafe { s.store(&file)? };
let b = std::fs::read(&file)?;

// Without the marker, the ε-copy form of Outer<Vec<isize>> would keep
// the inner field's type as Inner<Vec<isize>>. With it, the inner field
// is replaced by Inner<&[isize]> — just like a top-level vector field
// would be.
let t = unsafe { <Outer<Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.inner.0.as_slice(), t.inner.0);
# Ok(())
# }
```

`#[epserde(force_repl)]` makes the attribute's contract local and explicit:
the field's type must substitute its parameters transitively in its own
associated deserialization type (`<F<…> as DeserInner>::DeserType<'a>`
must be `F<… A::DeserType<'a> …>`). Standard library wrappers that satisfy
the contract include `Box<T>`, `Rc<T>`, `Arc<T>`, `Option<T>`, the `Range`
family, tuples, and arrays for deep-copy `T`; `Epserde`-derived types
satisfy it for their naturally-replaceable parameters. `Vec<T>`,
`Box<[T]>`, `[T; N]`, and `String` satisfy the contract only when their
parameter is deep-copy — using the marker with a zero-copy inner parameter
produces a compile error in the derived ε-deserialization body.

A violated contract surfaces as a type mismatch in the derived
`_deser_eps_inner`, not as silent miscompilation. `force_repl` also lifts
the restriction (described earlier) that a parameter cannot appear both as
a direct field type and as a type argument of another field's type — by
marking the wrapping field, that occurrence stops contributing to
irreplaceability.

The symmetric `#[epserde(force_irrepl)]` marker resolves the same
restriction from the opposite side: applied to a field whose type is a
single-segment struct generic (e.g. `data: T`), it reclassifies that
direct occurrence from replaceable to irreplaceable AND flips the field's
dispatch from ε-deserialization to full-copy. Use it when you want the
parameter to stay un-substituted across the struct — for example, because
a sibling field's type contains the parameter as a type argument and
substituting would not make sense.
```

- [ ] **Step 2: Refresh the "Replaceable and irreplaceable parameters" section**

In `README.md`, find the section `### Replaceable and irreplaceable parameters`. Replace the body with:

```markdown
### Replaceable and irreplaceable parameters

Given a type `T` with generics, we say that a type parameter is _replaceable_
if it appears as the direct (single-segment) type of a field of `T` that
does not carry `#[epserde(force_irrepl)]`, or anywhere inside the type of a
field of `T` carrying `#[epserde(force_repl)]`. We say that a type parameter
is _irreplaceable_ if it appears as a type argument inside the type of a
field of `T` that does NOT carry `#[epserde(force_repl)]`, or as the direct
type of a field carrying `#[epserde(force_irrepl)]`. Occurrences nested
inside `PhantomData<…>` count toward neither classification — the derive's
PhantomData handling makes such occurrences neutral.

The basic assumption in what follows, and in the derived code of ε-serde, is
that no type parameter is both replaceable and irreplaceable. If the
classifier finds a parameter in both buckets, the derive emits a compile-time
error spanned on the parameter, listing the offending fields and suggesting
that the irreplaceable field be marked with `#[epserde(force_repl)]` when
appropriate.

For example, in the following structure

```rust
struct Bad<A> {
    data: A,
    vec: Vec<A>,
}
```

the type parameter `A` is both replaceable (it is the type of `data`) and
irreplaceable (it appears as a type argument of `vec`'s type). The derive
rejects this with the new diagnostic. The user can either restructure or
mark the offending field:

```rust
struct Fixed<A> {
    data: A,
    #[epserde(force_repl)]
    vec: Vec<A>,
}
```

When `Vec<A>` substitutes `A` transitively (which holds for deep-copy `A`),
`Fixed<A>::DeserType<'_>` becomes `Fixed<A::DeserType<'_>>` and the inner
slot is consistent. When the contract is violated (for example, with
zero-copy `A`, where `Vec<A>::DeserType<'_>` is `&[A]` rather than
`Vec<A::DeserType<'_>>`), Rust reports a type mismatch in the derived
ε-deserialization body. You can also use the `bound` attribute to solve
some cases (e.g., when `DeserType<'_, A>` is equal to `A` — see the example
above about pinning associated types).
```

- [ ] **Step 3: Refresh the cross-link in the "User-defined structures with parameters" section**

Earlier in `README.md`, a paragraph reads (roughly):

> Finally, when every occurrence of the parameter can be substituted consistently
> (the wrapper around the parameter must itself substitute its own parameter
> transitively), you can use the [`force_repl`
> attribute](#example-forcing-transitive-replaceability-with-force_repl) to opt the
> parameter into substitution everywhere it appears.

Update it to describe the field-level surface:

> Finally, if you need a parameter to be replaceable through a concrete
> wrapper rather than as the direct type of a field, mark the wrapping field
> with the [field-level `force_repl`
> attribute](#example-forcing-transitive-replaceability-with-force_repl). The
> marker lifts the wrapper's parameter occurrences out of the irreplaceability
> classification.

- [ ] **Step 4: Refresh the cross-link in "Note, however, that field types are not replaced…"**

Find the paragraph that says:

> Note, however, that field types are not replaced if they are not type
> parameters. In particular, by default you cannot have `T` both as the type
> of a field and as a type parameter of another field; the structure-level
> [`#[epserde(force_repl(T))]`](#example-forcing-transitive-replaceability-with-force_repl)
> attribute lifts this restriction…

Replace with:

> Note, however, that field types are not replaced if they are not type
> parameters. In particular, by default you cannot have `T` both as the type
> of a field and as a type parameter of another field; marking the latter
> field with [`#[epserde(force_repl)]`](#example-forcing-transitive-replaceability-with-force_repl)
> lifts the restriction when the wrapper substitutes its parameter
> transitively, and `PhantomData<T>` is handled natively by the derive so
> it never blocks compilation here.

- [ ] **Step 5: Refresh the "Serialization and deserialization types" section**

In that section, the bullet describing where substitution happens currently mentions a parameter named in `#[epserde(force_repl(…))]` counting as replaceable. Update it:

> - if `T` is a deep-copy concrete type obtained by resolving the type parameters
>   `P₀`, `P₁`, `P₂`, … of a type definition (struct or enum) to concrete types
>   `T₀`, `T₁`, `T₂`, …, then the deserialization type is obtained by resolving
>   each replaceable type parameter `Pᵢ` with the deserialization type of `Tᵢ`
>   instead. (Note that the first rule still applies, so if `Tᵢ` is zero-copy
>   the its deserialization type is `&Tᵢ`.) A parameter is replaceable iff it
>   appears as the direct type of some field or anywhere inside the type of a
>   field marked with `#[epserde(force_repl)]`.

Adjacent prose using `#[epserde(force_repl(…))]` (struct-level form) should be updated to refer to the field-level marker.

- [ ] **Step 6: Run the README doctest**

Run: `cargo test --doc -p epserde`
Expected: PASS — the new doctest in the `force_repl` example section compiles and runs.

- [ ] **Step 7: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add README.md
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
README: rewrite force_repl story against field-level surface

Replaces the struct-level force_repl(T) example and the surrounding
prose with a field-level Outer<T> { #[epserde(force_repl)] inner:
Inner<T> } example. Refreshes the "Replaceable and irreplaceable
parameters" section to describe the per-field marker rule and the
new conflict diagnostic, and updates cross-references elsewhere in
the README.
EOF
)"
```

---

## Task 10: Update `CLAUDE.md` and the derive doc-comment

**Files:**
- Modify: `CLAUDE.md`
- Modify: `epserde-derive/src/lib.rs` (the `#[derive(Epserde)]` doc comment)

- [ ] **Step 1: Update `CLAUDE.md`'s "Key Invariants" entry**

Open `CLAUDE.md`. Find the "Key Invariants" line currently reading:

> - A replaceable type parameter must not appear both as a field type and as a parameter of another field type, unless the type is annotated with `#[epserde(force_repl(T))]`

Replace with:

> - A replaceable type parameter must not appear both as a field type and as a type argument of another field's type. The restriction can be lifted by marking the wrapper field with `#[epserde(force_repl)]` when its type substitutes the parameter transitively, or alternatively by marking the direct field with `#[epserde(force_irrepl)]` to pin the parameter as irreplaceable across the struct. Occurrences inside `PhantomData<…>` do not count toward either classification.

- [ ] **Step 2: Update the derive doc-comment**

In `epserde-derive/src/lib.rs`, find the doc-comment block immediately preceding `#[proc_macro_derive(Epserde, …)]`. Replace the `# \`force_repl\` attribute` subsection (added in the earlier feature) with:

```rust
/// # `force_repl` and `force_irrepl` attributes
///
/// Two symmetric **field-level** markers (no arguments) that override the
/// default replaceable/irreplaceable classification for the field they
/// decorate.
///
/// `#[epserde(force_repl)]` applies to a field whose type is concrete
/// (e.g. `A<T>`, `Vec<T>`). It reclassifies the parameter occurrences inside
/// that type from contributing to *irreplaceability* to contributing to
/// *replaceability*, and flips the field's dispatch from full-copy to
/// ε-deserialization. The wrapper's `DeserInner` impl must substitute its
/// parameters uniformly in `DeserType<'_>` (and its `SerInner` impl in
/// `SerType`):
/// `<F<A, B, …> as DeserInner>::DeserType<'a> == F<A::DeserType<'a>, B::DeserType<'a>, …>`.
/// Standard library wrappers that satisfy the contract: `Box<T>`, `Rc<T>`,
/// `Arc<T>`, `Option<T>`, `Range<T>` and its kin, tuples, and arrays for
/// deep-copy `T`. `Vec<T>`/`Box<[T]>`/`[T; N]`/`String` satisfy it only for
/// deep-copy inner parameters. `PhantomData<T>` is handled natively and
/// always works.
///
/// `#[epserde(force_irrepl)]` applies to a field whose type is a single-
/// segment struct generic (e.g. `data: T`). It reclassifies that direct
/// occurrence from contributing to *replaceability* to contributing to
/// *irreplaceability*, and flips the field's dispatch from ε-deserialization
/// to full-copy. The marker is useful when the parameter must stay
/// un-substituted across the struct — for example, because a sibling field
/// contains the parameter as a type argument and substituting would not be
/// meaningful.
///
/// Both markers are rejected on fields of `zero_copy` types and on the item
/// itself; both reject any arguments; they are mutually exclusive on the
/// same field; `force_irrepl` is rejected on fields whose type is not a
/// single-segment generic.
///
/// The pre-existing rule "a type parameter cannot be both replaceable and
/// irreplaceable" still holds. When the classifier sees a parameter in both
/// buckets, the derive emits a clear error naming the conflicting fields.
///
/// Example:
///
/// ```ignore
/// #[derive(Epserde)]
/// struct Inner<T>(T);
///
/// #[derive(Epserde)]
/// struct Outer<T> {
///     #[epserde(force_repl)]
///     inner: Inner<T>,
/// }
///
/// #[derive(Epserde)]
/// struct Stays<T> {
///     #[epserde(force_irrepl)]
///     data: T,            // stays as T in DeserType<'_>
///     wrapped: Vec<T>,    // T's type-argument occurrence here drives
///                         // the (now consistent) irreplaceable classification
/// }
/// ```
```

- [ ] **Step 3: Verify docs build cleanly**

Run: `cargo doc --no-deps --all-features`
Expected: PASS, no broken intra-doc links.

- [ ] **Step 4: Run the full validation suite**

Run sequentially:

- `cargo fmt -- --check`
- `cargo clippy --all-features`
- `cargo test -- --skip fail`
- `cargo test fail`

Expected: all PASS.

- [ ] **Step 5: Commit (after approval)**

```bash
git -C /Users/vigna/git/epserde-rs add CLAUDE.md epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Update CLAUDE.md and derive doc-comment for field-level force_repl

CLAUDE.md's "Key Invariants" entry now describes the field-level
mechanism for lifting the no-overlap restriction and notes the
PhantomData exception. The derive macro's doc-comment subsection on
force_repl is rewritten to describe the field-level marker, its
contract, the validation surface, and the conflict diagnostic.
EOF
)"
```

---

## Final verification

- [ ] **Run the full CI matrix locally**

Run:

```bash
cargo build --all-features
cargo fmt -- --check
cargo clippy --all-features
cargo test -- --skip fail
cargo test fail
cargo doc --no-deps --all-features
```

Expected: all six steps PASS.

- [ ] **Cross-check against the spec**

Confirm by inspection that:

- The classifier in `epserde-derive/src/lib.rs` implements both the `force_repl` rule and the `force_irrepl` rule described under "Conceptual semantics" and "Conflict diagnostic" in the spec.
- `gen_eps_deser_method_call` dispatches per the rule in "Effects on the derived code → per-field dispatch" (force_irrepl → full; force_repl → eps; no marker + single-segment generic → eps; otherwise full).
- `Self::DeserType<'a>` is produced by `gen_generics_for_deser_type` from `repl_params` derived by the classifier (matching "Self::DeserType<'a> and Self::SerType").
- The where-clause adds `T: DeserInner`/`T: SerInner` only where the field-type bound or the classifier-derived substitution require it (matching the bound rule).
- The struct-level `#[epserde(force_repl(T, U, …))]` is no longer parsed; `tests/fail/force_repl_on_item.rs`/`force_repl_with_args.rs` catch attempts to use the old surface.
- `tests/fail/force_irrepl_on_non_param.rs`/`force_irrepl_on_zero_copy.rs`/`force_repl_and_irrepl_together.rs` catch `force_irrepl` misuse and the mutex.
- `tests/fail/both_repl_and_irrepl.rs` captures the conflict diagnostic.
- `tests/test_phantom.rs` passes unchanged (the PhantomData exception in the classifier preserves `test_phantom_data_substitution` and `test_not_serializable_in_phantom`).

If all seven bullets check out, the implementation matches the spec.
