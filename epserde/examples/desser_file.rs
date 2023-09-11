/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct PersonVec<A, B> {
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

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

fn main() {
    // Create a new value to serialize
    let person = Person {
        a: vec![0x89; 6],
        b: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
    // Create an aligned vector to serialize into so we can do an ε-copy
    // deserialization safely
    let mut file = std::fs::File::create("test.bin").unwrap();
    // Serialize
    let _bytes_written = person.serialize(&mut file).unwrap();

    drop(file);

    let file = std::fs::File::open("test.bin").unwrap();

    // Do a full-copy deserialization

    let full = Person::deserialize_full_copy(&file).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Person>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let file = std::fs::read("test.bin").unwrap();
    let eps = Person::deserialize_eps_copy(&file).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<Person as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(person.a, eps.a);
    assert_eq!(person.b.a, eps.b.a);
    assert_eq!(person.b.b, eps.b.b);
    assert_eq!(person.test, eps.test);
}
