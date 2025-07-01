/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;

/// Example of an internal parameter of a zero-copy structure,
/// which is left untouched, but needs some decoration to be used.
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A: ZeroCopy> {
    a: Vec<A>,
}

fn main() {
    // Create a new value to serialize
    let data = Data {
        a: vec![0, 1, 2, 3],
    };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let schema = unsafe { data.serialize_with_schema(&mut cursor).unwrap() };

    // Show the schema
    println!("{}", schema.debug(cursor.as_bytes()));

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data<i32>>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Data<i32>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Data<i32> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
