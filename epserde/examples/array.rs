/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;

/// Example of zero-copy deserialization of an array.
fn main() {
    // Create a vector to serialize

    let a = [1_usize; 100];
    let mut cursor = <AlignedCursor<A16>>::new(); // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <[usize; 100]>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        core::any::type_name::<[usize; 100]>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization (which will be a zero-copy deserialization)
    let eps = unsafe { <[usize; 100]>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        core::any::type_name::<DeserType<'_, [usize; 100]>>(),
    );
    println!("Value: {:x?}", eps);
}
