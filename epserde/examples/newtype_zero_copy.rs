/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showing how the standard behavior of ε-serde on primitive
/// types (returning a value rather than a reference) is somewhat custom:
/// if we derive code for a zero-copy newtype containing just a `usize`,
/// the associated deserialization type is a reference.
use epserde::prelude::*;
use maligned::{AsBytesMut, A16};

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Default, Clone)]
#[repr(C)]
#[zero_copy]
struct USize(usize);

fn main() {
    let x = USize(0);
    let mut aligned_buf = vec![A16::default(); 1024];
    let mut cursor = std::io::Cursor::new(aligned_buf.as_bytes_mut());

    // Serialize
    let _bytes_written = x.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <USize>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<USize>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(x, full);

    println!();

    // Do an ε-copy deserialization (which will be zero-copy deserialization)
    let buf = cursor.into_inner();
    let eps = <USize>::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<USize as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(x, *eps);
}
