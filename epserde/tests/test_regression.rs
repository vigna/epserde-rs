/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use epserde::ser::SerType;
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

#[derive(Epserde, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
#[epserde(zero_copy)]
struct MyStructZero {
    a: u32,
    b: u64,
}

#[derive(Epserde, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
#[epserde(zero_copy)]
enum MyEnumZeroFieldless {
    A = 4,
    B = 7,
}

#[allow(dead_code)]
#[derive(Epserde, Debug, Copy, Clone, PartialEq)]
#[repr(C, u8)]
#[epserde(zero_copy)]
enum MyEnumZeroData {
    A(u16) = 1,
    B(u32),
}

// x86_64 / aarch64 regression tests

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
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

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_option() {
    assert_eq!(get_type_hash::<Option<i32>>(), 0x36d9437e00a00833);
    assert_eq!(get_align_hash::<Option<i32>>(), 0x6881f435bc0ca85f);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_string_types() {
    assert_eq!(get_type_hash::<String>(), 0xe4297f5be0f5dd50);
    assert_eq!(get_type_hash::<str>(), 0x393e833de113cd8c);
    assert_eq!(get_type_hash::<Box<str>>(), 0x19aa1d67f7ad7a3e);
    assert_eq!(get_align_hash::<Box<str>>(), 0xd1fba762150c532c);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_array_types() {
    assert_eq!(get_type_hash::<[i32; 5]>(), 0xff020632241e51b0);
    assert_eq!(get_align_hash::<[i32; 5]>(), 0x6881f435bc0ca85f);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_slice_types() {
    assert_eq!(get_type_hash::<[i32]>(), 0xe053d268c8ad5c04);
    assert_eq!(get_type_hash::<SerType<&[i32]>>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<SerType<&[i32]>>(), 0x6881f435bc0ca85f);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_boxed_slice_types() {
    assert_eq!(get_type_hash::<Box<[i32]>>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<Box<[i32]>>(), 0x6881f435bc0ca85f);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_tuple_types() {
    assert_eq!(get_type_hash::<(i32,)>(), 0x4c6eb7a52a31e7b9);
    assert_eq!(get_align_hash::<(i32,)>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<(i32, f64)>(), 0x6c1bf8932e12dc1);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_vec_types() {
    assert_eq!(get_type_hash::<Vec<i32>>(), 0x117f4821c6983671);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_stdlib_types() {
    use core::ops::{
        Bound, ControlFlow, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    };
    #[cfg(feature = "std")]
    use std::collections::hash_map::DefaultHasher;
    #[cfg(feature = "std")]
    assert_eq!(get_type_hash::<DefaultHasher>(), 0x216366ce6df79e86);

    assert_eq!(get_type_hash::<Range<i32>>(), 0x837a1968d53dcff1);
    assert_eq!(get_align_hash::<Range<i32>>(), 0xde0fd80637b3a4da);
    assert_eq!(get_type_hash::<RangeFrom<i32>>(), 0xad8267db843d93b8);
    assert_eq!(get_align_hash::<RangeFrom<i32>>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<RangeInclusive<i32>>(), 0xf90fab627ecbd1a6);
    assert_eq!(get_align_hash::<RangeInclusive<i32>>(), 0xde0fd80637b3a4da);
    assert_eq!(get_type_hash::<RangeTo<i32>>(), 0xd889856367fa2fe3);
    assert_eq!(get_align_hash::<RangeTo<i32>>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<RangeToInclusive<i32>>(), 0xc3682b190d94704d);
    assert_eq!(
        get_align_hash::<RangeToInclusive<i32>>(),
        0x6881f435bc0ca85f
    );
    assert_eq!(get_type_hash::<RangeFull>(), 0x1d5d4cc6e963d594);
    assert_eq!(get_align_hash::<RangeFull>(), 0xd1fba762150c532c);
    assert_eq!(get_type_hash::<Bound<i32>>(), 0x1f77c5db6e0be477);
    assert_eq!(get_align_hash::<Bound<i32>>(), 0x6881f435bc0ca85f);
    assert_eq!(get_type_hash::<ControlFlow<i32, f64>>(), 0x5f4feceae713afe0);
    assert_eq!(
        get_align_hash::<ControlFlow<i32, f64>>(),
        0xc3caaeef7aa4605a
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct() {
    assert_eq!(get_type_hash::<MyStruct>(), 0x129b1d45c6b6ae6c);
    assert_eq!(get_align_hash::<MyStruct>(), 0xc3caaeef7aa4605a);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_generic() {
    assert_eq!(get_type_hash::<MyStructGeneric<i32>>(), 0x1833f5eac633cd47);
    assert_eq!(get_align_hash::<MyStructGeneric<i32>>(), 0x6881f435bc0ca85f);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_enum() {
    assert_eq!(get_type_hash::<MyEnum>(), 0x019604e4828c4317);
    assert_eq!(get_align_hash::<MyEnum>(), 0x7d4ad9f26be56fb9);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_const() {
    assert_eq!(get_type_hash::<MyStructConst<5>>(), 0x97ce1c92e2729d50);
    assert_eq!(get_align_hash::<MyStructConst<5>>(), 0x6881f435bc0ca85f);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_mixed() {
    assert_eq!(get_type_hash::<MyStructMixed<i32, 5>>(), 0x3f0db7304bed06cf);
    assert_eq!(
        get_align_hash::<MyStructMixed<i32, 5>>(),
        0xde0fd80637b3a4da
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_const_then_type() {
    assert_eq!(
        get_type_hash::<MyStructConstThenType<5, i32>>(),
        0x27b6bddd3324eef3
    );
    assert_eq!(
        get_align_hash::<MyStructConstThenType<5, i32>>(),
        0xde0fd80637b3a4da
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_zero() {
    assert_eq!(get_type_hash::<MyStructZero>(), 0x28afb99d6bb0d07e);
    assert_eq!(get_align_hash::<MyStructZero>(), 0x30fc37ff8c51e03d);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_enum_zero_fieldless() {
    assert_eq!(get_type_hash::<MyEnumZeroFieldless>(), 0x37608eb95b95b64f);
    assert_eq!(get_align_hash::<MyEnumZeroFieldless>(), 0xec7c07418292efe6);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_enum_zero_data() {
    assert_eq!(get_type_hash::<MyEnumZeroData>(), 0x9e08224d4df3a446);
    assert_eq!(get_align_hash::<MyEnumZeroData>(), 0x341561197f99ab52);
}

// i686 regression tests

#[cfg(target_arch = "x86")]
#[test]
fn test_primitive_types() {
    assert_eq!(get_type_hash::<isize>(), 0xad77ef2a0c071b87);
    assert_eq!(get_align_hash::<isize>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<i8>(), 0x1bb527fe1af58754);
    assert_eq!(get_align_hash::<i8>(), 0x609ada9fcd0d4297);
    assert_eq!(get_type_hash::<i16>(), 0x568b3e81c4910f1b);
    assert_eq!(get_align_hash::<i16>(), 0xfac4ea0239a7e51f);
    assert_eq!(get_type_hash::<i32>(), 0x19b22886e521147a);
    assert_eq!(get_align_hash::<i32>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<i64>(), 0xba3703df82fb4e98);
    assert_eq!(get_align_hash::<i64>(), 0x5cef13c907be3ad0);
    assert_eq!(get_type_hash::<i128>(), 0x29a957130a3bc847);
    assert_eq!(get_align_hash::<i128>(), 0x612e9a0b5fc8d4f6);
    assert_eq!(get_type_hash::<usize>(), 0xa12462c6d36e68b0);
    assert_eq!(get_align_hash::<usize>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<u8>(), 0xbc9d6eeaea22ffb5);
    assert_eq!(get_align_hash::<u8>(), 0x609ada9fcd0d4297);
    assert_eq!(get_type_hash::<u16>(), 0x704072ef7f3dd44);
    assert_eq!(get_align_hash::<u16>(), 0xfac4ea0239a7e51f);
    assert_eq!(get_type_hash::<u32>(), 0x20aa0c10687491ad);
    assert_eq!(get_align_hash::<u32>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<u64>(), 0xaee7f05a097ffa16);
    assert_eq!(get_align_hash::<u64>(), 0x5cef13c907be3ad0);
    assert_eq!(get_type_hash::<u128>(), 0x19c3bfd795ae2ec8);
    assert_eq!(get_align_hash::<u128>(), 0x612e9a0b5fc8d4f6);
    assert_eq!(get_type_hash::<f32>(), 0xc80e25fc3a1c97d8);
    assert_eq!(get_align_hash::<f32>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<f64>(), 0x7b785833ec3cc6e8);
    assert_eq!(get_align_hash::<f64>(), 0x5cef13c907be3ad0);
    assert_eq!(get_type_hash::<bool>(), 0x947c0c03c59c6f07);
    assert_eq!(get_align_hash::<bool>(), 0x609ada9fcd0d4297);
    assert_eq!(get_type_hash::<char>(), 0x80aa991b46310ff6);
    assert_eq!(get_align_hash::<char>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<()>(), 0x2439715d39cd513);
    assert_eq!(get_align_hash::<()>(), 0xbd60acb658c79e45);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_option() {
    assert_eq!(get_type_hash::<Option<i32>>(), 0x36d9437e00a00833);
    assert_eq!(get_align_hash::<Option<i32>>(), 0x832178dce3dc2030);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_string_types() {
    assert_eq!(get_type_hash::<String>(), 0xe4297f5be0f5dd50);
    assert_eq!(get_type_hash::<str>(), 0x393e833de113cd8c);
    assert_eq!(get_type_hash::<Box<str>>(), 0x19aa1d67f7ad7a3e);
    assert_eq!(get_align_hash::<Box<str>>(), 0xd1fba762150c532c);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_array_types() {
    assert_eq!(get_type_hash::<[i32; 5]>(), 0x269a95634f2c1c51);
    assert_eq!(get_align_hash::<[i32; 5]>(), 0x832178dce3dc2030);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_slice_types() {
    assert_eq!(get_type_hash::<[i32]>(), 0xe053d268c8ad5c04);
    assert_eq!(get_type_hash::<SerType<&[i32]>>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<SerType<&[i32]>>(), 0x832178dce3dc2030);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_boxed_slice_types() {
    assert_eq!(get_type_hash::<Box<[i32]>>(), 0x400f9211e94c1834);
    assert_eq!(get_align_hash::<Box<[i32]>>(), 0x832178dce3dc2030);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_tuple_types() {
    assert_eq!(get_type_hash::<(i32,)>(), 0x4c6eb7a52a31e7b9);
    assert_eq!(get_align_hash::<(i32,)>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<(i32, f64)>(), 0x6c1bf8932e12dc1);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_vec_types() {
    assert_eq!(get_type_hash::<Vec<i32>>(), 0x117f4821c6983671);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_stdlib_types() {
    use core::ops::{
        Bound, ControlFlow, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    };
    #[cfg(feature = "std")]
    use std::collections::hash_map::DefaultHasher;
    #[cfg(feature = "std")]
    assert_eq!(get_type_hash::<DefaultHasher>(), 0x216366ce6df79e86);

    assert_eq!(get_type_hash::<Range<i32>>(), 0x837a1968d53dcff1);
    assert_eq!(get_align_hash::<Range<i32>>(), 0x896839ed01ec9b9);
    assert_eq!(get_type_hash::<RangeFrom<i32>>(), 0xad8267db843d93b8);
    assert_eq!(get_align_hash::<RangeFrom<i32>>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<RangeInclusive<i32>>(), 0xf90fab627ecbd1a6);
    assert_eq!(get_align_hash::<RangeInclusive<i32>>(), 0x896839ed01ec9b9);
    assert_eq!(get_type_hash::<RangeTo<i32>>(), 0xd889856367fa2fe3);
    assert_eq!(get_align_hash::<RangeTo<i32>>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<RangeToInclusive<i32>>(), 0xc3682b190d94704d);
    assert_eq!(
        get_align_hash::<RangeToInclusive<i32>>(),
        0x832178dce3dc2030
    );
    assert_eq!(get_type_hash::<RangeFull>(), 0x1d5d4cc6e963d594);
    assert_eq!(get_align_hash::<RangeFull>(), 0xd1fba762150c532c);
    assert_eq!(get_type_hash::<Bound<i32>>(), 0x1f77c5db6e0be477);
    assert_eq!(get_align_hash::<Bound<i32>>(), 0x832178dce3dc2030);
    assert_eq!(get_type_hash::<ControlFlow<i32, f64>>(), 0x5f4feceae713afe0);
    assert_eq!(
        get_align_hash::<ControlFlow<i32, f64>>(),
        0x7bc9c77917deb867
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct() {
    assert_eq!(get_type_hash::<MyStruct>(), 0x129b1d45c6b6ae6c);
    assert_eq!(get_align_hash::<MyStruct>(), 0x7bc9c77917deb867);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_generic() {
    assert_eq!(get_type_hash::<MyStructGeneric<i32>>(), 0x1833f5eac633cd47);
    assert_eq!(get_align_hash::<MyStructGeneric<i32>>(), 0x832178dce3dc2030);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_enum() {
    assert_eq!(get_type_hash::<MyEnum>(), 0x019604e4828c4317);
    assert_eq!(get_align_hash::<MyEnum>(), 0x8aa3c35a7ab6a4c);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_const() {
    assert_eq!(get_type_hash::<MyStructConst<5>>(), 0xe408cc80da05d9ad);
    assert_eq!(get_align_hash::<MyStructConst<5>>(), 0x832178dce3dc2030);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_mixed() {
    assert_eq!(get_type_hash::<MyStructMixed<i32, 5>>(), 0xba2753d300eb99b);
    assert_eq!(get_align_hash::<MyStructMixed<i32, 5>>(), 0x896839ed01ec9b9);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_const_then_type() {
    assert_eq!(
        get_type_hash::<MyStructConstThenType<5, i32>>(),
        0x40d18efeac8cc96e
    );
    assert_eq!(
        get_align_hash::<MyStructConstThenType<5, i32>>(),
        0x896839ed01ec9b9
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_zero() {
    assert_eq!(get_type_hash::<MyStructZero>(), 0x28afb99d6bb0d07e);
    assert_eq!(get_align_hash::<MyStructZero>(), 0xa4e436092f13e0dc);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_enum_zero_fieldless() {
    assert_eq!(get_type_hash::<MyEnumZeroFieldless>(), 0x37608eb95b95b64f);
    assert_eq!(get_align_hash::<MyEnumZeroFieldless>(), 0x6bebb5a256035706);
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_enum_zero_data() {
    assert_eq!(get_type_hash::<MyEnumZeroData>(), 0x9e08224d4df3a446);
    assert_eq!(get_align_hash::<MyEnumZeroData>(), 0x196b7051405055aa);
}
