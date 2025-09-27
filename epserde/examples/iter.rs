/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example showcasing the convenience serialization of iterators.
//!
//! Note that we deserialize into `Vec` or `Box<[T]>`, as iterators cannot be
//! deserialized directly.
//!
//! Please compile with the "schema" feature to see the schema output.

use std::slice::Iter;

use epserde::{impls::iter::SerIter, prelude::*, ser::SerType};
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
}

type Type<'a> = SerIter<'a, i32, Iter<'a, i32>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<Type>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Type>>()
    );
    println!();

    let a = vec![0, 1, 2, 3];
    // Turn it into an iterator
    let i = a.iter();

    let mut cursor = <AlignedCursor<A16>>::new();

    // Serialize the iterator
    #[cfg(feature = "schema")]
    {
        let schema = unsafe { SerIter::from(i).serialize_with_schema(&mut cursor)? };
        println!("{}", schema.debug(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { SerIter::from(i).serialize(&mut cursor)? };

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
        core::any::type_name::<Box<[i32]>>()
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization as a slice
    let eps = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Vec<i32>>>(),
    );
    println!("Value: {:x?}", eps);

    println!();
    println!();

    // Let's do it with a structure
    let i = a.iter();
    let d: Data<Type> = Data {
        a: SerIter::from(i),
    };

    println!(
        "Serializable type: {}",
        core::any::type_name::<Data<Type>>()
    );
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Data<Type>>>()
    );
    println!();

    cursor.set_position(0);

    // Serialize the structure
    #[cfg(feature = "schema")]
    {
        let schema = unsafe { d.serialize_with_schema(&mut cursor)? };
        println!("{}", schema.debug(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { d.serialize(&mut cursor)? };

    // Do a full-copy deserialization with a vector
    cursor.set_position(0);
    let full = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do a full-copy deserialization with a boxed slice
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
