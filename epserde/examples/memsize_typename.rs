/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;
use std::hash::Hasher;

#[derive(MemSize, MemDbg, TypeName)]
struct PersonVec<A, B> {
    a: A,
    b: B,
    test: isize,
}

#[derive(MemSize, MemDbg, TypeName)]
struct Data<A> {
    a: A,
    b: Vec<i32>,
}

fn main() {
    // create a new value to serialize
    let person = PersonVec {
        a: vec![0x89; 600],
        b: Data {
            a: vec![0x42; 700],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
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
