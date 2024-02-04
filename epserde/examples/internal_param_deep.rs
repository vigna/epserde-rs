/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example of an internal parameter of a deep-copy structure, which
/// is left untouched, but needs some decoration to be used.
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A: DeepCopy + 'static> {
    a: Vec<A>,
}

fn main() {
    // Create a new value to serialize
    let data = Data {
        a: vec![vec![0x89; 6]; 9],
    };
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize
    let schema = data.serialize_with_schema(&mut cursor).unwrap();

    // Show the schema
    let aligned_buf = cursor.into_inner();
    println!("{}", schema.debug(aligned_buf));
    let mut cursor = std::io::Cursor::new(aligned_buf);

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data<Vec<i32>>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <Data<Vec<i32>>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Data<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
