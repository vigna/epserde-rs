/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showcasing serialization of an `Option``.
use epserde::prelude::*;

fn main() {
    let a = Some(vec![0, 1, 2, 3]);
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize
    let _bytes_written = a.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Option<Vec<i32>>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Option<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <Option<Vec<i32>>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Option<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);

    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    println!("\n");

    // Serialize
    let a: Option<Vec<i32>> = None;
    let _bytes_written = a.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Option<Vec<i32>>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Option<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <Option<Vec<i32>>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Option<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
