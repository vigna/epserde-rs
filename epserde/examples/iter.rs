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

use epserde::{impls::iter::SerIter, prelude::*, ser::SerType};

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
}

type Type = SerIter<i32, std::vec::IntoIter<i32>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<Type>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Type>>()
    );
    println!();

    let a = vec![0, 1, 2, 3];

    let mut cursor = <AlignedCursor<Aligned16>>::new();

    #[cfg(feature = "schema")]
    {
        let i = a.clone().into_iter();
        let schema = unsafe { SerIter::from(i).serialize_with_schema(&mut cursor)? };
        println!("{}", schema.debug(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    {
        let i = a.clone().into_iter();
        let _bytes_written = unsafe { SerIter::from(i).serialize(&mut cursor)? };
    }

    cursor.set_position(0);
    let full = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Vec<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    cursor.set_position(0);
    let full = unsafe { <Box<[i32]>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Box<[i32]>>()
    );
    println!("Value: {:x?}", full);

    println!();

    let eps = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Vec<i32>>>(),
    );
    println!("Value: {:x?}", eps);

    println!();
    println!();

    let i = a.iter();
    let d: Data<SerIter<i32, core::slice::Iter<i32>>> = Data {
        a: SerIter::from(i),
    };

    println!(
        "Serializable type: {}",
        core::any::type_name::<Data<SerIter<i32, core::slice::Iter<i32>>>>()
    );
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Data<SerIter<i32, core::slice::Iter<i32>>>>>()
    );
    println!();

    cursor.set_position(0);

    #[cfg(feature = "schema")]
    {
        let schema = unsafe { d.serialize_with_schema(&mut cursor)? };
        println!("{}", schema.debug(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { d.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    cursor.set_position(0);
    let full = unsafe { <Data<Box<[i32]>>>::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Data<Box<[i32]>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

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
