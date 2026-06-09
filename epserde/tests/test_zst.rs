/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use anyhow::Result;
use core::marker::PhantomData;
use epserde::prelude::*;

#[test]
fn test_vec_unit() -> Result<()> {
    let data = vec![(); 3];
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<()>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, data);
    let eps = unsafe { <Vec<()>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.len(), data.len());
    Ok(())
}

#[test]
fn test_vec_phantom() -> Result<()> {
    let data = vec![PhantomData::<usize>; 3];
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<PhantomData<usize>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, data);
    let eps = unsafe { <Vec<PhantomData<usize>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.len(), data.len());
    Ok(())
}
