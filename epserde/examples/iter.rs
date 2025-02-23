/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use std::{iter::Copied, slice::Iter};

/// Example showcasing the convenience serialization of iterators.
use epserde::{impls::iter::ZeroCopyIter, prelude::*};
use maligned::A16;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
#[repr(C)]
#[zero_copy]
struct Data<A: 'static> {
    a: A,
}

fn main() {
    let a = vec![0, 1, 2, 3];
    // Turn it into an interator
    let i: Copied<Iter<'_, i32>> =  a.iter().copied();

    println!("Original type: {}", std::any::type_name::<Copied<Iter<'_, i32>>>());

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize the slice
    let _bytes_written = ZeroCopyIter::from(i).serialize(&mut cursor).unwrap();

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

    let i: Copied<Iter<'_, i32>> =  a.iter().copied();    

    // Let's do with a structure
    let d: Data<ZeroCopyIter<i32, Copied<Iter<'_, i32>>>> = Data { a: ZeroCopyIter::from(i) };

    println!("Original type: {}", std::any::type_name::<Data<Data<ZeroCopyIter<i32, Copied<Iter<'_, i32>>>>>>());

    // Serialize the structure
    cursor.set_position(0);
    let _bytes_written = d.serialize(&mut cursor).unwrap();

    // Do a full-copy deserializations
    cursor.set_position(0);
    let full = <Data<Vec<i32>>>::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Data<Vec<i32>>>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization as, again, a slice
    let eps = <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes()).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Data<Vec<i32>> as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
