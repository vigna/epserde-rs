/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example of a nested struct in which one of the fields of the inner struct is
//! recursively ε-copied, as its type is a parameter.
//!
//! When deserializing, we show three variants: one in which both
//! parameters are `Vec`, one in which the outer parameter is a boxed slice
//! and the inner parameter is a `Vec`, and one in which both parameters
//! are boxed slices.
//!
//! Please compile with the "schema" feature to see the schema output.

use epserde::{deser::DeserType, prelude::*, ser::SerType};

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct StructParam<A, B> {
    a: A,
    b: B,
    test: isize,
}

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
    /// This is a field whose type is not a parameter,
    /// so it will not be ε-copied, but rather fully copied.
    b: Vec<i32>,
}

type Type = StructParam<Vec<usize>, Data<Vec<u16>>>;
type TypeOneBoxed = StructParam<Box<[usize]>, Data<Vec<u16>>>;
type TypeBothBoxed = StructParam<Box<[usize]>, Data<Box<[u16]>>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializable type: {}", core::any::type_name::<Type>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<Type>>()
    );
    println!();

    // Create a new value to serialize
    let data = Type {
        a: vec![0x1; 4],
        b: Data {
            a: vec![0x2; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
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

    // Do a full-copy deserialization with vectors
    cursor.set_position(0);
    let full = unsafe { Type::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Type>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(data, full);

    println!();

    // Do a full-copy deserialization with one boxed slice
    cursor.set_position(0);
    let full = unsafe { TypeOneBoxed::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<TypeOneBoxed>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do a full-copy deserialization with boxed slices
    cursor.set_position(0);
    let full = unsafe { TypeBothBoxed::deserialize_full(&mut cursor)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<TypeBothBoxed>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { Type::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, Type>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(data.a, eps.a);
    assert_eq!(data.b.a, eps.b.a);
    assert_eq!(data.b.b, eps.b.b);
    assert_eq!(data.test, eps.test);

    #[cfg(not(feature = "schema"))]
    println!("\nPlease compile with the \"schema\" feature to see the schema output");
    Ok(())
}
