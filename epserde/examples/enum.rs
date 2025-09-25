/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example of an enum with variants some of which depend
/// on a parameter, and some dont. See in particular the failed
/// check of type hash.
use epserde::prelude::*;
use maligned::A16;

#[derive(Epserde, Debug, Clone, Copy)]
enum Data<T = Vec<i32>> {
    A,                        // Unit type
    B { a: usize, b: usize }, // Struct variant with two fields
    C(T),                     // Tuple variant with one parametric field
}

fn main() {
    // Note that we need an explicitly type annotation here,
    // as the type of the enum is not fully determined by the
    // value--we need to know the type of the parameter, which
    // is assumed to be `Vec<i32>` by default.
    let a: Data = Data::A;
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<DeserType<'_, Data>>(),
    );
    println!("Value: {:x?}", eps);

    // Now we give to the parameter a type different from the
    // default one.
    let a: Data<Vec<usize>> = Data::A;
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor).unwrap() };

    println!();

    println!("Deserializing with a different parameter type...");
    // When we try to deserialize without specifying again
    // the type, we get an error even if we just serialized
    // Data::A because the default value of the parameter
    // is different from the one we used.
    cursor.set_position(0);
    println!("Error in full-copy deserialization: {}", unsafe {
        <Data>::deserialize_full(&mut cursor).err().unwrap()
    });
}
