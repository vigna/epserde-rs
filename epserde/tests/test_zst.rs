/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
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

/// A zero-sized zero-copy type with alignment stricter than that of any
/// following field.
#[derive(Epserde, Copy, Clone, Debug, PartialEq)]
#[repr(C, align(64))]
#[epserde(zero_copy)]
struct AlignedMarker;

/// Serialization writes alignment padding even for zero-sized types, so
/// ε-copy deserialization must consume it: a field following an aligned
/// zero-sized type behind an ε-copy parameter used to be read from the
/// padding bytes.
#[derive(Epserde, Clone, Debug, PartialEq)]
#[epserde(deep_copy)]
struct FollowedBy<T> {
    zst: T,
    value: u64,
}

macro_rules! test_aligned_zst {
    ($ty:ty, $init:expr) => {{
        let data = FollowedBy::<$ty> {
            zst: $init,
            value: 0xDEAD_BEEF_DEAD_F00D,
        };
        let mut cursor = <AlignedCursor<Aligned64>>::new();
        unsafe { data.serialize(&mut cursor)? };
        cursor.set_position(0);
        let full = unsafe { <FollowedBy<$ty>>::deserialize_full(&mut cursor)? };
        assert_eq!(full.value, data.value);
        let eps = unsafe { <FollowedBy<$ty>>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(eps.value, data.value);
    }};
}

#[test]
fn test_aligned_zst_marker() -> Result<()> {
    test_aligned_zst!(AlignedMarker, AlignedMarker);
    Ok(())
}

#[test]
fn test_aligned_zst_vec_of_empty_arrays() -> Result<()> {
    test_aligned_zst!(Vec<[u128; 0]>, vec![[0u128; 0]; 3]);
    Ok(())
}

#[test]
fn test_aligned_zst_empty_array() -> Result<()> {
    test_aligned_zst!([u128; 0], [0u128; 0]);
    Ok(())
}
