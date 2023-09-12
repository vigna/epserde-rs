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
    let mut buf = new_aligned_cursor();
    // Serialize
    let schema = data.serialize_with_schema(&mut buf).unwrap();
    println!("{}", schema.debug(&buf.clone().into_inner()));

    // Do a full-copy deserialization
    let full = <Data<Vec<Vec<i32>>>>::deserialize_full_copy(&mut buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Data<Vec<Vec<i32>>>>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <Data<Vec<Vec<i32>>>>::deserialize_eps_copy(&buf).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<Data<Vec<Vec<i32>>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
