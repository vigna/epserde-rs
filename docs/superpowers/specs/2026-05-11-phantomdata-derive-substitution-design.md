# Native `PhantomData<T>` substitution in the `Epserde` derive

## Motivation

`PhantomData<T>` from `core::marker` does not substitute its parameter in its
ε-serde `DeserType<'a>`: its impl in `epserde/src/impls/prim.rs:289` defines
`type DeserType<'a> = Self`. This is deliberate, since the `impl<T: ?Sized>
DeserInner for PhantomData<T>` deliberately drops any bound on `T` so that
users can place non-serializable marker types inside `PhantomData`.

The downstream consequence is that a struct with a generic parameter `T`
appearing *both* as a direct field and inside a `PhantomData<T>` fails to
compile under `#[derive(Epserde)]`:

```rust
struct Data<T> {
    data: T,                  // makes T naturally replaceable
    phantom: PhantomData<T>,  // but PhantomData<T>::DeserType<'_> = PhantomData<T>
}
```

`Data<T>::DeserType<'_>` substitutes `T` everywhere, so the slot for `phantom`
becomes `PhantomData<T::DeserType<'_>>`, while the field-level ε-deser of
`PhantomData<T>` returns `PhantomData<T>`. The mismatch is a compile error in
the derived `_deser_eps_inner` body.

`PhantomDeserData<T>` exists exclusively to work around this, by carrying its
own substituting `DeserType<'a> = PhantomDeserData<T::DeserType<'a>>` and a
zero-size transmute inside `_deser_eps_inner_special`.

The new `enforce_repl` attribute introduces *additional* cases where this same
mismatch arises (a `PhantomData<T>` field inside a struct that opts a parameter
into substitution), so the workaround surface grows rather than shrinks.

This spec proposes handling `PhantomData<...>` natively in the derive macro,
removing the need for `PhantomDeserData` in all common cases. The latter is
kept (deprecated) so existing user code continues to compile.

## Design

### Mechanism

In `gen_eps_deser_method_call` (`epserde-derive/src/lib.rs:77`), insert one
new branch immediately after the existing `PhantomDeserData` special case.
When the field type is a `syn::Type::Path` whose last segment's identifier is
exactly `PhantomData`, emit:

```rust
#field_name: ::core::marker::PhantomData
```

with no method call, no transmute, no substitution computation.

The surrounding struct literal in the derived `_deser_eps_inner` body is
constructing `Self::DeserType<'a>`. Rust infers the generic parameter of the
emitted `PhantomData` literal from the corresponding slot in
`Self::DeserType<'a>`, which is the substituted form of the original field
type. The construction therefore yields exactly the expected type with no
explicit substitution work in the derive.

Worked examples (all with `#[derive(Epserde)]`):

- `struct S<T> { data: T, p: PhantomData<T> }`: `Self::DeserType<'a> =
  S<T::DeserType<'a>>`; the slot for `p` is `PhantomData<T::DeserType<'a>>`;
  the emitted literal infers to that.
- `struct S<T> { p: PhantomData<()> }`: slot is `PhantomData<()>`; literal
  infers to `PhantomData<()>`.
- `struct S<T> { p: PhantomData<T> }` with `#[epserde(enforce_repl(T))]`:
  slot is `PhantomData<T::DeserType<'a>>`; literal infers correctly.
- `struct S<T> { data: Vec<T>, p: PhantomData<Vec<T>> }` with
  `enforce_repl(T)`: slot is `PhantomData<Vec<T::DeserType<'a>>>`; literal
  infers correctly.

### Detection

Same pattern as the existing `PhantomDeserData` arm at
`epserde-derive/src/lib.rs:92-95`: match `syn::Type::Path`, inspect the last
segment's identifier, compare against the string `"PhantomData"`. This catches
the bare `PhantomData<...>`, `core::marker::PhantomData<...>`, and
`std::marker::PhantomData<...>` forms uniformly. The check is intentionally
syntactic and weak — a user who defines their own `PhantomData` type would
collide, exactly as for the existing `PhantomDeserData` check.

The new branch fires for *any* inner type (replaceable or not), since the
literal-emission is always correct: when no substitution would occur, the
inferred type still equals the original `PhantomData<...>`.

### Full-copy path

`_deser_full_inner` returns `Self`, not `Self::DeserType<'a>`, so the existing
behaviour — calling `<PhantomData<T>>::_deser_full_inner`, which yields
`PhantomData<T>` — already produces the correct shape. No change is required
in the full-copy path. Optimising it to also emit the literal is left out:
the existing call is correct and matches the pattern used for every other
non-zero-copy field type.

### Zero-copy interaction

Zero-copy types do not substitute parameters; their `DeserType<'a> = &'a Self`
and ε-deserialization reads raw bytes. The proposed change is unrelated to
the zero-copy code path and does not touch it.

### Implementation surface

Localised to:

1. **`epserde-derive/src/lib.rs`** — one new arm in `gen_eps_deser_method_call`,
   placed immediately after the existing `PhantomDeserData` arm.
2. **`epserde/src/lib.rs`** — add `#[deprecated]` to `PhantomDeserData`, with
   a message that points users to plain `PhantomData<T>` and calls out the
   wire-format implication of migrating.
3. **`epserde/README.md`** (via the symlink at `epserde/README.md` →
   `../README.md`) — replace the `compile_fail` example that currently
   motivates `PhantomDeserData` with a working example using plain
   `PhantomData<T>`. Add a short note pointing out that `PhantomDeserData` is
   deprecated and that changing an existing struct from `PhantomDeserData<T>`
   to `PhantomData<T>` is a wire-format change.

No new traits, no new helpers, no validation surface.

### Backward compatibility

`PhantomDeserData<T>` remains in the public API as a deprecated item. Existing
user code that uses it continues to compile and to round-trip exactly the same
serialised data as before. The `#[deprecated]` warning surfaces at use sites
so users can migrate at their pace.

### Wire-format implication of migrating

`PhantomDeserData<T>::type_hash` hashes the string `"PhantomDeserData"`
(`epserde/src/lib.rs:184-189`); `PhantomData<T>::type_hash` hashes
`"PhantomData"`. A user migrating a struct from `PhantomDeserData<T>` to
`PhantomData<T>` changes the struct's overall type hash, which means existing
serialised files produced before the migration will fail the type-hash check
when deserialised against the new struct definition. This is documented in
the deprecation message and in the README.

### Failure modes

The literal `::core::marker::PhantomData` is constructed unconditionally and
relies on Rust's type-inference inside the surrounding struct literal to
choose its generic parameter. If — for some pathological reason — the
inference fails (e.g. the substituted slot is itself an unresolved associated
type the compiler cannot project), the failure surfaces as a clear compile
error inside the derived code. There is no silent miscompilation.

## Testing

Append two new positive tests to `epserde/tests/test_phantom.rs`:

1. **Plain-PhantomData substitution** — define `struct Data<T> { data: T,
   phantom: PhantomData<T> }`, round-trip a `Data<Vec<u32>>`, and assert that
   the ε-deserialized form's `data` field is `&[u32]` (the natural ε-form of
   `Vec<u32>`) and that `phantom` has type `PhantomData<&[u32]>`.
2. **PhantomData substitution under `enforce_repl`** — define `struct
   OnlyPhantom<T> { other: u32, phantom: PhantomData<T> }` with
   `#[epserde(enforce_repl(T))]`, round-trip `OnlyPhantom::<Vec<u32>>`, and
   assert that the ε-deserialized form's `phantom` has type
   `PhantomData<&[u32]>`.

The existing tests that exercise `PhantomDeserData` (`test_deser_phantom_deep_copy`,
`test_deser_phantom_zero_copy`) keep passing unchanged, demonstrating that
`PhantomDeserData` remains functional through deprecation.

No `trybuild` compile-fail tests are added — there is no new user-facing
validation rule.

## Out of scope

- Removing `PhantomDeserData`. The type stays in the public API behind a
  `#[deprecated]` attribute. A future major-version bump could remove it; this
  spec does not.
- Generalising the special case to other zero-sized marker types. The
  detection is intentionally limited to `PhantomData` because it is the one
  stdlib marker that ε-serde already accepts as a non-substituting field.
- Changing `PhantomData<T>`'s own `DeserInner` impl in `prim.rs`. The impl
  remains `DeserType<'a> = Self`; the substituting behaviour is achieved
  entirely by the derive emitting a literal in the surrounding struct.
- Handling `PhantomData<...>` nested deep inside another type (e.g. a field
  of type `(u32, PhantomData<T>)`). Such fields go through the normal
  per-field dispatch and rely on the inner type's `DeserInner` impl; the
  proposed change targets only top-level `PhantomData<...>` field types.
