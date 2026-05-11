/*
 * Counter-test: Option<T>::DeserType IS structurally Option<T::DeserType>,
 * so this wrapper case actually *does* satisfy the contract — verify
 * that enforce_repl(T) works for it.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl(T))]
struct OptionWrapper<T>(Option<T>);

#[test]
fn test_option_wrapper() -> anyhow::Result<()> {
    let original: OptionWrapper<u32> = OptionWrapper(Some(42));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <OptionWrapper<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);
    let _eps =
        unsafe { <OptionWrapper<u32>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
