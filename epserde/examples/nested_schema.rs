/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example of a nested struct in which one of the fields
/// of the inner struct is recursively ε-copied, as its
/// type is a parameter. We also generate a schema.
use epserde::prelude::*;
use maligned::A16;

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

type Struct = StructParam<Vec<usize>, Data<Vec<u16>>>;

fn main() {
    // create a new value to serialize
    let person = Struct {
        a: vec![0x89; 6],
        b: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let schema = unsafe { person.ser_with_schema(&mut cursor).unwrap() };

    // Show the schema
    println!("{}", schema.debug(cursor.as_bytes()));

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { Struct::deser_full(&mut cursor).unwrap() };
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Struct>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { Struct::deser_eps(cursor.as_bytes()).unwrap() };
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<Struct as DeserInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(person.a, eps.a);
    assert_eq!(person.b.a, eps.b.a);
    assert_eq!(person.b.b, eps.b.b);
    assert_eq!(person.test, eps.test);
}
