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

#[test]
fn test_slices() -> Result<()> {
    let a = vec![1, 2, 3, 4];
    let s = a.as_slice();
    let mut cursor = <AlignedCursor<A16>>::new();
    s.serialize(&mut cursor)?;
    cursor.set_position(0);
    let b = <Vec<i32>>::deserialize_full(&mut cursor)?;
    assert_eq!(a, b.as_slice());
    let b = <Vec<i32>>::deserialize_eps(cursor.as_bytes())?;
    assert_eq!(a, b);

    cursor.set_position(0);
    let d = Data {
        a: vec![0, 1, 2, 3].into_boxed_slice(),
        b: [1, 2, 3],
    };
    d.serialize(&mut cursor)?;
    cursor.set_position(0);
    let e = Data::<Box<[i32]>>::deserialize_full(&mut cursor)?;
    assert_eq!(e, d);

    cursor.set_position(0);
    let d = Data { a: s, b: [1, 2, 3] };
    d.serialize(&mut cursor)?;
    cursor.set_position(0);
    let e = Data::<Vec<i32>>::deserialize_eps(cursor.as_bytes())?;
    assert_eq!(e, d);

    Ok(())
}
