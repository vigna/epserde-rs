/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
/// Random struct we will use to test the nested serialization and deserialization.
struct Data<A> {
    a: A,
    test: isize,
}

fn main() {
    // Create a new value to serialize
    let data = Data {
        a: vec![vec![0x89; 6]; 9],
        test: -0xbadf00d,
    };

    // Create an aligned vector to serialize into so we can do an ε-copy
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

    // Serialize
    let mut schema = data.serialize_with_schema(&mut buf).unwrap();
    // Sort the schema by offset so we can print it in order
    schema.0.sort_by_key(|a| a.offset);
    let buf = buf.into_inner();
    println!("{}", schema.debug(buf));

    // Do a full-copy deserialization
    let buf = std::io::Cursor::new(&mut v);
    let full = <Data<Vec<Vec<i32>>>>::deserialize_full_copy(buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Data<Vec<Vec<i32>>>>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization
    let eps = <Data<Vec<Vec<i32>>>>::deserialize_eps_copy(&v).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<Data<Vec<Vec<i32>>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
