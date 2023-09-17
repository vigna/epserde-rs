/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

fn main() {
    // Create a vector to serialize

    let a = Some(vec![0, 1, 2, 3]);
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Option<Vec<i32>>>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Option<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <Option<Vec<i32>>>::deserialize_eps(&buf).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<Option<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);

    let mut buf = epserde::new_aligned_cursor();

    println!("\n");

    // Serialize
    let a: Option<Vec<i32>> = None;
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Option<Vec<i32>>>::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Option<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <Option<Vec<i32>>>::deserialize_eps(&buf).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<Option<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
