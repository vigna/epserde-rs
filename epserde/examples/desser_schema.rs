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
    /// TODO: does it depend on the fact it's inner?
    /// This is an inner field whose type is not a parameter,
    /// so it will not be ε-copied, but rather fully copied.
    b: Vec<i32>,
}

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

fn main() {
    // create a new value to serialize
    let person = Person {
        a: vec![0x89; 6],
        b: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
    // Create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // Wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let mut schema = person.serialize_with_schema(&mut buf).unwrap();
    // Sort the schema by offset so we can print it in order
    schema.0.sort_by_key(|a| a.offset);
    let buf = buf.into_inner();
    println!("{}", schema.debug(buf));

    // Do a full-copy deserialization
    let buf = std::io::Cursor::new(&mut v);
    let full = Person::deserialize_full_copy(buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Person>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let eps = Person::deserialize_eps_copy(&v).unwrap();
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
