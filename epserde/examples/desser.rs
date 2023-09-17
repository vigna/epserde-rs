/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

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
    // Create a new value to serialize
    let person = Struct {
        a: vec![0x89; 6],
        b: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = person.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = Struct::deserialize_full(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Struct>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = Struct::deserialize_eps(&buf).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<Struct as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(person.a, eps.a);
    assert_eq!(person.b.a, eps.b.a);
    assert_eq!(person.b.b, eps.b.b);
    assert_eq!(person.test, eps.test);
}
