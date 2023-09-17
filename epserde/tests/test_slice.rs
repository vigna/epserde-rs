/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use anyhow::Result;
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data<A: PartialEq = usize, const Q: usize = 3> {
    a: A,
    b: [i32; Q],
}

#[test]
fn test_box_slice_usize() -> Result<()> {
    let a = vec![1, 2, 3, 4];
    let mut cursor = epserde::new_aligned_cursor();
    a.serialize(&mut cursor)?;
    cursor.set_position(0);
    let b = <Vec<i32>>::deserialize_full(&mut cursor)?;
    assert_eq!(a, b.as_slice());
    let backend = cursor.into_inner();
    let b = <Vec<i32>>::deserialize_eps(&backend)?;
    assert_eq!(a, b);
    Ok(())
}
