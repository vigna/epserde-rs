/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example of zero-copy deserialization of a zero-copy struct.
use epserde::prelude::*;
use maligned::A16;

#[derive(Epserde, Copy, Clone, Debug)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: [usize; 100],
}

fn main() {
    let a = Data { a: [1_usize; 100] };
    let mut cursor = <AlignedCursor<A16>>::new();

    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { Data::deser_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization (which will be a zero-copy deserialization)
    let eps = unsafe { Data::deser_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Data as DeserInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
