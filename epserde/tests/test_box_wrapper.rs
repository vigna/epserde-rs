/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

// Box<T>'s DeserType<'a> is Box<T::DeserType<'a>> (structural substitution),
// so it satisfies the force_repl user contract. This pins the positive-
// control behaviour: a Box<T>-wrapped field marked with force_repl
// round-trips correctly.

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct BoxWrapper<T> {
    #[epserde(force_repl)]
    inner: Box<T>,
}

#[test]
fn test_box_wrapper() -> anyhow::Result<()> {
    let original: BoxWrapper<Vec<u32>> = BoxWrapper { inner: Box::new(vec![1, 2, 3]) };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <BoxWrapper<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <BoxWrapper<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner_slice: &[u32] = *eps.inner;
    assert_eq!([1u32, 2, 3].as_slice(), inner_slice);

    Ok(())
}
