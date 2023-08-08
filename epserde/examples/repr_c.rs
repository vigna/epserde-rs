/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Serialize, Deserialize, TypeHash, Debug, PartialEq, Eq, Default, Clone)]
struct Object<A> {
    a: A,
    test: isize,
}

#[repr(C)]
#[derive(Serialize, Deserialize, TypeHash, Debug, PartialEq, Eq, Default, Clone)]
struct Point {
    x: usize,
    y: usize,
}

impl ZeroCopy for Point {}

fn main() {
    // create a new value to serialize
    let person0: Object<Vec<Point>> = Object {
        a: vec![Point { x: 2, y: 1 }; 6],
        test: -0xbadf00d,
    };
    // create an aligned vector to serialize into so we can do a zero-copy
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
    // wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // serialize
    let _bytes_written = person0.serialize(&mut buf).unwrap();

    // do a full-copy deserialization
    let person1 = <Object<Vec<Point>>>::deserialize_full_copy(&v).unwrap();
    println!("{:02x?}", person1);
    assert_eq!(person0, person1);

    println!("\n");

    // do a zero-copy deserialization
    let person2 = <Object<Vec<Point>>>::deserialize_eps_copy(&v).unwrap();
    println!("{:x?}", person2);
    assert_eq!(person0.a, person2.a);
    assert_eq!(person0.test, person2.test);
}
