/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;
use std::hash::Hasher;

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
    // print stats about the value
    println!("mem_size: {}", person0.mem_size());
    println!("type_name: {}", person0.type_name_val());
    person0.mem_dbg().unwrap();
    println!("{:02x?}", person0);

    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person0.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("type_hash: {:08x}", hash);

    println!("");

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
    let mut schema = person0.serialize_with_schema(&mut buf).unwrap();
    // sort the schema by offset so we can print it in order
    schema.0.sort_by_key(|a| a.offset);
    let buf = buf.into_inner();
    println!("{:02x?}\n", &buf);
    println!("{}", schema.debug(buf));

    // do a full-copy deserialization
    let person1 = Person::deserialize(&v).unwrap();
    // print stats about the value
    println!("deser_memsize: {}", person1.mem_size());
    println!("deser_type_name: {}", person1.type_name_val());
    person1.mem_dbg().unwrap();
    println!("{:02x?}", person1);
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person1.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("deser_type_hash: {:08x}", hash);

    println!("\n");

    // do a zero-copy deserialization
    let person1 = Person::deserialize_zero_copy(&v).unwrap();
    // print stats about the value
    println!("deser_memsize: {}", person1.mem_size());
    println!("deser_type_name: {}", person1.type_name_val());
    person1.mem_dbg().unwrap();
    println!("{:x?}", person1);
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person1.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("deser_type_hash: {:08x}", hash);
}
