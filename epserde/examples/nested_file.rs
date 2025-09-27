/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example of a nested struct in which one of the fields of the inner struct is
//! recursively ε-copied, as its type is a parameter. We also serialize on file.
//!
//! When deserializing, we show three variants: one in which both parameters are
//! `Vec`, one in which the outer parameter is a boxed slice and the inner
//! parameter is a `Vec`, and one in which both parameters are boxed slices.
//!
//! Please compile with the "schema" feature to see the schema output.

#[cfg(not(feature = "std"))]
fn main() {
    println!("This example requires the standard library");
}

#[cfg(feature = "std")]
use epserde::{deser::DeserType, prelude::*, ser::SerType};

#[cfg(feature = "std")]
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct StructParam<A, B> {
    a: A,
    b: B,
    test: isize,
}

#[cfg(feature = "std")]
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
    /// This is a field whose type is not a parameter,
    /// so it will not be ε-copied, but rather fully copied.
    b: Vec<i32>,
}

#[cfg(feature = "std")]
type Type = StructParam<Vec<usize>, Data<Vec<u16>>>;
#[cfg(feature = "std")]
type TypeOneBoxed = StructParam<Box<[usize]>, Data<Vec<u16>>>;
#[cfg(feature = "std")]
type TypeBothBoxed = StructParam<Box<[usize]>, Data<Box<[u16]>>>;

#[cfg(feature = "std")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    const FILE_NAME: &str = "test.bin";

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
    // Create an aligned vector to serialize into so we can do an ε-copy
    // deserialization safely
    let mut file = std::fs::File::create(FILE_NAME)?;

    // Serialize
    #[cfg(feature = "schema")]
    {
        let schema = unsafe { data.serialize_with_schema(&mut file)? };
        println!("{}", schema.debug(&std::fs::read(FILE_NAME)?));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { data.serialize(&mut file)? };

    drop(file);

    // Do a full-copy deserialization with vectors
    let mut file = std::fs::File::open(FILE_NAME)?;
    let full = unsafe { Type::deserialize_full(&mut file)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<Type>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(data, full);

    println!();

    // Do a full-copy deserialization with one boxed slice
    let mut file = std::fs::File::open(FILE_NAME)?;
    let full = unsafe { TypeOneBoxed::deserialize_full(&mut file)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<TypeOneBoxed>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do a full-copy deserialization with boxed slices
    let mut file = std::fs::File::open(FILE_NAME)?;
    let full = unsafe { TypeBothBoxed::deserialize_full(&mut file)? };
    println!(
        "Full-copy deserialization: returns the deserializable type {}",
        core::any::type_name::<TypeBothBoxed>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let file = std::fs::read(FILE_NAME)?;
    let eps = unsafe { Type::deserialize_eps(&file)? };
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
