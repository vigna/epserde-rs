/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showing that vectors of a zero-copy type are ε-copy
/// deserialized to a reference.
use epserde::prelude::*;
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Object<A> {
    a: A,
    test: isize,
}

#[repr(C)]
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone, Copy)]
// We want to use zero-copy deserialization on Point,
// and thus ε-copy deserialization on Vec<Point>, etc.
#[zero_copy]
struct Point {
    x: usize,
    y: usize,
}

fn main() {
    let point: Object<Vec<Point>> = Object {
        a: vec![Point { x: 2, y: 1 }; 6],
        test: -0xbadf00d,
    };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { point.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Object<Vec<Point>>>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        core::any::type_name::<Object<Vec<Point>>>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(point, full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Object<Vec<Point>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        core::any::type_name::<DeserType<'_, Object<Vec<Point>>>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(point.a, eps.a);
    assert_eq!(point.test, eps.test);
}
