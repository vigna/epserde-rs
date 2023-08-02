/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use std::hash::Hasher;

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
    b: usize,
}

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

fn main() {
    let person0: Person = PersonVec {
        name: vec![0x89; 6],
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

    let mut schema = person0.serialize_with_schema(&mut buf).unwrap();
    schema.0.sort_by_key(|a| a.offset);
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

    println!("\n");

    let person1 = Person::deserialize_zero_copy(&v).unwrap();
    println!("deser_memsize: {}", person1.mem_size());
    println!("deser_type_name: {}", person1.type_name_val());
    person1.mem_dbg().unwrap();
    println!("{:x?}", person1);
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person1.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("deser_type_hash: {:08x}", hash);
}
