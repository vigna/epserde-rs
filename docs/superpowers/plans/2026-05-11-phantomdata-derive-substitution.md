# Native `PhantomData<T>` substitution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `Epserde` derive macro emit a literal `::core::marker::PhantomData` for fields whose type's last path segment is `PhantomData`, so that `PhantomData<T>` substitutes its parameter naturally in the derived `Self::DeserType<'a>` — obsoleting `PhantomDeserData` in all common cases while keeping it (deprecated) for backward compatibility.

**Architecture:** One new arm in `gen_eps_deser_method_call` in `epserde-derive/src/lib.rs`, placed immediately after the existing `PhantomDeserData` special case. The new arm emits a literal `PhantomData` whose generic parameter is inferred from the surrounding struct literal (which constructs `Self::DeserType<'a>`). No new helpers, no traits, no substitution computation in the macro. `PhantomDeserData` gets a `#[deprecated]` attribute with a message pointing users at plain `PhantomData<T>` and calling out the wire-format implication.

**Tech Stack:** Rust 2024 (MSRV 1.85), `syn` 2 / `quote` 1 (proc-macro crate). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-05-11-phantomdata-derive-substitution-design.md`

**Note on comment style:** In this codebase, `//` line comments use plain text (no backticks); `///` doc comments use backticks freely (they render via rustdoc). Apply this throughout.

---

## Task 1: Add the `PhantomData` literal-emission arm

**Files:**
- Modify: `epserde-derive/src/lib.rs` (the `gen_eps_deser_method_call` function)

- [ ] **Step 1: Locate the function**

Open `epserde-derive/src/lib.rs`. Find the function `gen_eps_deser_method_call`. It contains a path-match block that starts with `if let syn::Type::Path(syn::TypePath { qself: None, path: syn::Path { leading_colon: None, segments } }) = field_type`. Inside that block, find the existing `PhantomDeserData` special case:

```rust
        // This is a pretty weak check, as a user could define its own
        // PhantomDeserData, but it should be good enough in practice
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: unsafe { <#field_type>::_deser_eps_inner_special(backend)? });
            }
        }
```

- [ ] **Step 2: Add the new arm immediately after**

Replace the block above with:

```rust
        // This is a pretty weak check, as a user could define its own
        // PhantomDeserData, but it should be good enough in practice
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: unsafe { <#field_type>::_deser_eps_inner_special(backend)? });
            }
            // PhantomData<...> is handled natively: we emit a literal
            // PhantomData whose generic parameter is inferred from the
            // surrounding Self::DeserType<'a> struct literal. This
            // matches whatever substitution is applied to the parent
            // type, without the derive computing it explicitly.
            if segment.ident == "PhantomData" {
                return syn::parse_quote!(#field_name: ::core::marker::PhantomData);
            }
        }
```

- [ ] **Step 3: Build to verify nothing regressed**

Run: `cargo build --all-features` from `/Users/vigna/git/epserde-rs`.
Expected: clean build.

- [ ] **Step 4: Run the full test suite**

Run: `cargo test -- --skip fail`.
Expected: PASS. Existing `PhantomData` and `PhantomDeserData` tests in `epserde/tests/test_phantom.rs` should all keep passing — the new arm doesn't change their behavior because (a) tests using plain `PhantomData<T>` where `T` is non-replaceable land in the natural inference path; (b) tests using `PhantomDeserData<T>` continue to hit the existing `PhantomDeserData` arm first.

Run: `cargo test fail`.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git -C /Users/vigna/git/epserde-rs add epserde-derive/src/lib.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Emit literal PhantomData for PhantomData<...> fields in derive

Adds one arm to gen_eps_deser_method_call that detects field types
whose last path segment is PhantomData and emits the literal
::core::marker::PhantomData instead of a method call. Rust infers the
generic parameter from the surrounding Self::DeserType<'a> struct
literal, so the field's type matches the substituted slot without the
derive computing the substitution explicitly. This makes
PhantomData<T> behave correctly in structs where T is substituted
(both naturally and via enforce_repl), removing the need for the
PhantomDeserData workaround.
EOF
)"
```

---

## Task 2: Positive test — plain `PhantomData` substitution

**Files:**
- Modify: `epserde/tests/test_phantom.rs`

- [ ] **Step 1: Append the new test**

Open `epserde/tests/test_phantom.rs` and append the following at the end of the file:

```rust
// New behaviour from the native PhantomData arm in the derive:
// a struct with T both as a direct field and inside PhantomData<T>
// now compiles and round-trips correctly, without PhantomDeserData.
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct DataWithPhantomData<T> {
    data: T,
    phantom: PhantomData<T>,
}

#[test]
fn test_phantom_data_substitution() -> anyhow::Result<()> {
    let obj: DataWithPhantomData<Vec<i32>> = DataWithPhantomData {
        data: vec![1, 2, 3, 4],
        phantom: PhantomData,
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { obj.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <DataWithPhantomData<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(obj, full);

    let eps = unsafe { <DataWithPhantomData<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    // The data field comes back as &[i32] (Vec<i32>::DeserType<'_>).
    assert_eq!(obj.data.as_slice(), eps.data);
    // The phantom field has type PhantomData<&[i32]>. The annotation
    // forces the type-check; if PhantomData were not substituting its
    // parameter, this line would fail to compile.
    let _phantom_check: PhantomData<&[i32]> = eps.phantom;

    Ok(())
}
```

- [ ] **Step 2: Run the new test**

Run: `cargo test --test test_phantom test_phantom_data_substitution` from `/Users/vigna/git/epserde-rs`.
Expected: PASS.

The crucial line is `let _phantom_check: PhantomData<&[i32]> = eps.phantom;`. If `DataWithPhantomData<Vec<i32>>::DeserType<'_>` had a `phantom` slot of type `PhantomData<Vec<i32>>` instead of `PhantomData<&[i32]>`, this line would fail to compile.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test -- --skip fail`.
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git -C /Users/vigna/git/epserde-rs add epserde/tests/test_phantom.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Test plain PhantomData<T> substitution in deep-copy struct

Round-trips a DataWithPhantomData<Vec<i32>> and asserts the ε-form has
type DataWithPhantomData with a phantom field of type
PhantomData<&[i32]>. Type-annotated binding forces the check at
compile time.
EOF
)"
```

---

## Task 3: Positive test — `PhantomData` substitution under `enforce_repl`

**Files:**
- Modify: `epserde/tests/test_phantom.rs`

- [ ] **Step 1: Append the new test**

Open `epserde/tests/test_phantom.rs` and append the following at the end of the file:

```rust
// PhantomData<T> as the only mention of T in a struct: without
// enforce_repl(T), T is non-replaceable and the phantom field stays
// PhantomData<T> after deserialization. With enforce_repl(T), T is
// substituted, and the derive's native PhantomData arm produces
// PhantomData<T::DeserType<'_>> for the phantom slot.
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
#[epserde(enforce_repl(T))]
struct OnlyPhantomEnforceRepl<T> {
    other: u32,
    phantom: PhantomData<T>,
}

#[test]
fn test_phantom_data_enforce_repl() -> anyhow::Result<()> {
    let obj: OnlyPhantomEnforceRepl<Vec<i32>> = OnlyPhantomEnforceRepl {
        other: 42,
        phantom: PhantomData,
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { obj.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full =
        unsafe { <OnlyPhantomEnforceRepl<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(obj, full);

    let eps =
        unsafe { <OnlyPhantomEnforceRepl<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(obj.other, eps.other);
    // The phantom slot must be PhantomData<&[i32]> after enforce_repl
    // substitutes T into <Vec<i32> as DeserInner>::DeserType<'_>.
    let _phantom_check: PhantomData<&[i32]> = eps.phantom;

    Ok(())
}
```

- [ ] **Step 2: Run the new test**

Run: `cargo test --test test_phantom test_phantom_data_enforce_repl`.
Expected: PASS.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test -- --skip fail`.
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git -C /Users/vigna/git/epserde-rs add epserde/tests/test_phantom.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Test PhantomData substitution driven by enforce_repl

Round-trips OnlyPhantomEnforceRepl<Vec<i32>> and asserts that the
phantom slot is substituted to PhantomData<&[i32]>. This is the case
that enforce_repl alone could not solve previously: it now works
through the derive's native PhantomData arm.
EOF
)"
```

---

## Task 4: Deprecate `PhantomDeserData`

**Files:**
- Modify: `epserde/src/lib.rs` (the `PhantomDeserData` struct definition and its impls)

- [ ] **Step 1: Locate `PhantomDeserData`**

Open `epserde/src/lib.rs`. Find the `PhantomDeserData` struct definition (search for `pub struct PhantomDeserData`). It is preceded by a `///` doc-comment block describing it.

- [ ] **Step 2: Add `#[deprecated]` to the struct**

Immediately above the `#[derive(...)]` line of `PhantomDeserData`, insert a `#[deprecated]` attribute. The struct currently looks like:

```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhantomDeserData<T>(pub PhantomData<T>);
```

Replace those two lines with:

```rust
#[deprecated(
    since = "0.13.0",
    note = "use plain `PhantomData<T>` instead — the `Epserde` derive now substitutes \
its parameter natively. Note: switching an existing struct from \
`PhantomDeserData<T>` to `PhantomData<T>` changes the struct's type hash, so \
previously-serialized files will fail to deserialize against the new definition."
)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhantomDeserData<T>(pub PhantomData<T>);
```

- [ ] **Step 3: Suppress the deprecation warning at the derive-generated use site**

The `#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]` on the struct itself, and the various trait impls below it (`CopyType`, `AlignTo`, `TypeHash`, `AlignHash`, `SerInner`, `DeserInner`), will trigger the deprecation warning when they mention `PhantomDeserData`. Wrap each impl block and the inherent `impl<T: DeserInner> PhantomDeserData<T> { ... }` with `#[allow(deprecated)]`.

Find each of the following impl blocks (they all appear consecutively after the struct definition) and add `#[allow(deprecated)]` immediately above each one. The impls to annotate are:

1. `impl<T: DeserInner> PhantomDeserData<T> { … }` (the inherent impl with `_deser_eps_inner_special`)
2. `unsafe impl<T> CopyType for PhantomDeserData<T> { … }`
3. `impl<T> AlignTo for PhantomDeserData<T> { … }`
4. `impl<T: TypeHash> TypeHash for PhantomDeserData<T> { … }`
5. `impl<T> AlignHash for PhantomDeserData<T> { … }`
6. `impl<T> SerInner for PhantomDeserData<T> { … }`
7. `impl<T: DeserInner> DeserInner for PhantomDeserData<T> { … }`

Example shape (apply to each impl):

```rust
#[allow(deprecated)]
impl<T> AlignTo for PhantomDeserData<T> {
    // ...
}
```

- [ ] **Step 4: Build**

Run: `cargo build --all-features` from `/Users/vigna/git/epserde-rs`.
Expected: clean build. No warnings about uses of deprecated `PhantomDeserData` inside `epserde/src/lib.rs`.

- [ ] **Step 5: Run the full test suite**

Run: `cargo test -- --skip fail`.
Expected: PASS. The existing tests in `epserde/tests/test_phantom.rs` that use `PhantomDeserData` (`test_deser_phantom_deep_copy`, `test_deser_phantom_zero_copy`) will emit deprecation warnings at the test-file level; the tests themselves must still pass. The new tests from Tasks 2 and 3 must also still pass.

If the deprecation warnings on the existing tests are treated as errors (e.g. by `-D warnings` in CI), the tests will fail. The fix is to add `#[allow(deprecated)]` at the top of `epserde/tests/test_phantom.rs` (a single `#![allow(deprecated)]` inner attribute right after the SPDX header). Do this now to keep the existing tests passing while leaving the deprecation warning visible to downstream users.

Concretely, modify `epserde/tests/test_phantom.rs` by inserting after the existing SPDX comment block and before the first `use` line:

```rust
// Tests still exercise the deprecated PhantomDeserData type for
// backward-compatibility coverage; suppress the warnings file-wide.
#![allow(deprecated)]
```

Re-run `cargo test -- --skip fail`. Expected: PASS, no deprecation-as-error failures.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --all-features`.
Expected: PASS. Any warning about a use of `PhantomDeserData` outside the impl-block annotations should be suppressed by Step 3 or by the file-level allow in Step 5. If clippy still flags something inside `epserde/src/lib.rs` or `epserde/tests/test_phantom.rs`, add a targeted `#[allow(deprecated)]` on that specific item rather than widening the suppression.

- [ ] **Step 7: Commit**

```bash
git -C /Users/vigna/git/epserde-rs add epserde/src/lib.rs epserde/tests/test_phantom.rs
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Deprecate PhantomDeserData

The Epserde derive now handles PhantomData<T> natively (see commit
adding the PhantomData arm to gen_eps_deser_method_call), so
PhantomDeserData is no longer necessary for new code. Marks the type
with #[deprecated], pointing users to plain PhantomData<T> and noting
that migrating an existing struct changes its type hash. Existing
impl blocks and the backward-compat tests gain #[allow(deprecated)] so
the type remains fully functional for downstream code that still
references it.
EOF
)"
```

---

## Task 5: Update README with the new behaviour and deprecation note

**Files:**
- Modify: `README.md` (workspace root; `epserde/README.md` is a symlink pointing here)

- [ ] **Step 1: Locate the existing `PhantomData`-related prose**

Open `README.md`. Find the existing example block that ends roughly with the line `### PhantomData` heading or look for the `compile_fail` doctest that uses `PhantomData<T>` to motivate `PhantomDeserData`. The relevant doctest contains lines similar to:

```rust
struct Data<T> {
    data: T,
    phantom: PhantomData<T>,
}
```

inside a `` ```compile_fail `` block, followed by a working example using `PhantomDeserData<T>`.

- [ ] **Step 2: Convert the `compile_fail` doctest into a working `rust` doctest**

Locate the `` ```compile_fail `` fence. Change it to `` ```rust ``. Wrap the snippet so it forms a valid runnable doctest (use the same `# use epserde::prelude::*;` / `# fn main() -> Result<(), Box<dyn std::error::Error>> {` / `# Ok(()) }` framing that other doctests in the README use). The full doctest should be:

```rust
# use epserde::prelude::*;
# use std::marker::PhantomData;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct Data<T> {
    data: T,
    phantom: PhantomData<T>,
}

let s: Data<Vec<isize>> = Data { data: vec![0, 1, 2, 3], phantom: PhantomData };
let mut file = std::env::temp_dir();
file.push("serialized_phantom");
unsafe { s.store(&file)? };
let b = std::fs::read(&file)?;

// The data field is substituted to &[isize] in the ε-deserialized form,
// and so is the phantom slot.
let t = unsafe { <Data<Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.data.as_slice(), t.data);
let _phantom_check: PhantomData<&[isize]> = t.phantom;
#     Ok(())
# }
```

- [ ] **Step 3: Update the surrounding prose**

Find the paragraph above the (previously) compile-failing example. It currently explains why `PhantomData<T>` fails to compile in this context and steers the user towards `PhantomDeserData<T>`. Replace the explanation with a description of the new behaviour. Concretely, the new paragraph should say roughly:

> The derive macro substitutes `T` natively inside `PhantomData<T>` fields, so the example below compiles and round-trips correctly without any special wrapper. (Earlier versions of ε-serde required a dedicated `PhantomDeserData<T>` type for this; that type is now deprecated.)

Adapt the wording to fit the surrounding tone — the existing prose uses sentences like "Note how the field originally of type … remains of the same type". Match that style.

- [ ] **Step 4: Add a deprecation note covering the wire-format implication**

After the converted doctest, insert a short paragraph noting:

> `PhantomDeserData<T>` is kept as a deprecated alias for backward compatibility. Migrating an existing struct from `PhantomDeserData<T>` to `PhantomData<T>` changes the struct's type hash, so previously-serialised files will fail to deserialise against the new definition; re-serialise the data after migration.

- [ ] **Step 5: Run the doctest**

Run: `cargo test --doc -p epserde` from `/Users/vigna/git/epserde-rs`.
Expected: PASS. The converted doctest must compile and run.

- [ ] **Step 6: Build docs**

Run: `cargo doc --no-deps --all-features`.
Expected: PASS, no broken intra-doc links or unclosed code fences.

- [ ] **Step 7: Run the full check suite**

Run sequentially: `cargo fmt -- --check`, `cargo clippy --all-features`, `cargo test -- --skip fail`, `cargo test fail`.
Expected: all PASS.

- [ ] **Step 8: Commit**

```bash
git -C /Users/vigna/git/epserde-rs add README.md
git -C /Users/vigna/git/epserde-rs commit -m "$(cat <<'EOF'
Document native PhantomData substitution; deprecate PhantomDeserData in README

Converts the previously compile_fail example into a working doctest
that exercises the new derive behaviour. Adds a deprecation note for
PhantomDeserData that calls out the type-hash wire-format implication
of migrating from PhantomDeserData<T> to plain PhantomData<T>.
EOF
)"
```

---

## Final verification

- [ ] **Run the full CI matrix locally**

Run sequentially: `cargo build --all-features`, `cargo fmt -- --check`, `cargo clippy --all-features`, `cargo test -- --skip fail`, `cargo test fail`, `cargo doc --no-deps --all-features`.
Expected: all PASS.

- [ ] **Manual confirmation of the spec's two motivating scenarios**

Confirm by inspection that:

1. `test_phantom_data_substitution` (added in Task 2) exercises the canonical case where `T` is both a direct field and inside `PhantomData<T>`, and asserts via the type annotation `let _phantom_check: PhantomData<&[i32]> = eps.phantom;` that the phantom slot is substituted.
2. `test_phantom_data_enforce_repl` (added in Task 3) exercises the `enforce_repl` interaction case and asserts the same substitution.

If both pass, the implementation matches the spec.
