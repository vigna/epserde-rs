# `#[epserde(enforce_repl(...))]`: forcing transitive replaceability

## Motivation

ε-serde's derive macro classifies a generic type parameter `T` of a struct/enum
as "replaceable" only when `T` appears as the exact type of one of its fields.
Replaceable parameters are substituted with `T::DeserType<'a>` (and
`SerType<T>`) in `Self::DeserType<'a>` and `Self::SerType`, and the
corresponding fields are deserialized through `_deser_eps_inner` rather than
`_deser_full_inner`.

This rule misses a common shape:

```rust
#[derive(Epserde)]
struct A<T>(T);

#[derive(Epserde)]
struct B<T>(A<Vec<T>>);
```

In `B`, `T` never appears as a direct field type — the field type is
`A<Vec<T>>` — so under the current rule `T` is non-replaceable in `B`. The
result is that `B<Vec<usize>>::DeserType<'a> = B<Vec<usize>>`, even though
intuitively a user serializing a `B<Vec<usize>>` would expect the ε-copy
deserialized form to be `B<&'a [usize]>`, since both `A<U>` and `Vec<U>`
already substitute their parameter transitively.

The same blind spot is what motivates the existing invariant from `CLAUDE.md`
that "a replaceable type parameter must not appear both as a field type and as
a parameter of another field type": that combination would produce a
`Self::DeserType<'a>` whose two T-slots are inconsistent (one substituted, one
not), so the derive forbids it implicitly via syntactic detection.

This spec proposes a struct/enum-level attribute that lets the user assert
transitive replaceability explicitly, lifting both restrictions.

## Design

### Attribute

A new arm of the existing `#[epserde(...)]` attribute:

```rust
#[derive(Epserde)]
#[epserde(enforce_repl(T))]
struct B<T>(A<Vec<T>>);
```

- Takes one or more comma-separated type-parameter idents.
- Each ident must name a type parameter of the annotated item.
- Idempotent on parameters that are already naturally replaceable.
- Allowed on structs (named and tuple) and enums.
- Rejected on `zero_copy` types (their `DeserType<'a> = &'a Self`, no
  substitution).
- Lifetime and const parameters are not accepted.

### Semantics

`enforce_repl(T)` asserts that every field type in which `T` appears
substitutes `T` transitively in its own `DeserType<'a>` and `SerType`. That is,
for every field type `F<…T…>` of the annotated item:

```
for<'a> <F<…T…> as DeserInner>::DeserType<'a> == F<…T::DeserType<'a>…>
<F<…T…> as SerInner>::SerType         == F<…SerType<T>…>
```

This is a contract on the user. The derive does not verify it; the failure
mode is a compile error in the generated `_deser_eps_inner` body (the eps-deser
call returns the wrong shape and Rust's type checker rejects the struct
literal).

Stdlib impls already satisfy the contract for their parameters (`Vec<T>`,
`Box<T>`, `Option<T>`, `Range<T>`, tuples, arrays, `Rc<T>`, `Arc<T>`). User
types derived with `Epserde` satisfy it for their naturally-replaceable
parameters and for any parameter they themselves declare with `enforce_repl`.

### Effects on the derived code

Let `repl_params := natural ∪ enforce_repl_idents`.

1. `Self::DeserType<'a>` substitutes every ident in `repl_params` with
   `<that ident as DeserInner>::DeserType<'a>`. (Already handled by
   `gen_generics_for_deser_type`, which reads `repl_params`.)
2. `Self::SerType` substitutes every ident in `repl_params` with `SerType<that
   ident>`. (Already handled by `gen_generics_for_ser_type`.)
3. The bound-propagation in `bound_ser_deser_types` emits the trait bounds of
   every replaceable parameter onto its substituted form. (Already correct;
   reads `repl_params`.)
4. Field dispatch in `gen_eps_deser_method_call` chooses `_deser_eps_inner`
   for any field whose type *contains* a parameter in `repl_params`, instead
   of the current "is exactly a single-segment replaceable param" check.
   Fields whose type contains no replaceable parameter continue to use
   `_deser_full_inner`.

The "appears both as field type and as parameter of another field type"
invariant is naturally lifted: if T is naturally replaceable *and* listed in
`enforce_repl`, both syntactic positions resolve to the same substituted form
because dispatch now consults type-containment, not exact equality.

### Type-containment walk

A helper introduced in `epserde-derive/src/lib.rs`:

```rust
fn type_contains_any(ty: &syn::Type, params: &HashSet<&syn::Ident>) -> bool
```

Recurses on the variants of `syn::Type` that epserde actually supports:

- `Type::Path` — for each segment, return `true` if the segment ident is in
  `params`; otherwise recurse into `PathArguments::AngleBracketed` type
  arguments.
- `Type::Tuple` — recurse on each element.
- `Type::Array`, `Type::Slice`, `Type::Paren`, `Type::Group` — recurse on the
  inner type.
- All other `syn::Type` variants return `false`.

Lifetime and const generic arguments are ignored.

### Validation

Performed at derive-expansion time:

- `enforce_repl(X)` where X is not a declared generic type parameter →
  compile error spanned on X.
- `enforce_repl(...)` on a `zero_copy` type → compile error.
- `enforce_repl` with a lifetime or const ident → compile error.
- A duplicate ident inside one `enforce_repl(...)` list is silently deduped.
- Listing a naturally-replaceable parameter in `enforce_repl(...)` is allowed
  (no-op).

### Implementation surface

Localized to `epserde-derive/src/lib.rs`:

1. `EpserdeAttrs`: add `enforce_repl: Vec<syn::Ident>`.
2. `parse_epserde_attrs`: parse `enforce_repl(...)` as a new arm in the
   nested-meta walk.
3. `EpserdeContext` (or its construction): after the existing
   `get_type_const_params` call, validate the parsed `enforce_repl` list
   against the declared generics; emit errors per the validation rules above.
4. `gen_epserde_struct_impl` and `gen_epserde_enum_impl`: after the existing
   natural field-scan that populates `repl_params`, union the validated
   `enforce_repl` idents.
5. New helper `type_contains_any` (free function in the same file).
6. `gen_eps_deser_method_call`: replace the existing single-segment check
   with a `type_contains_any(field_type, repl_params)` check. Preserve the
   `PhantomDeserData` special case at the top of the function.

No changes to:

- The runtime crate `epserde`.
- `TypeHash`, `AlignHash`, `AlignTo` derive paths (they don't consult
  replaceability).
- Existing behavior in the absence of `enforce_repl(...)`.

## Testing

Add tests under `epserde/tests/`:

- **Wrapper case**: `struct B<T>(A<Vec<T>>)` with `#[epserde(enforce_repl(T))]`,
  round-tripped with `T = u32` (inner is zero-copy) and `T = Vec<u8>` (inner
  is deep-copy). Assertions on the ε-copy deserialized form's type.
- **Mixed-position case**: a struct where T appears both as a direct field and
  as a parameter of another field's type, demonstrating that
  `enforce_repl(T)` lifts the old invariant.
- **Bounded parameter**: `struct C<T: Clone>(...)` with `enforce_repl(T)`,
  ensuring bound propagation onto `T::DeserType<'_>` and `SerType<T>` still
  works.
- **Enum**: an enum with at least one variant whose field type contains a
  forced-repl parameter.
- **Idempotency**: a struct where T is already naturally replaceable, also
  listed in `enforce_repl(T)`; the resulting derived code is observably the
  same as without the attribute.

Add `trybuild` compile-fail cases under `epserde/tests/` (the existing `fail/`
pattern):

- `enforce_repl(X)` where X is not a generic parameter.
- `enforce_repl(...)` on a `zero_copy` struct.
- Contract violation: a struct using `enforce_repl(T)` whose field is a
  user-defined wrapper that does not substitute T transitively. (The compile
  error should point at the derived `_deser_eps_inner` body.)
- `enforce_repl` naming a lifetime or const parameter.

## Documentation

- Doc comment on `#[derive(Epserde)]` in `epserde-derive/src/lib.rs` lists
  `enforce_repl(...)` alongside `zero_copy`, `deep_copy`, and `bound(...)`.
- Prose section in `epserde/src/lib.rs` near the existing `PhantomDeserData`
  documentation, explaining the contract and showing the motivating
  wrapper-through-Vec example.
- `CLAUDE.md` "Key Invariants" entry on the "appears both as field type and as
  parameter of another field type" rule is updated to note that
  `#[epserde(enforce_repl(...))]` lifts the restriction.

## Out of scope

- Field-level marker attributes. Considered and rejected during brainstorming:
  because `Self::DeserType<'a>` substitutes a parameter uniformly across all
  fields, per-field dispatch is mechanically determined by the struct-level
  declaration, and a field-level marker only adds opportunities to forget one.
- Compile-time enforcement of the transitive-substitution contract via a
  marker trait (`TransRepl<T>` or similar). Deferred unless misuse in practice
  shows that the type-mismatch error message is too opaque.
- Automatic detection of transitively-replaceable parameters. Not feasible at
  derive time without inspecting other types' impls.

## Failure modes

A user who declares `enforce_repl(T)` but has a field type `Foo<T>` whose own
`DeserType<'a>` does not substitute T transitively produces, in the generated
`_deser_eps_inner`:

```rust
Ok(Name {
    f: unsafe { <Foo<T> as DeserInner>::_deser_eps_inner(backend)? },
})
```

which evaluates to `Foo<T>`, while `Self::DeserType<'a>` expects
`Foo<T::DeserType<'a>>` in that slot. The Rust type checker rejects the struct
literal at derive expansion. No silent miscompilation.
