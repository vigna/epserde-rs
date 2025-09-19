/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

fn get_type_hash<T: TypeHash + ?Sized>() -> u64 {
    let mut hasher = DefaultHasher::new();
    T::type_hash(&mut hasher);
    hasher.finish()
}

fn get_align_hash<T: AlignHash + ?Sized>() -> u64 {
    let mut hasher = DefaultHasher::new();
    let mut offset = 0;
    T::align_hash(&mut hasher, &mut offset);
    hasher.finish()
}

#[test]
fn test_primitive_types() {
    assert_eq!(get_type_hash::<isize>(), 0xad77ef2a0c071b87);
    assert_eq!(get_align_hash::<isize>(), 0xd3eed631c35c21cf);
    assert_eq!(get_type_hash::<i8>(), 0x1bb527fe1af58754);
    assert_eq!(get_align_hash::<i8>(), 0x7359aa1156ce877a);
    assert_eq!(get_type_hash::<i16>(), 0x568b3e81c4910f1b);
    assert_eq!(get_align_hash::<i16>(), 0xeaf7d87e9d1ee4bc);
    assert_eq!(get_type_hash::<i32>(), 0x19b22886e521147a);
    assert_eq!(get_align_hash::<i32>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<i64>(), 0xba3703df82fb4e98);
    assert_eq!(get_align_hash::<i64>(), 0xd3eed631c35c21cf);
    assert_eq!(get_type_hash::<i128>(), 0x29a957130a3bc847);
    assert_eq!(get_align_hash::<i128>(), 0x6c9b3167d412086c);
    assert_eq!(get_type_hash::<usize>(), 0xa12462c6d36e68b0);
    assert_eq!(get_align_hash::<usize>(), 0xd3eed631c35c21cf);
    assert_eq!(get_type_hash::<u8>(), 0xbc9d6eeaea22ffb5);
    assert_eq!(get_align_hash::<u8>(), 0x7359aa1156ce877a);
    assert_eq!(get_type_hash::<u16>(), 0x704072ef7f3dd44);
    assert_eq!(get_align_hash::<u16>(), 0xeaf7d87e9d1ee4bc);
    assert_eq!(get_type_hash::<u32>(), 0x20aa0c10687491ad);
    assert_eq!(get_align_hash::<u32>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<u64>(), 0xaee7f05a097ffa16);
    assert_eq!(get_align_hash::<u64>(), 0xd3eed631c35c21cf);
    assert_eq!(get_type_hash::<u128>(), 0x19c3bfd795ae2ec8);
    assert_eq!(get_align_hash::<u128>(), 0x6c9b3167d412086c);
    assert_eq!(get_type_hash::<f32>(), 0xc80e25fc3a1c97d8);
    assert_eq!(get_align_hash::<f32>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<f64>(), 0x7b785833ec3cc6e8);
    assert_eq!(get_align_hash::<f64>(), 0xd3eed631c35c21cf);
    assert_eq!(get_type_hash::<bool>(), 0x947c0c03c59c6f07);
    assert_eq!(get_align_hash::<bool>(), 0x7359aa1156ce877a);
    assert_eq!(get_type_hash::<char>(), 0x80aa991b46310ff6);
    assert_eq!(get_align_hash::<char>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<()>(), 0x2439715d39cd513);
    assert_eq!(get_align_hash::<()>(), 0x76be999e3e25b2a0);
}

#[test]
fn test_option() {
    assert_eq!(get_type_hash::<Option<i32>>(), 0x36d9437e00a00833);
    assert_eq!(get_align_hash::<Option<i32>>(), 0x6881f435bc0ca85f);
}

#[test]
fn test_string_types() {
    assert_eq!(get_type_hash::<String>(), 0xe4297f5be0f5dd50);
    assert_eq!(get_align_hash::<String>(), 0xd1fba762150c532c);
    assert_eq!(get_type_hash::<Box<str>>(), 0x19aa1d67f7ad7a3e);
    assert_eq!(get_align_hash::<Box<str>>(), 0xd1fba762150c532c);
    assert_eq!(get_type_hash::<str>(), 0x393e833de113cd8c);
}

#[test]
fn test_array_types() {
    assert_eq!(get_type_hash::<[i32; 5]>(), 0xff020632241e51b0);
    assert_eq!(get_align_hash::<[i32; 5]>(), 0x6881f435bc0ca85f);
}

#[test]
fn test_slice_types() {
    assert_eq!(get_type_hash::<&[i32]>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<&[i32]>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<[i32]>(), 0xe053d268c8ad5c04);
}

#[test]
fn test_boxed_slice_types() {
    assert_eq!(get_type_hash::<Box<[i32]>>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<Box<[i32]>>(), 0x6881f435bc0ca85f);
}

#[test]
fn test_tuple_types() {
    assert_eq!(get_type_hash::<(i32,)>(), 0x4c6eb7a52a31e7b9);
    assert_eq!(get_align_hash::<(i32,)>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<(i32, f64)>(), 0x6c1bf8932e12dc1);
}

#[test]
fn test_vec_types() {
    assert_eq!(get_type_hash::<Vec<i32>>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<Vec<i32>>(), 0x6881f435bc0ca85f);
}

#[test]
fn test_stdlib_types() {
    use core::ops::{
        Bound, ControlFlow, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    };
    use std::collections::hash_map::DefaultHasher;
    assert_eq!(get_type_hash::<DefaultHasher>(), 0x216366ce6df79e86);
    assert_eq!(get_type_hash::<Range<i32>>(), 0x837a1968d53dcff1);
    assert_eq!(get_align_hash::<Range<i32>>(), 0xde0fd80637b3a4da);
    assert_eq!(get_type_hash::<RangeFrom<i32>>(), 0xad8267db843d93b8);
    assert_eq!(get_align_hash::<RangeFrom<i32>>(), 0xde0fd80637b3a4da);
    assert_eq!(get_type_hash::<RangeInclusive<i32>>(), 0xf90fab627ecbd1a6);
    assert_eq!(get_align_hash::<RangeInclusive<i32>>(), 0xde0fd80637b3a4da);
    assert_eq!(get_type_hash::<RangeTo<i32>>(), 0xd889856367fa2fe3);
    assert_eq!(get_align_hash::<RangeTo<i32>>(), 0xde0fd80637b3a4da);
    assert_eq!(get_type_hash::<RangeToInclusive<i32>>(), 0xc3682b190d94704d);
    assert_eq!(
        get_align_hash::<RangeToInclusive<i32>>(),
        0xde0fd80637b3a4da
    );
    assert_eq!(get_type_hash::<RangeFull>(), 0x1d5d4cc6e963d594);
    assert_eq!(get_align_hash::<RangeFull>(), 0xd1fba762150c532c);
    assert_eq!(get_type_hash::<Bound<i32>>(), 0x1f77c5db6e0be477);
    assert_eq!(get_align_hash::<Bound<i32>>(), 0xd1fba762150c532c);
    assert_eq!(get_type_hash::<ControlFlow<i32, f64>>(), 0x5f4feceae713afe0);
    assert_eq!(
        get_align_hash::<ControlFlow<i32, f64>>(),
        0xc3caaeef7aa4605a
    );
}

#[derive(Epserde, Debug, PartialEq)]
struct MyStruct {
    a: i32,
    b: f64,
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructGeneric<T: PartialEq> {
    a: T,
}

#[derive(Epserde, Debug, PartialEq)]
enum MyEnum {
    A,
    B(i32),
    C { a: i32, b: f64 },
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructConst<const N: usize> {
    a: [i32; N],
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructMixed<T: PartialEq, const N: usize> {
    a: T,
    b: [i32; N],
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructConstThenType<const N: usize, T: PartialEq> {
    a: [i32; N],
    b: T,
}

#[test]
fn test_derive_struct() {
    assert_eq!(get_type_hash::<MyStruct>(), 0x65125c7b120befff);
    assert_eq!(get_align_hash::<MyStruct>(), 0xc3caaeef7aa4605a);
}

#[test]
fn test_derive_struct_generic() {
    assert_eq!(get_type_hash::<MyStructGeneric<i32>>(), 0x6dced006dd1acb8f);
    assert_eq!(get_align_hash::<MyStructGeneric<i32>>(), 0x6881f435bc0ca85f);
}

#[test]
fn test_derive_enum() {
    assert_eq!(get_type_hash::<MyEnum>(), 0xf5e19aa69f2d9fac);
    assert_eq!(get_align_hash::<MyEnum>(), 0x7c4ea1189a62724c);
}

#[test]
fn test_derive_struct_const() {
    assert_eq!(get_type_hash::<MyStructConst<5>>(), 0x87c97042d431cbf7);
    assert_eq!(get_align_hash::<MyStructConst<5>>(), 0x6881f435bc0ca85f);
}

#[test]
fn test_derive_struct_mixed() {
    assert_eq!(get_type_hash::<MyStructMixed<i32, 5>>(), 0xa8a943379dbe6ea7);
    assert_eq!(
        get_align_hash::<MyStructMixed<i32, 5>>(),
        0xde0fd80637b3a4da
    );
}

#[test]
fn test_derive_struct_const_then_type() {
    assert_eq!(
        get_type_hash::<MyStructConstThenType<5, i32>>(),
        0xba025cd70e024ad5
    );
    assert_eq!(
        get_align_hash::<MyStructConstThenType<5, i32>>(),
        0xde0fd80637b3a4da
    );
}
