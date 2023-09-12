/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

fn main() {
    // Create a vector to serialize

    let a = [1_usize; 100];
    let mut buf = new_aligned_cursor();
    // Serialize
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <[usize; 100]>::deserialize_full_copy(&mut buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<[usize; 100]>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do a ε-copy deserialization (which will be a zero-copy deserialization)
    let buf = buf.into_inner();
    let eps = <[usize; 100]>::deserialize_eps_copy(&buf).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<[usize; 100] as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
