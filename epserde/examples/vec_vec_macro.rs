/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(TypeHash, Debug, PartialEq, Eq, Default, Clone)]
/// Create a new type around `Vec<Vec<T>>` because for orphan rule you can't
/// implement `SerializeInner` and the other traits directly.
struct Vec2D<A> {
    data: A,
}

#[derive(Serialize, Deserialize, TypeHash, Debug, PartialEq, Eq, Default, Clone)]
/// Random struct we will use to test the nested serialization and deserialization.
struct Data<A> {
    a: A,
    test: isize,
}

fn main() {
    // create a new value to serialize
    let data = Data {
        a: vec![vec![0x89; 6]; 9],
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
    let mut schema = data.serialize_with_schema(&mut buf).unwrap();
    // sort the schema by offset so we can print it in order
    schema.0.sort_by_key(|a| a.offset);
    let buf = buf.into_inner();
    println!("{}", schema.debug(buf));

    // do a full-copy deserialization
    let data1 = <Data<Vec<Vec<i32>>>>::deserialize_full_copy(&v).unwrap();
    println!("{:02x?}", data1);

    println!("\n");

    // do a zero-copy deserialization
    let data2 = <Data<Vec<Vec<i32>>>>::deserialize_eps_copy(&v).unwrap();
    println!("{:x?}", data2);
}
