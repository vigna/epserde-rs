/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use epserde::prelude::*;
use maligned::{AsBytesMut, A16};
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
    let mut aligned_buf = vec![A16::default(); 1024];
    let mut cursor = std::io::Cursor::new(aligned_buf.as_bytes_mut());

    // Serialize
    let _bytes_written = person.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data<Vec<usize>, 2>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let bytes = cursor.into_inner();
    let eps = <Data<Vec<usize>, 2>>::deserialize_eps(&bytes).unwrap();
    assert_eq!(person.a, eps.a);
    assert_eq!(person.b, eps.b);
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data2<A, B> {
    a: B,
    _marker: std::marker::PhantomData<A>,
}

#[test]
fn test_inner_param_eps() {
    // Create a new value to serialize
    let data = Data2::<usize, Vec<usize>> {
        a: vec![0x89; 6],
        _marker: PhantomData,
    };

    let mut aligned_buf = vec![A16::default(); 1024];
    let mut cursor = std::io::Cursor::new(aligned_buf.as_bytes_mut());

    // Serialize
    let _bytes_written = data.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data2<usize, Vec<usize>>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(data, full);
    // Do an ε-copy deserialization
    cursor.set_position(0);
    let bytes = cursor.into_inner();
    let eps = <Data2<usize, Vec<usize>>>::deserialize_eps(&bytes).unwrap();
    assert_eq!(data.a, eps.a);
}
