/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Serialize, Deserialize, MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
struct PersonVec<A, B> {
    name: A,
    age: B,
    test: isize,
}

#[derive(Serialize, Deserialize, MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
    /// This is an inner field, so IT WILL NOT BE ZERO-COPIED
    b: Vec<i32>,
}

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

fn main() {
    // create a new value to serialize
    let person0: Person = PersonVec {
        name: vec![0x89; 6],
        test: -0xbadf00d,
        age: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
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
    let person1 = Person::deserialize(&v).unwrap();
    println!("{:02x?}", person1);

    println!("\n");

    // do a zero-copy deserialization
    let person1 = Person::deserialize_zero_copy(&v).unwrap();
    println!("{:x?}", person1);
}
