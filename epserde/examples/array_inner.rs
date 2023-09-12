/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Epserde, Debug)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: [usize; 100],
}

fn main() {
    // Create a vector to serialize

    let a = Data { a: [1_usize; 100] };
    let mut buf = new_aligned_cursor();
    // Serialize
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = Data::deserialize_full_copy(&mut buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Data>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do a ε-copy deserialization (which will be a zero-copy deserialization)
    let buf = buf.into_inner();
    let eps = Data::deserialize_eps_copy(&buf).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<Data as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
