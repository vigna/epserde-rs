/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*
 * This example shows how the standard behavior of ε-serde on primitive
 * types (returning a value rather than a reference) is somewhat custom:
 * if we derive code for a zero-copy newtype containing just a `usize`,
 * the associated deserialization type is a reference.
 */
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
#[repr(C)]
#[zero_copy]
struct USize {
    value: usize,
}

fn main() {
    // Create a new value to serialize
    let x = USize { value: 0 };
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = x.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <USize>::deserialize_full_copy(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<USize>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(x, full);

    println!();

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = <USize>::deserialize_eps_copy(&buf).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<USize as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(x, *eps);
}
