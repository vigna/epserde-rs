/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Serialize, Deserialize, MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
struct PersonVec<A, B> {
    a: A,
    b: B,
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
        a: vec![0x89; 6],
        b: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
    // create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let mut file = std::fs::File::create("test.bin").unwrap();
    // serialize
    let _bytes_written = person0.serialize(&mut file).unwrap();

    drop(file);

    let file = std::fs::read("test.bin").unwrap();
    println!("{:02x?}", file);
    // do a full-copy deserialization
    let person1 = Person::deserialize_full_copy(&file).unwrap();
    println!("{:02x?}", person1);

    println!("\n");

    // do a zero-copy deserialization
    let person1 = Person::deserialize_eps_copy(&file).unwrap();
    println!("{:x?}", person1);
}
