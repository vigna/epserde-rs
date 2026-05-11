/*
 * Regression test for finding #4: `PhantomData<T>` field with
 * `enforce_repl(T)`.
 *
 * `PhantomData<T>` is the one stdlib generic wrapper that does NOT
 * substitute its parameter:
 *
 *     impl<T: ?Sized> DeserInner for PhantomData<T> {
 *         type DeserType<'a> = Self;
 *         ...
 *     }
 *     (`impls/prim.rs:289-302`)
 *
 * This is the deliberate reason `PhantomDeserData<T>` exists. The
 * `enforce_repl` spec/docs never mention PhantomData; the derive
 * doc-comment on `#[derive(Epserde)]`
 * (`epserde-derive/src/lib.rs:1158-1159`) says
 *
 *     "standard library wrappers […] and Epserde-derived types
 *      satisfy [the contract] automatically"
 *
 * which a user will read as covering PhantomData. It doesn't, and
 * the failure mode is a confusing `?`-operator type mismatch in the
 * generated `_deser_eps_inner` body.
 *
 * Once the implementation either special-cases PhantomData at
 * derive time (suggesting `PhantomDeserData<T>`) or rejects the
 * combination with a clear error, this test must either compile and
 * round-trip (if the fix replaces PhantomData with PhantomDeserData
 * silently) or be moved into `fail/` with a deliberate error spec.
 */

use core::marker::PhantomData;
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy, force_repl(T))]
struct PhantomDataWrapper<T>(u32, PhantomData<T>);

#[test]
fn test_phantom_data() -> anyhow::Result<()> {
    let original = PhantomDataWrapper::<u32>(7, PhantomData);
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <PhantomDataWrapper<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let _eps = unsafe { <PhantomDataWrapper<u32>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
