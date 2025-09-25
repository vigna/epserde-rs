/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showing how the standard behavior of ε-serde on primitive
/// types (returning a value rather than a reference) is somewhat custom:
/// the deserialization type associated to a one-element tuple containing
/// just a `usize` is a reference.
use epserde::prelude::*;
use maligned::A16;

fn main() {
    let x = (0_usize,);
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { x.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <(usize,)>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<(usize,)>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(x, full);

    println!();

    // Do an ε-copy deserialization (which will be zero-copy deserialization)
    let eps = unsafe { <(usize,)>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<(usize,) as DeserInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(x, *eps);
}
