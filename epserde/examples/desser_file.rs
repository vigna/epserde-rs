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
    let s = Struct {
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
    let _bytes_written = s.serialize(&mut file).unwrap();

    drop(file);

    let mut file = std::fs::File::open("test.bin").unwrap();

    // Do a full-copy deserialization

    let full = Struct::deserialize_full_copy(&mut file).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<Struct>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(s, full);

    println!();

    // Do an ε-copy deserialization
    let file = std::fs::read("test.bin").unwrap();
    let eps = Struct::deserialize_eps_copy(&file).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<Struct as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(s.a, eps.a);
    assert_eq!(s.b.a, eps.b.a);
    assert_eq!(s.b.b, eps.b.b);
    assert_eq!(s.test, eps.test);
}
