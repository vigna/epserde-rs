/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use anyhow::Result;
use epserde::prelude::*;

#[test]
fn test_slices() -> Result<()> {
    let v = vec![0, 1, 2];
    let s = v.as_slice();
    let t = vec![s, s, s];
    let mut cursor = AlignedCursor::<Aligned16>::new();
    unsafe { t.serialize(&mut cursor) }?;

    // References are erased in the serialization type, so the header hash
    // accepts owned deserialization types.
    cursor.set_position(0);
    let full = unsafe { <Vec<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, vec![vec![0, 1, 2]; 3]);

    cursor.set_position(0);
    let full = unsafe { <Vec<Box<[i32]>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, vec![vec![0, 1, 2].into_boxed_slice(); 3]);
    Ok(())
}

#[test]
fn test_ref_str() -> Result<()> {
    let t = vec!["a", "b", "c"];
    let mut cursor = AlignedCursor::<Aligned16>::new();
    unsafe { t.serialize(&mut cursor) }?;

    // References are erased in the serialization type, so the header hash
    // accepts the owned deserialization type.
    cursor.set_position(0);
    let full = unsafe { <Vec<String>>::deserialize_full(&mut cursor)? };
    assert_eq!(
        full,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    Ok(())
}
