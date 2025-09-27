/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example of zero-copy deserialization of a zero-copy struct.
//!
//! Please compile with the "schema" feature to see the schema output.

use epserde::{prelude::*, ser::SerType};
use maligned::A16;

#[derive(Epserde, Copy, Clone, Debug)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: [i32; 10],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<Data>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Data>>()
    );
    println!();

    let data = Data { a: [1; 10] };
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
    let full = unsafe { Data::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Data>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization (which will be a zero-copy deserialization)
    let eps = unsafe { Data::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Data>>(),
    );
    println!("Value: {:x?}", eps);

    #[cfg(not(feature = "schema"))]
    println!("\nPlease compile with the \"schema\" feature to see the schema output");
    Ok(())
}
