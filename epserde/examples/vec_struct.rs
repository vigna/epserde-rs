/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example showing that vectors of zero-copy types are
//! ε-copy deserialized as references to slices.
//!
//! Please compile with the "schema" feature to see the schema output.

use epserde::{deser::DeserType, prelude::*, ser::SerType};
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone, Copy)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<Vec<Data>>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Vec<Data>>>()
    );
    println!();

    let data = vec![Data { a: 5 }, Data { a: 6 }];
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

    // Do a full-copy deserialization as a vector
    cursor.set_position(0);
    let full = unsafe { <Vec<Data>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Vec<Data>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do a full-copy deserialization as a boxed slice
    cursor.set_position(0);
    let full = unsafe { <Box<[Data]>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Box<[Data]>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Vec<Data>>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Vec<Data>>>(),
    );
    println!("Value: {:x?}", eps);

    #[cfg(not(feature = "schema"))]
    println!("\nPlease compile with the \"schema\" feature to see the schema output");
    Ok(())
}
