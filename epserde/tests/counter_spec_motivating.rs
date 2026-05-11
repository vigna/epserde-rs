/*
 * Regression test for finding #1: the design spec's own canonical
 * motivating example.
 *
 * Per `docs/superpowers/specs/2026-05-11-epserde-enforce-repl-attribute-design.md`
 * lines 14-27 and 44-48, this exact shape is supposed to be the
 * driving use case of `enforce_repl`:
 *
 *     struct A<T>(T);
 *     #[epserde(enforce_repl(T))]
 *     struct B<T>(A<Vec<T>>);
 *
 * Currently fails to compile: the bound-skip workaround for Rust
 * issue #152409 in `gen_ser_deser_where_clauses` removes the
 * `A<Vec<T>>: SerInner/DeserInner` bound, but the only replacement
 * bounds added for `T` are `T: SerInner` and `T: DeserInner`. To
 * resolve `Vec<T>: DeserInner` Rust needs at minimum `T: CopyType`,
 * which is never propagated.
 *
 * Once the implementation is fixed (or the spec narrowed), this
 * test must compile and round-trip cleanly.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct A<T>(T);

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct SpecMotivating<T>(A<Vec<T>>);

#[test]
fn test_spec_motivating() -> anyhow::Result<()> {
    let original = SpecMotivating::<u32>(A(vec![1u32, 2, 3, 4]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <SpecMotivating<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let _eps = unsafe { <SpecMotivating<u32>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
