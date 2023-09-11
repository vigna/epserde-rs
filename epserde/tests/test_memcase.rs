/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

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
    b: Vec<i32>,
}

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

#[test]
fn test_mem_case() {
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

    let res = load::<Person>("test.bin").unwrap();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = load_mmap::<Person>("test.bin", Flags::empty()).unwrap();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = load::<Person>("test.bin").unwrap();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = map::<Person>("test.bin", Flags::empty()).unwrap();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = map::<Person>("test.bin", Flags::TRANSPARENT_HUGE_PAGES).unwrap();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = map::<Person>("test.bin", Flags::empty()).unwrap();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    // cleanup the file
    std::fs::remove_file("test.bin").unwrap();
}
