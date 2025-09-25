/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use std::slice::Iter;

/// Example showcasing the convenience serialization of iterators.
use epserde::{impls::iter::SerIter, prelude::*};
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
}

fn main() {
    let a = vec![0, 1, 2, 3];
    // Turn it into an iterator
    let i: Iter<'_, i32> = a.iter();

    println!("Original type: {}", core::any::type_name::<Iter<'_, i32>>());

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize the iterator
    let _bytes_written = unsafe { SerIter::from(i).serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization as a vector
    cursor.set_position(0);
    let full = unsafe { <Vec<i32>>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        core::any::type_name::<Vec<i32>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization as a slice
    let eps = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        core::any::type_name::<DeserType<'_, Vec<i32>>>(),
    );
    println!("Value: {:x?}", eps);

    println!();
    println!();

    // Let's do with a structure
    let i: Iter<'_, i32> = a.iter();
    let d: Data<SerIter<i32, Iter<'_, i32>>> = Data {
        a: SerIter::from(i),
    };

    println!(
        "Original type: {}",
        core::any::type_name::<Data<Data<SerIter<i32, Iter<'_, i32>>>>>()
    );

    // Serialize the structure
    cursor.set_position(0);
    let _bytes_written = unsafe { d.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        core::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        core::any::type_name::<DeserType<'_, Data<Vec<i32>>>>(),
    );
    println!("Value: {:x?}", eps);
}
