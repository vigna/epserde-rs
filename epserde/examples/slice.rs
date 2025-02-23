/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showcasing the convenience serialization of references to slices.
use epserde::prelude::*;
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
}

fn main() {
    let a = vec![0, 1, 2, 3];
    // Turn it into a slice
    let a: &[i32] = a.as_ref();

    println!("Original type: {}", std::any::type_name::<&[i32]>());

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize the slice
    let _bytes_written = a.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization as a vector
    cursor.set_position(0);
    let full = <Vec<i32>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Vec<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization as, again, a slice
    let eps = <Vec<i32>>::deserialize_eps(cursor.as_bytes()).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Vec<i32> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);

    println!();
    println!();

    // Let's do with a structure
    let d = Data { a };

    println!("Original type: {}", std::any::type_name::<Data<&[i32]>>());

    // Serialize the structure
    cursor.set_position(0);
    let _bytes_written = d.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data<Vec<i32>>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes()).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Data<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
