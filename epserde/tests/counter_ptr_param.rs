/*
 * Regression test for finding #5: `type_contains_any` walk is
 * incomplete relative to Rust's automatic generic substitution.
 *
 * `gen_generics_for_deser_type` substitutes V uniformly in
 * `Self::DeserType<'_>` because that's how Rust's generic
 * substitution works — every occurrence of V in the struct
 * definition is rewritten, including inside `*const V`, `&V`,
 * `fn(V) -> V`, `dyn Trait<V>`, `impl Trait<V>`, etc.
 *
 * `type_contains_any` (`epserde-derive/src/lib.rs:79-115`) only
 * walks Path / Tuple / Array / Slice / Paren / Group. It returns
 * `false` for Ptr / Reference / BareFn / TraitObject / ImplTrait,
 * so a field whose type only contains V in such a position gets
 * dispatched through `_deser_full_inner`. Result: the slot in the
 * struct literal expects `PhantomData<*const V::DeserType<'_>>`
 * but the eps-deser call returns `PhantomData<*const V>`, producing
 * a confusing `?` operator type mismatch.
 *
 * Once `type_contains_any` covers all `syn::Type` variants that can
 * carry a type argument (or the validator rejects forced-repl
 * idents that only appear in unsupported positions), this test must
 * compile and round-trip.
 */

use core::marker::PhantomData;
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy, force_repl(V))]
struct PtrParam<T, V> {
    t: T,
    _v: PhantomData<*const V>,
}

#[test]
fn test_ptr_param() -> anyhow::Result<()> {
    let original: PtrParam<u32, u64> = PtrParam {
        t: 42,
        _v: PhantomData,
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <PtrParam<u32, u64>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let _eps = unsafe { <PtrParam<u32, u64>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
