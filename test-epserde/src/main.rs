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
    a: Vec<usize>,
    b: usize,
}

type Person = PersonVec<Vec<u8>, Data>;

fn main() {
    let person0 = PersonVec {
        name: vec![10_usize; 10],
        test: -0xbadf00d,
        age: Data {
            a: vec![0; 10],
            b: 0,
        },
    };
    println!("mem_size: {}", person0.mem_size());
    println!("type_name: {}", person0.type_name_val());
    person0.mem_dbg().unwrap();
    println!("{:?}", person0);

    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person0.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("type_hash: {:08x}", hash);

    println!("");
    let mut v = vec![0; 100];
    let mut buf = std::io::Cursor::new(&mut v);
    let schema = person0.serialize_with_schema(&mut buf).unwrap();
    println!("{:x?}", &buf);
    println!("{}", schema.to_csv());

    let person1 = <Person as DeserializeZeroCopy<false>>::deserialize_zero_copy(&v).unwrap();

    println!("deser_memsize: {}", person1.mem_size());
    println!("deser_type_name: {}", person1.type_name_val());
    person1.mem_dbg().unwrap();
    println!("{:?}", person1);
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person1.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("deser_type_hash: {:08x}", hash);

    println!("{}", <Vec<usize>>::Des);
}
