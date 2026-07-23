/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;
#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[repr(align(32))]
#[repr(align(64))] // The max wins
#[epserde(zero_copy)]
struct MyStruct64 {
    u: u32,
}

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[repr(align(2))]
#[epserde(zero_copy)]
struct MyStruct2 {
    u: u32,
}

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[epserde(zero_copy)]
struct MyStruct {
    u: u32,
}

#[test]
/// Check that pad_to reflects the repr(align) attributes and that the
/// resulting types round trip
fn test_pad_to() -> anyhow::Result<()> {
    assert_eq!(MyStruct64::pad_to(), 64);
    assert_eq!(MyStruct2::pad_to(), MyStruct::pad_to());

    let x = MyStruct { u: 0x89 };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor)? };

    // Do an ε-copy deserialization
    let eps = unsafe { <MyStruct>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, x);

    // Create a new value to serialize
    let x = MyStruct2 { u: 0x89 };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor)? };

    // Do an ε-copy deserialization
    let eps = unsafe { <MyStruct2>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, x);

    // Create a new value to serialize
    let x = MyStruct64 { u: 0x89 };
    // We need a higher alignment
    let mut cursor = <AlignedCursor<Aligned64>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor)? };

    // Do an ε-copy deserialization
    let eps = unsafe { <MyStruct64>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, x);
    Ok(())
}
