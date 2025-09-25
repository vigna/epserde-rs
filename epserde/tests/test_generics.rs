/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;
use std::marker::PhantomData;
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
    let full = unsafe { <Data<Vec<usize>, 2>>::deser_full(&mut cursor).unwrap() };
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Data<Vec<usize>, 2>>::deser_eps(cursor.as_bytes()).unwrap() };
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
    let full = unsafe { <Data2<usize, Vec<usize>>>::deser_full(&mut cursor).unwrap() };
    assert_eq!(data, full);
    // Do an ε-copy deserialization

    let eps = unsafe { <Data2<usize, Vec<usize>>>::deser_eps(cursor.as_bytes()).unwrap() };
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
    let eps = unsafe { <Data3<12>>::deser_full(&mut cursor) };
    assert!(eps.is_err());

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data3<11>>::deser_full(&mut cursor).unwrap() };
    assert_eq!(data, full);

    // Do an ε-copy deserialization
    let eps = unsafe { <Data3<11>>::deser_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(&data, eps);

    // with a different const the deserialization should fail
    let eps = unsafe { <Data3<12>>::deser_eps(cursor.as_bytes()) };
    assert!(eps.is_err());
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
struct DeepCopyParam<T> {
    data: T,
}

#[test]
fn test_types_deep_copy_param() {
    let _test_usize: <DeepCopyParam<usize> as SerInner>::SerType = DeepCopyParam { data: 0 };
    let _test: <DeepCopyParam<usize> as DeserInner>::DeserType<'_> = DeepCopyParam { data: 0 };
    let _test_array: <DeepCopyParam<[i32; 4]> as SerInner>::SerType =
        DeepCopyParam { data: [1, 2, 3, 4] };
    let _test: <DeepCopyParam<[i32; 4]> as DeserInner>::DeserType<'_> = DeepCopyParam {
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
    let _test_usize: <ZeroCopyParam<usize> as SerInner>::SerType = ZeroCopyParam { data: 0 };
    let _test: <ZeroCopyParam<usize> as DeserInner>::DeserType<'_> = &ZeroCopyParam { data: 0 };
    let _test_array: <ZeroCopyParam<[i32; 4]> as SerInner>::SerType =
        ZeroCopyParam { data: [1, 2, 3, 4] };
    let _test: <ZeroCopyParam<[i32; 4]> as DeserInner>::DeserType<'_> =
        &ZeroCopyParam { data: [1, 2, 3, 4] };
}

// Check that bounds are propagated to associated (de)serialization types.
#[allow(dead_code)]
#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone)]
#[repr(align(8))]
#[repr(align(16))]
enum DeepCopyEnumParam<T: ZeroCopy> {
    A(T),
}
