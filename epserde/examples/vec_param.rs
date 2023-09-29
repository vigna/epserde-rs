/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
/// Example of an internal parameter, which is left untouched,
/// but needs some decoration to be used.
struct Data<A: DeepCopy + 'static> {
    a: Vec<A>,
}

fn main() {
    // Create a new value to serialize
    let data = Data {
        a: vec![vec![0x89; 6]; 9],
    };
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let schema = data.serialize_with_schema(&mut buf).unwrap();
    println!("{}", schema.debug(&buf.clone().into_inner()));

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Data<Vec<i32>>>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <Data<Vec<i32>>>::deserialize_eps(&buf).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<Data<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
