/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;
use std::{hash::Hash, marker::PhantomData};
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data<A: PartialEq = usize, const Q: usize = 3> {
    a: A,
    b: [i32; Q],
}

#[test]
fn test_inner_param_full() {
    // Create a new value to serialize
    let person = Data {
        a: vec![0x89; 6],
        b: [0xbadf00d; 2],
    };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { person.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data<Vec<usize>, 2>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Data<Vec<usize>, 2>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(person.a, eps.a);
    assert_eq!(person.b, eps.b);
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data2<P, B> {
    a: B,
    // this should be ignored, but contains `P` in the type name so it might
    // be erroneously matched
    _marker2: PhantomData<()>,
    _marker: std::marker::PhantomData<P>,
}

#[test]
fn test_inner_param_eps() {
    // Create a new value to serialize
    let data = Data2::<usize, Vec<usize>> {
        a: vec![0x89; 6],
        _marker2: PhantomData,
        _marker: PhantomData,
    };

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { data.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data2<usize, Vec<usize>>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(data, full);
    // Do an ε-copy deserialization

    let eps = unsafe { <Data2<usize, Vec<usize>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(data.a, eps.a);
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Copy)]
#[zero_copy]
#[repr(C)]
struct Data3<const N: usize = 10>;

#[test]
fn test_consts() {
    // Create a new value to serialize
    let data = Data3::<11> {};

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { data.serialize(&mut cursor).unwrap() };
    cursor.set_position(0);

    // with a different const the deserialization should fail
    let eps = unsafe { <Data3<12>>::deserialize_full(&mut cursor) };
    assert!(eps.is_err());

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data3<11>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(data, full);

    // Do an ε-copy deserialization
    let eps = unsafe { <Data3<11>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(&data, eps);

    // with a different const the deserialization should fail
    let eps = unsafe { <Data3<12>>::deserialize_eps(cursor.as_bytes()) };
    assert!(eps.is_err());
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
struct DeepCopyParam<T> {
    data: T,
}

#[test]
fn test_types_deep_copy_param() {
    let _test_usize: <DeepCopyParam<usize> as SerializeInner>::SerType = DeepCopyParam { data: 0 };
    let _test: <DeepCopyParam<usize> as DeserializeInner>::DeserType<'_> =
        DeepCopyParam { data: 0 };
    let _test_array: <DeepCopyParam<[i32; 4]> as SerializeInner>::SerType =
        DeepCopyParam { data: [1, 2, 3, 4] };
    let _test: <DeepCopyParam<[i32; 4]> as DeserializeInner>::DeserType<'_> = DeepCopyParam {
        data: &[1, 2, 3, 4],
    };
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[zero_copy]
struct ZeroCopyParam<T: ZeroCopy> {
    data: T,
}

#[test]
fn test_types_zero_copy_param() {
    let _test_usize: <ZeroCopyParam<usize> as SerializeInner>::SerType = ZeroCopyParam { data: 0 };
    let _test: <ZeroCopyParam<usize> as DeserializeInner>::DeserType<'_> =
        &ZeroCopyParam { data: 0 };
    let _test_array: <ZeroCopyParam<[i32; 4]> as SerializeInner>::SerType =
        ZeroCopyParam { data: [1, 2, 3, 4] };
    let _test: <ZeroCopyParam<[i32; 4]> as DeserializeInner>::DeserType<'_> =
        &ZeroCopyParam { data: [1, 2, 3, 4] };
}

// Check that bounds are propagated to associated (de)serialization types.
#[allow(dead_code)]
#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone)]
enum DeepCopyEnumParam<T: ZeroCopy> {
    A(T),
}

#[derive(Copy, Debug, PartialEq, Eq, Clone)]
struct NewStr(&'static str);

impl TypeHash for NewStr {
    fn type_hash(mut _hasher: &mut impl core::hash::Hasher) {}
}

impl AlignHash for NewStr {
    fn align_hash(mut _hasher: &mut impl core::hash::Hasher, _offset: &mut usize) {}
}

impl MaxSizeOf for NewStr {
    fn max_size_of() -> usize {
        0
    }
}

impl CopyType for NewStr {
    type Copy = Zero;
}

impl DeserializeInner for NewStr {
    type DeserType<'a>
        = &'a str
    where
        Self: 'a;

    unsafe fn _deserialize_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        todo!();
    }

    unsafe fn _deserialize_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        todo!()
    }
}

impl SerializeInner for NewStr {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;
    type SerType = Self;

    unsafe fn _serialize_inner(&self, _backend: &mut impl ser::WriteWithNames) -> ser::Result<()> {
        todo!()
    }
}

#[derive(Epserde, Copy, Clone)]
#[repr(C)]
#[zero_copy]
struct S {
    a: NewStr,
}
