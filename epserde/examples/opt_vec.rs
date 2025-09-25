/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showcasing serialization of an `Option``.
use epserde::prelude::*;
use maligned::A16;

fn main() {
    let a = Some(vec![0, 1, 2, 3]);
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Option<Vec<i32>>>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Option<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Option<Vec<i32>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Option<Vec<i32>> as DeserInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);

    let mut cursor = <AlignedCursor<A16>>::new();
    println!("\n");

    // Serialize
    let a: Option<Vec<i32>> = None;
    let _bytes_written = unsafe { a.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Option<Vec<i32>>>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Option<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Option<Vec<i32>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Option<Vec<i32>> as DeserInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
