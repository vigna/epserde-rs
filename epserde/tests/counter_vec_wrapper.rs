/*
 * Regression test for finding #2: `Vec<T>` as a direct wrapper with
 * `enforce_repl(T)`.
 *
 * Per the design spec line 74, "Stdlib impls already satisfy the
 * contract for their parameters (`Vec<T>`, `Box<T>`, `Option<T>`, …)".
 * If that claim were true, this would compile and round-trip.
 *
 * It does not. `Vec<T>::DeserType<'a>` is conditional on `T`'s copy
 * kind (see `impls/vec.rs:62`):
 *   - `&'a [T]`              when T: ZeroCopy
 *   - `Vec<T::DeserType<'a>>` when T: DeepCopy
 *
 * So `Vec<T>` only satisfies the spec's contract
 * `<F<T>>::DeserType == F<T::DeserType>` in the DeepCopy branch.
 *
 * Once the implementation supports this (or the spec is narrowed to
 * exclude Vec from the "stdlib satisfies the contract" list), this
 * test must either compile and round-trip or be deleted alongside
 * the spec change.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct VecWrapper<T>(Vec<T>);

#[test]
fn test_vec_wrapper() -> anyhow::Result<()> {
    let original = VecWrapper::<u32>(vec![1, 2, 3, 4]);
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <VecWrapper<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let _eps = unsafe { <VecWrapper<u32>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
