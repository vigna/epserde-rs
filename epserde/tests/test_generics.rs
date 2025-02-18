/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

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
    let _bytes_written = person.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data<Vec<usize>, 2>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <Data<Vec<usize>, 2>>::deserialize_eps(cursor.as_bytes()).unwrap();
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
    let _bytes_written = data.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data2<usize, Vec<usize>>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(data, full);
    // Do an ε-copy deserialization

    let eps = <Data2<usize, Vec<usize>>>::deserialize_eps(cursor.as_bytes()).unwrap();
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
    let _bytes_written = data.serialize(&mut cursor).unwrap();
    cursor.set_position(0);

    // with a different const the deserialization should fail
    let eps = <Data3<12>>::deserialize_full(&mut cursor);
    assert!(eps.is_err());

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data3<11>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(data, full);

    // Do an ε-copy deserialization
    let eps = <Data3<11>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(&data, eps);

    // with a different const the deserialization should fail
    let eps = <Data3<12>>::deserialize_eps(cursor.as_bytes());
    assert!(eps.is_err());
}
