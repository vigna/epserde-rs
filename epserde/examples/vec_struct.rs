/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showing that vectors of zero-copy types are
/// ε-copy deserialized as references to slices.
use epserde::prelude::*;
use maligned::{AsBytesMut, A16};

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone, Copy)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: usize,
}

fn main() {
    let a = vec![Data { a: 5 }, Data { a: 6 }];
    let mut aligned_buf = vec![A16::default(); 1024];
    let mut cursor = std::io::Cursor::new(aligned_buf.as_bytes_mut());

    // Serialize
    let _bytes_written = a.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Vec<Data>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Vec<Data>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = <Vec<Data>>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Vec<Data> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
