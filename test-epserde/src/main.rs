/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use std::hash::Hasher;

use epserde_derive::*;
use epserde_trait::*;

#[derive(Serialize, Deserialize, MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
struct PersonVec<A, B> {
    name: A,
    age: B,
    test: isize,
}

#[derive(Serialize, Deserialize, MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
struct Data {
    a: Vec<u16>,
    b: usize,
}

type Person = PersonVec<usize, Data>;

fn main() {
    let person0 = Person {
        name: 10,
        test: -0xbadf00d,
        age: Data {
            a: vec![0x42; 7],
            b: 0xffaaaaaaaaaaaaff,
        },
    };
    println!("mem_size: {}", person0.mem_size());
    println!("type_name: {}", person0.type_name_val());
    person0.mem_dbg().unwrap();
    println!("{:02x?}", person0);

    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person0.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("type_hash: {:08x}", hash);

    println!("");
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    let mut buf = std::io::Cursor::new(&mut v);

    let schema = person0.serialize_with_schema(&mut buf).unwrap();
    let buf = buf.into_inner();
    println!("{:02x?}", &buf);
    println!("{}", schema.debug(buf));

    let person1 = Person::deserialize(&v).unwrap();
    println!("deser_memsize: {}", person1.mem_size());
    println!("deser_type_name: {}", person1.type_name_val());
    person1.mem_dbg().unwrap();
    println!("{:02x?}", person1);
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person1.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("deser_type_hash: {:08x}", hash);

    let slice = v.as_slice();

    let person1 = <Person>::deserialize_zero_copy(slice).unwrap();
    println!("deser_memsize: {}", person1.mem_size());
    println!("deser_type_name: {}", person1.type_name_val());
    person1.mem_dbg().unwrap();
    println!("{:x?}", person1);
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person1.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("deser_type_hash: {:08x}", hash);
}
