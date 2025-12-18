/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example showcasing the convenience serialization of references to slices.
//!
//! Please compile with the "schema" feature to see the schema output.

use epserde::{deser::DeserType, prelude::*, ser::SerType};

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<&[i32]>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<&[i32]>>()
    );
    println!();

    let data = vec![0, 1, 2, 3];
    // Turn it into a slice
    let data: &[i32] = data.as_ref();

    let mut cursor = <AlignedCursor<Aligned16>>::new();

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
    let full = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Vec<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do a full-copy deserialization as a boxed slice
    cursor.set_position(0);
    let full = unsafe { <Box<[i32]>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Box<[i32]>>(),
    );
    println!("Value: {:x?}", full);

    println!();
    // Do an ε-copy deserialization as, again, a slice
    let eps = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Vec<i32>>>(),
    );
    println!("Value: {:x?}", eps);

    println!();
    println!();

    // Let's do with a structure
    let data = Data { a: data };

    println!(
        "Serializable type: {}",
        core::any::type_name::<Data<&[i32]>>()
    );
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Data<&[i32]>>>()
    );
    println!();

    // Serialize the structure
    cursor.set_position(0);
    #[cfg(feature = "schema")]
    {
        let schema = unsafe { data.serialize_with_schema(&mut cursor)? };
        println!("{}", schema.debug(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { data.serialize(&mut cursor)? };

    // Do a full-copy deserialization with field as a vector
    cursor.set_position(0);
    let full = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do a full-copy deserialization with field as a boxed slice
    cursor.set_position(0);
    let full = unsafe { <Data<Box<[i32]>>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Data<Box<[i32]>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Data<Vec<i32>>>>(),
    );
    println!("Value: {:x?}", eps);

    #[cfg(not(feature = "schema"))]
    println!("\nPlease compile with the \"schema\" feature to see the schema output");
    Ok(())
}
