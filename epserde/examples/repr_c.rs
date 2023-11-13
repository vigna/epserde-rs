/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

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
    // Create a new value to serialize
    let point: Object<Vec<Point>> = Object {
        a: vec![Point { x: 2, y: 1 }; 6],
        test: -0xbadf00d,
    };
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = point.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Object<Vec<Point>>>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Object<Vec<Point>>>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(point, full);

    println!();

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <Object<Vec<Point>>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Object<Vec<Point>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(point.a, eps.a);
    assert_eq!(point.test, eps.test);
}
