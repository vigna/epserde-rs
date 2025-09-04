/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use std::io::Cursor;

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

#[cfg(feature = "mmap")]
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
    // Serialize
    unsafe { person.store("test.bin").unwrap() };

    let res = unsafe { Person::load_mem("test.bin").unwrap() };
    let res = res.get();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_mmap("test.bin", Flags::empty()).unwrap() };
    let res = res.get();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_mem("test.bin").unwrap() };
    let res = res.get();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_full("test.bin").unwrap() };
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty()).unwrap() };
    let res = res.get();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::TRANSPARENT_HUGE_PAGES).unwrap() };
    let res = res.get();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty()).unwrap() };
    let res = res.get();
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    // cleanup the file
    std::fs::remove_file("test.bin").unwrap();
}

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct TestData {
    values: Vec<i32>,
    count: usize,
}

#[test]
fn test_read_mem() {
    // Create test data
    let data = TestData {
        values: vec![1, 2, 3, 4, 5],
        count: 42,
    };

    // Serialize to a buffer
    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer).unwrap() };

    // Test read_mem with a Cursor (implements Read)
    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { TestData::read_mem(cursor, buffer.len()).unwrap() };
    let deserialized = mem_case.get();

    assert_eq!(data.values, deserialized.values);
    assert_eq!(data.count, deserialized.count);
}

#[cfg(feature = "mmap")]
#[test]
fn test_read_mmap() {
    // Create test data

    let data = TestData {
        values: vec![10, 20, 30, 40, 50],
        count: 123,
    };

    // Serialize to a buffer
    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer).unwrap() };

    // Test read_mmap with a Cursor (implements Read)
    let cursor = Cursor::new(&buffer);
    let mmap_case = unsafe { TestData::read_mmap(cursor, buffer.len(), Flags::empty()).unwrap() };
    let deserialized = mmap_case.get();

    assert_eq!(data.values, deserialized.values);
    assert_eq!(data.count, deserialized.count);
}
