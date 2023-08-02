/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;
use std::hash::Hasher;

#[derive(MemSize, TypeName)]
struct PersonVec<A, B> {
    name: A,
    age: B,
    test: isize,
}

#[derive(MemSize, TypeName)]
struct Data<A> {
    a: A,
    b: Vec<i32>,
}

fn main() {
    // create a new value to serialize
    let person = PersonVec {
        name: vec![0x89; 6],
        test: -0xbadf00d,
        age: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
    };

    // get the type name of the value
    println!("type_name: {}", person.type_name_val());

    // print the size in bytes of the value
    println!("mem_size: {}", person.mem_size());

    // print the tree of fields and their memory size
    person.mem_dbg().unwrap();

    // compute the hash of the type using your own custom hasher
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    person.type_hash_val(&mut hasher);
    let hash = hasher.finish();
    println!("type_hash: {:08x}", hash);
}
