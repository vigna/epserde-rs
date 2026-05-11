# Field-level `#[epserde(force_repl)]` and `#[epserde(force_irrepl)]`

## Motivation

The earlier struct-level `#[epserde(force_repl(T, U, …))]` design (spec at
`docs/superpowers/specs/2026-05-11-epserde-force-repl-attribute-design.md`)
broadened per-field ε-dispatch via a `type_contains_any` walk over every
field's type. Reviewing the implementation produced concrete counter-examples
(see `epserde/tests/counter_*.rs` on this branch) showing that the
parent-level parameter substitution and the wrapper-level parameter
substitution diverge whenever a wrapper carries more parameters than the
parent classifies as replaceable. For example, with

```rust
struct A<X, Y>(X, Y);
struct S<T, U>(T, A<T, U>);
```

`A<T, U>::DeserType<'a>` is `A<T::DeserType<'a>, U::DeserType<'a>>` per `A`'s
own derive, but `S<T, U>::DeserType<'a>` substitutes only `T` (the parameter
made replaceable by the direct field) and leaves `U` alone — so the slot for
field `A<T, U>` in the substituted parent is `A<T::DeserType<'a>, U>`. The
two forms disagree; the derived `_deser_eps_inner` body fails to type-check.
The mismatch is silent at attribute level and surfaces as an opaque Rust
issue #152409 projection error inside generated code.

The fix is to move the attribute from the struct/enum to the **field**, and
to express its semantics directly in terms of the README's existing
"replaceable / irreplaceable" parameter classification rather than through a
type-walk-driven dispatch decision.

## Design

### Attribute surface

Two symmetric **field-level** attributes, each taking no arguments:

```rust
#[epserde(force_repl)]    // applies to fields whose type is concrete
#[epserde(force_irrepl)]  // applies to fields whose type is a single-segment generic
```

Both are valid only on fields of a struct or an enum variant. Neither is
valid on the struct/enum item itself, nor on a field of a `zero_copy`
type. The two markers are mutually exclusive on the same field. Each
marker has an additional position restriction: `force_repl` is meaningful
only when the field's type is concrete (it would be redundant on a
single-segment generic field, but is silently accepted as a no-op);
`force_irrepl` is rejected at derive time if the field's type is anything
other than a single-segment generic parameter, since there is no direct
occurrence to reclassify.

### Conceptual semantics, in the README's vocabulary

Today's `README.md` defines:

- A struct generic `T` is **replaceable** iff it appears as the type of a
  field.
- `T` is **irreplaceable** iff it appears as a type parameter of the type of
  a field.
- `T` cannot be both replaceable and irreplaceable.

The new attribute extends those definitions as follows:

- `T` is **replaceable** iff it appears as the direct (single-segment) type
  of some field *that does NOT carry* `#[epserde(force_irrepl)]`, *or* it
  appears anywhere inside the type of some field carrying
  `#[epserde(force_repl)]`.
- `T` is **irreplaceable** iff it appears as a type argument inside the type
  of some field *not* carrying `#[epserde(force_repl)]`, *or* it appears as
  the direct (single-segment) type of some field carrying
  `#[epserde(force_irrepl)]`.
- **Occurrences of `T` nested inside `PhantomData<…>` (at any depth, in any
  field, regardless of whether the field is marked) do not count toward
  either classification.** The derive's existing literal-emission arm makes
  `PhantomData<…>` neutral with respect to parameter substitution — the
  field's slot in `Self::DeserType<'a>` is satisfied by Rust's inference
  regardless of whether the parameter is substituted, so neither pole of
  the classification applies. This preserves today's working patterns
  `struct S<T> { data: T, phantom: PhantomData<T> }` (T direct + inside an
  unmarked PhantomData stays replaceable, no conflict) and `struct
  DataFull<D> { a: usize, b: PhantomData<D> }` (D only inside an unmarked
  PhantomData remains unclassified, no `D: DeserInner` bound).
- `T` cannot be both replaceable and irreplaceable. Conflicts produce a
  derive-time error spanned on (one of) the conflicting fields.

Neither marker changes the nature of any parameter — they change the nature
of the field's *appearance*. `#[epserde(force_repl)]` flips the inside-the-
wrapper appearances from contributing to irreplaceability to contributing
to replaceability. `#[epserde(force_irrepl)]` flips a direct (single-segment
generic) appearance from contributing to replaceability to contributing to
irreplaceability — useful when a parameter is needed as a direct field but
the same parameter also appears as a type argument somewhere else and the
user wants the wrapper occurrence to drive both classifications consistently.

### User contract

`#[epserde(force_repl)]` on a field of type `F<A, B, …>` asserts that

```
for<'a> <F<A, B, …> as DeserInner>::DeserType<'a>
      == F<A::DeserType<'a>, B::DeserType<'a>, …>
```

and the symmetric equation for `SerInner::SerType`. That is, the wrapper
`F` substitutes its own parameters uniformly in its associated
(de)serialization types. Standard library wrappers satisfying the contract
include `Box<T>`, `Rc<T>`, `Arc<T>`, `Option<T>`, the `Range` family,
tuples, arrays for deep-copy `T`, and `Epserde`-derived deep-copy types for
their naturally-replaceable parameters. Wrappers whose `DeserType<'a>`
changes *shape* depending on parameter copy-kind — notably `Vec<T>`,
`Box<[T]>`, `[T; N]`, `String` — satisfy the contract only when the inner
parameter is deep-copy. `PhantomData<T>` is special: its `DeserType<'a>` is
`Self`, but the derive macro emits a literal `::core::marker::PhantomData`
for `PhantomData<…>` fields (existing native-substitution arm), so the
parameter substitution applied by the parent flows through inference and
the field works under `force_repl` without further help.

A violated contract — for instance, marking `Vec<T>` when `T` is
instantiated zero-copy — produces a type mismatch in the derived
`_deser_eps_inner` body. This is unchanged from the existing failure mode
for any contract violation in the derive.

### Effects on the derived code

Let `repl_params` denote the replaceable set computed per the rules above.

1. `Self::DeserType<'a>` substitutes exactly the parameters in
   `repl_params`: `P → <P as DeserInner>::DeserType<'a>`. Irreplaceable and
   unused parameters appear unsubstituted.
2. `Self::SerType` substitutes the same set: `P → <P as SerInner>::SerType`.
3. The where-clause adds `P: DeserInner` (and `P: SerInner` for the ser
   side) only for parameters in `repl_params`. Parameters used solely in
   irreplaceable positions or `PhantomData`-only positions do not get the
   bound, preserving the existing `PhantomData<NotSerializableType>`
   pattern.
4. Per-field dispatch in `gen_eps_deser_method_call` becomes:
   - PhantomDeserData (special method, unchanged).
   - PhantomData (literal emission, unchanged).
   - Field has `#[epserde(force_irrepl)]` → `_deser_full_inner` (the dual
     switch: dispatch flips alongside classification).
   - Field has `#[epserde(force_repl)]` → `_deser_eps_inner`.
   - Field type is a single-segment generic of the struct (and no marker) →
     `_deser_eps_inner`.
   - Otherwise → `_deser_full_inner`.

   No type-walking is required for the dispatch decision itself — it is a
   per-field local syntactic check.

### Conflict diagnostic

Before producing the impl, the derive performs a single pass to classify
each struct generic's occurrences. The pass walks every field's type and,
for each occurrence of a struct generic `P`, records whether the
occurrence is

- the field's exact (single-segment) type and the field is unmarked →
  contributes to *replaceable*;
- the field's exact (single-segment) type and the field is marked with
  `#[epserde(force_irrepl)]` → contributes to *irreplaceable*;
- inside the type of a `#[epserde(force_repl)]`-marked field (and not inside
  a `PhantomData<…>`) → contributes to *replaceable*;
- inside the type of an unmarked field as a type argument (and not inside a
  `PhantomData<…>`) → contributes to *irreplaceable*;
- inside a `PhantomData<…>` at any depth in any field, marked or unmarked →
  contributes to neither, per the exception above.

If any `P` ends up in both buckets, the derive emits a `syn::Error`
spanned on one of the conflicting fields, with form

```
error: type parameter `T` is both replaceable and irreplaceable
note: replaceable: appears as the type of field `<f>` (or inside marked field `<g>`)
note: irreplaceable: appears as a type argument inside unmarked field `<h>`
help: also mark `<h>` with `#[epserde(force_repl)]` if its type substitutes `T` transitively
```

Exact wording is finalised during implementation; the substance is the
explicit naming of the offending fields and the suggested remedy. This
replaces today's opaque Rust issue #152409 projection mismatch that users
otherwise face.

The walk reuses the recursive shape of the existing `type_contains_any`
helper but used purely for *classification*: it does not influence dispatch
and runs once per derive.

### Edge cases for the markers

- `#[epserde(force_repl)]` on a parameterless field is a silent no-op. The
  field is ε-dispatched (its eps-deser yield equals its type unchanged) and
  contributes nothing to `repl_params`.
- `#[epserde(force_repl)]` on a single-segment-generic field is a silent
  no-op. The field would already be ε-dispatched by the natural rule, and
  the parameter's classification (replaceable from the direct occurrence)
  is unchanged by the marker. Accepted to make the macro permissive — users
  applying the marker uniformly should not see spurious errors.
- `#[epserde(force_irrepl)]` is only meaningful on a field whose type is a
  single-segment struct generic. Other field shapes are rejected at derive
  expansion (see Validation).

### Validation

- The `force_repl` attribute may appear only on fields, not on the item;
  takes no arguments; not on fields of `zero_copy` types.
- The `force_irrepl` attribute may appear only on fields whose type is a
  single-segment struct generic; not on the item; takes no arguments; not
  on fields of `zero_copy` types.
- A field carrying both `force_repl` and `force_irrepl` is rejected (the
  two are mutually exclusive).
- Lifetime and const parameters of the struct/enum are unaffected by the
  attributes and the classification pass.

### Removed

- The struct-level `#[epserde(force_repl(T, U, …))]` attribute is removed
  entirely.
- `EpserdeAttrs::force_repl` and `EpserdeContext::force_repl` are removed.
- The unknown-parameter validation in `epserde_derive` is removed.
- The `T: SerInner`/`T: DeserInner`/`T: CopyType` bound injection for
  struct-level forced params is removed.
- The bound-skip in `gen_ser_deser_where_clauses` (the Rust issue #152409
  workaround for struct-level force_repl fields) is removed.
- The `type_contains_any` widening of dispatch in
  `gen_eps_deser_method_call` is reverted; dispatch returns to a per-field
  local syntactic check (single-segment generic OR `force_repl` marker).

### Implementation surface

Localized to `epserde-derive/src/lib.rs`:

1. **Per-field attribute parsing.** Detect `#[epserde(force_repl)]` and
   `#[epserde(force_irrepl)]` on each field. Reject on the item itself;
   reject arguments; reject mutual co-occurrence on the same field; reject
   `force_irrepl` on a field whose type is not a single-segment struct
   generic; reject either marker on a field of a `zero_copy` type.
2. **Classification pass.** One function that, given the parsed struct/enum
   input, returns `(repl_params, conflicts)`. On any conflict, the derive
   short-circuits and emits the diagnostic.
3. **Substitution and bounds.** `gen_generics_for_deser_type`,
   `gen_generics_for_ser_type`, and `bound_ser_deser_types` continue to
   read `repl_params` — fed by the new classifier instead of the old
   "natural ∪ struct-force_repl" union.
4. **Dispatch.** `gen_eps_deser_method_call` becomes the per-field local
   check described above. The `repl_params` argument it currently receives
   is no longer needed for dispatch (only PhantomData/PhantomDeserData
   special cases plus the per-field-attribute and single-segment-param
   triggers).

No new traits, no changes to the runtime crate, no new helpers beyond the
classifier.

## Testing

### Positive tests

Rewrite `epserde/tests/test_force_repl.rs` against the field-level surface:

- **Single-segment param** (today's natural rule, unchanged):
  `struct S<T>(T)` round-trips a `S<Vec<u32>>` to `S<&[u32]>` with no
  attribute.
- **Wrapper case:**
  ```rust
  #[derive(Epserde)] struct A<T>(T);
  #[derive(Epserde)]
  struct S<T> {
      f: T,
      #[epserde(force_repl)]
      g: A<T>,
  }
  ```
  Round-trip `S<Vec<u32>>` and assert both fields land in their ε-form.
- **Mixed direct + marked:** the `struct D` example from the spec's design
  table.
- **Bounded parameter:** marked field with `T: Clone` propagation onto
  `T::DeserType<'_>`.
- **Enum with marked variant fields.**
- **Redundant force_repl on a single-segment-param field** (silent no-op).
- **Parameterless marked field** (silent no-op).
- **Multi-parameter wrapper:** marked `A<T, U>` field; both `T` and `U` end
  up replaceable, no conflict.
- **`force_irrepl` flips a single-segment field to irreplaceable + full-
  dispatch:**
  ```rust
  #[derive(Epserde)]
  struct S<T> {
      #[epserde(force_irrepl)]
      direct: T,
      wrapped: Vec<T>,    // unmarked → T contributes to irreplaceable here
  }
  ```
  T is classified as irreplaceable from both fields (direct via the marker,
  wrapped via the standard rule). No conflict; T is not substituted in
  `Self::DeserType<'a>`. Field `direct` is full-dispatched (returns `T`).
- **`force_irrepl` on a redundant case:** mark a single-segment-generic
  field in a struct where the parameter has no other occurrence. The
  parameter is unused for substitution; `Self::DeserType<'a>` does not
  substitute it; full-dispatch returns `T` unchanged.

### Backward-compatibility positive controls

The existing tests in `epserde/tests/test_phantom.rs` continue to pass
unchanged. In particular:

- `test_not_serializable_in_phantom` (a `D` used only inside an unmarked
  `PhantomData<D>` does not require `D: DeserInner`).
- `test_phantom_data_substitution` (a `T` used as both a direct field and
  inside an unmarked `PhantomData<T>` is replaceable; the `PhantomData`
  exception in classification keeps this from registering as a conflict).

### Compile-fail tests (under `epserde/tests/fail/`)

- **`force_repl_on_zero_copy.rs`**: `force_repl` on a field of a
  `zero_copy` struct/enum.
- **`force_repl_on_item.rs`**: the attribute placed on the item rather than
  a field.
- **`force_repl_with_args.rs`**: the attribute with arguments (e.g.,
  `#[epserde(force_repl(T))]`).
- **`both_repl_and_irrepl.rs`**: a struct where some parameter ends up
  classified as both replaceable and irreplaceable; the recorded `.stderr`
  captures the derive's clean diagnostic.
- **`force_irrepl_on_non_param.rs`**: `force_irrepl` on a field whose type
  is not a single-segment struct generic (e.g., `f: Vec<T>`) — derive-time
  error.
- **`force_irrepl_on_zero_copy.rs`**: `force_irrepl` on a field of a
  `zero_copy` type — derive-time error.
- **`force_repl_and_irrepl_together.rs`**: a field carrying both
  `force_repl` and `force_irrepl` — derive-time error (mutually exclusive).
- **(Removed)** The existing `force_repl_unknown_param.rs` fixture goes
  away — there is no struct-level parameter name to be unknown.

## Documentation

- **`README.md`**: rewrite the `Example: forcing transitive replaceability`
  section against the field-level surface; refresh the "Replaceable and
  irreplaceable parameters" specification chapter to incorporate the
  new classification rule for marked fields; update the spec section on
  what gets substituted in deserialization types.
- **`CLAUDE.md`**: the "Key Invariants" entry on the cannot-be-both
  restriction is rephrased to mention that `#[epserde(force_repl)]` on a
  field changes which occurrences contribute to which class.
- **`epserde-derive/src/lib.rs`**: doc comment on `#[derive(Epserde)]`
  rewritten to describe the field-level attribute and the user contract.
- **Existing struct-level spec/plan docs** at
  `docs/superpowers/specs/2026-05-11-epserde-force-repl-attribute-design.md`
  and
  `docs/superpowers/plans/2026-05-11-epserde-force-repl-attribute.md`:
  left in place as historical record of the abandoned design; the new
  spec/plan supersede them.

## Migration

- Any existing struct-level `#[epserde(force_repl(T))]` use is translated
  by removing the struct-level attribute and adding `#[epserde(force_repl)]`
  to each field whose type contains `T` and which was relying on the
  forced substitution. Mechanical, one-for-one.
- The `force_repl_unknown_param` `trybuild` fixture is deleted; the new
  `both_repl_and_irrepl` fixture replaces it as the canonical "you've
  written something the derive cannot handle" example.

## Failure modes recap

- **Contract violated** (marked field's wrapper does not substitute its
  parameters uniformly): rustc type-mismatch error in the derived
  `_deser_eps_inner` body, naming the slot and the eps-deser return
  type. Identical failure mode to today.
- **Parameter classified as both replaceable and irreplaceable**: the
  derive's new conflict diagnostic, emitted at attribute level before
  the impl is generated.
- **Attribute misused** (wrong position, wrong arguments, zero-copy item):
  derive-time validation error.

## Out of scope

- A trait-based formalization of the "uniform parameter substitution"
  contract (e.g., a `TransRepl<T>` marker trait). Deferred unless
  real-world misuse shows the rustc type-mismatch error to be too opaque
  even with the conflict diagnostic in place.
- Per-parameter forcing inside a marked field (e.g., "mark this field but
  only treat `T` as replaceable, not `U`"). Not supported: marking a field
  reclassifies all parameter occurrences inside its type uniformly. If a
  user needs heterogeneous treatment, they restructure.
- Automatic detection of the wrapper-contract failure at derive time. Not
  feasible without inspecting external impls.
