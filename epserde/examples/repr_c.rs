/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Object<A> {
    a: A,
    test: isize,
}

#[repr(C)]
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
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
    // Create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // Wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = point.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Object<Vec<Point>>>::deserialize_full_copy(buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Object<Vec<Point>>>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(point, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <Object<Vec<Point>>>::deserialize_eps_copy(&v).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<Object<Vec<Point>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(point.a, eps.a);
    assert_eq!(point.test, eps.test);
}
