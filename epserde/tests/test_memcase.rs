/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use std::{io::Cursor, iter::zip};
use anyhow::Result;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct PersonVec<A, B> {
    a: A,
    b: B,
    test: isize,
}

impl<A, B> PersonVec<A, B> {
    pub fn get_test(&self) -> isize {
        self.test
    }
}

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
    b: Vec<i32>,
}

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

#[cfg(feature = "mmap")]
#[test]
fn test_mem_case() -> Result<()> {
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
    unsafe { person.store("test.bin")? };

    let res = unsafe { Person::load_mem("test.bin")? };
    let res = res.uncase();
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_mmap("test.bin", Flags::empty())? };
    let res = res.uncase();
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_mem("test.bin")? };
    let res = res.uncase();
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_full("test.bin")? };
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty())? };
    let res = res.uncase();
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::TRANSPARENT_HUGE_PAGES)? };
    let res = res.uncase();
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty())? };
    let res = res.uncase();
    assert_eq!(person.test, res.test);
    assert_eq!(person.test, res.get_test());
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    // cleanup the file
    std::fs::remove_file("test.bin")?;
    Ok(())
}

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct TestData {
    values: Vec<i32>,
    count: usize,
}

#[test]
fn test_read_mem() -> Result<()> {
    let data = TestData {
        values: vec![1, 2, 3, 4, 5],
        count: 42,
    };

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { TestData::read_mem(cursor, buffer.len())? };
    let deserialized = mem_case.uncase();

    assert_eq!(data.values, deserialized.values);
    assert_eq!(data.count, deserialized.count);
    Ok(())
}

#[cfg(feature = "mmap")]
#[test]
fn test_read_mmap() -> Result<()> {
    // Create test data

    let data = TestData {
        values: vec![10, 20, 30, 40, 50],
        count: 123,
    };

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = Cursor::new(&buffer);
    let mmap_case = unsafe { TestData::read_mmap(cursor, buffer.len(), Flags::empty())? };
    let deserialized = mmap_case.uncase();

    assert_eq!(data.values, deserialized.values);
    assert_eq!(data.count, deserialized.count);
    Ok(())
}

#[test]
fn test_into_iter() -> Result<()> {
    let data = vec![10, 20, 30, 40, 50];

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { Vec::<i32>::read_mem(cursor, buffer.len())? };
    let deserialized = mem_case.uncase();

    zip(data.iter(), deserialized.into_iter()).for_each(|(v, w)| {
        assert_eq!(v, w);
    });
    Ok(())
}
