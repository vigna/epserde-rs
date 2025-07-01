/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use anyhow::Result;
use epserde::prelude::*;
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data<A: PartialEq = usize, const Q: usize = 3> {
    a: A,
    b: [i32; Q],
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct B<C: PartialEq> {
    c: C,
}

#[test]
fn test_boxed_slices() -> Result<()> {
    let a = vec![1, 2, 3, 4].into_boxed_slice();
    let mut cursor = <AlignedCursor<A16>>::new();
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let b = unsafe { Box::<[i32]>::deserialize_full(&mut cursor)? };
    assert_eq!(a, b);
    let b = unsafe { Box::<[i32]>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(b, a.as_ref());

    cursor.set_position(0);
    let d = Data {
        a: vec![0, 1, 2, 3].into_boxed_slice(),
        b: [1, 2, 3],
    };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Box<[i32]>>::deserialize_full(&mut cursor)? };
    assert_eq!(e, d);

    cursor.set_position(0);
    let d = Data { a, b: [1, 2, 3] };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Box<[i32]>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(e.a, d.a.as_ref());
    Ok(())
}
