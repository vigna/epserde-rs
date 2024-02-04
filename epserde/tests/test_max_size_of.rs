/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use epserde::prelude::*;

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[repr(align(32))]
#[repr(align(64))]
#[zero_copy]
struct MyStruct64 {
    u: u32,
}

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[repr(align(2))]
#[zero_copy]
struct MyStruct2 {
    u: u32,
}

#[derive(Epserde, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[zero_copy]
struct MyStruct {
    u: u32,
}

#[test]
/// Check that we don't have any collision on most types
fn test_max_size_of_align() {
    assert_eq!(64, MyStruct64::max_size_of());
    assert_eq!(MyStruct::max_size_of(), MyStruct2::max_size_of());

    let x = MyStruct { u: 0x89 };
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize
    let _bytes_written = x.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <MyStruct>::deserialize_eps(&buf).unwrap();
    assert_eq!(x, *eps);

    // Create a new value to serialize
    let x = MyStruct2 { u: 0x89 };
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize
    let _bytes_written = x.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <MyStruct2>::deserialize_eps(&buf).unwrap();
    assert_eq!(x, *eps);

    // Create a new value to serialize
    let x = MyStruct64 { u: 0x89 };
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize
    let _bytes_written = x.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <MyStruct64>::deserialize_eps(&buf).unwrap();
    assert_eq!(x, *eps);
}
