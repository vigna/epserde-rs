/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::{A16, A64};
#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[repr(align(32))]
#[repr(align(64))] // The max wins
#[epserde_zero_copy]
struct MyStruct64 {
    u: u32,
}

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[repr(align(2))]
#[epserde_zero_copy]
struct MyStruct2 {
    u: u32,
}

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[epserde_zero_copy]
struct MyStruct {
    u: u32,
}

#[test]
/// Check that we don't have any collision on most types
fn test_align_to() {
    assert_eq!(64, MyStruct64::align_to());
    assert_eq!(MyStruct::align_to(), MyStruct2::align_to());

    let x = MyStruct { u: 0x89 };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor).unwrap() };

    // Do an ε-copy deserialization
    let eps = unsafe { <MyStruct>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(x, *eps);

    // Create a new value to serialize
    let x = MyStruct2 { u: 0x89 };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor).unwrap() };

    // Do an ε-copy deserialization
    let eps = unsafe { <MyStruct2>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(x, *eps);

    // Create a new value to serialize
    let x = MyStruct64 { u: 0x89 };
    // We need a higher alignment
    let mut cursor = <AlignedCursor<A64>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor).unwrap() };

    // Do an ε-copy deserialization
    let eps = unsafe { <MyStruct64>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(x, *eps);
}
