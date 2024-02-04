/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use std::io::Cursor;

use bytemuck::*;
/// Example of zero-copy deserialization of an array.
use epserde::prelude::*;

fn main() {
    // Create a vector to serialize

    let a = [1_usize; 100];
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));
    // Serialize
    let _bytes_written = a.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <[usize; 100]>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<[usize; 100]>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization (which will be a zero-copy deserialization)
    let buf = cursor.into_inner();
    let eps = <[usize; 100]>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<[usize; 100] as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
