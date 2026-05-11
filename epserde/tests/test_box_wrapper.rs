/*
 * Investigates whether enforce_repl(T) on a Box<T> wrapper round-trips.
 * Box<T> compiles but the eps deser path may surface the type-shape
 * mismatch at runtime.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy)]
#[epserde(force_repl(T))]
struct BoxWrapper<T>(Box<T>);

#[test]
fn test_box_wrapper_eps() -> anyhow::Result<()> {
    let original: BoxWrapper<u32> = BoxWrapper(Box::new(42));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <BoxWrapper<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);
    let _eps = unsafe { <BoxWrapper<u32>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
