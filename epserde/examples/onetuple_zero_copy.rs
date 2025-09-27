/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example showing how the standard behavior of ε-serde on primitive types
//! (returning a value rather than a reference) is somewhat custom: the
//! deserialization type associated to a one-element tuple containing just a
//! `usize` is a reference.
//!
//! Please compile with the "schema" feature to see the schema output.

use epserde::{deser::DeserType, prelude::*, ser::SerType};
use maligned::A16;

type Type = (usize,);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<Type>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Type>>()
    );
    println!();

    let data = (0_usize,);
    let mut cursor = <AlignedCursor<A16>>::new();

    // Serialize
    #[cfg(feature = "schema")]
    {
        let schema = unsafe { data.serialize_with_schema(&mut cursor)? };
        println!("{}", schema.debug(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { data.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Type>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Type>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(data, full);

    println!();

    // Do an ε-copy deserialization (which will be zero-copy deserialization)
    let eps = unsafe { <Type>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Type>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(data, *eps);

    #[cfg(not(feature = "schema"))]
    println!("\nPlease compile with the \"schema\" feature to see the schema output");
    Ok(())
}
