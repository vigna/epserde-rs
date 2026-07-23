/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use core::iter::zip;
use epserde::{deser::Owned, prelude::*};

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct PersonVec<A, B> {
    a: A,
    b: B,
    test: isize,
}

impl<A, B> PersonVec<A, B> {
    #[cfg_attr(not(feature = "mmap"), allow(dead_code))]
    pub fn get_test(&self) -> isize {
        self.test
    }
}

#[cfg(feature = "mmap")]
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
    b: Vec<i32>,
}

#[cfg(feature = "mmap")]
#[test]
fn test_mem_case() -> anyhow::Result<()> {
    type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

    // Create a new value to serialize
    let person = Person {
        a: vec![0x89; 6],
        b: Data {
            a: vec![0x42; 7],
            b: vec![0xbadf00d; 2],
        },
        test: -0xbadf00d,
    };
    // Serialize to a unique file in the system temporary directory
    let path = std::env::temp_dir().join(format!("epserde-test-memcase-{}", std::process::id()));
    let result = (|| -> anyhow::Result<()> {
        unsafe { person.store(&path)? };

        let res = unsafe { Person::load_mem(&path)? };
        let res = res.uncase();
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);

        let res = unsafe { Person::load_mmap(&path, Flags::empty())? };
        let res = res.uncase();
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);

        let res = unsafe { Person::load_mem(&path)? };
        let res = res.uncase();
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);

        let res = unsafe { Person::load_full(&path)? };
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);

        let res = unsafe { Person::mmap(&path, Flags::empty())? };
        let res = res.uncase();
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);

        let res = unsafe { Person::mmap(&path, Flags::TRANSPARENT_HUGE_PAGES)? };
        let res = res.uncase();
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);

        let res = unsafe { Person::mmap(&path, Flags::empty())? };
        let res = res.uncase();
        assert_eq!(res.test, person.test);
        assert_eq!(res.get_test(), person.test);
        assert_eq!(res.a, person.a);
        assert_eq!(res.b.a, person.b.a);
        assert_eq!(res.b.b, person.b.b);
        Ok(())
    })();

    // cleanup the file, even if a check failed
    let removed = std::fs::remove_file(&path);
    result?;
    removed?;
    Ok(())
}

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct TestData {
    values: Vec<i32>,
    count: usize,
}

#[test]
fn test_read_mem() -> anyhow::Result<()> {
    let data = TestData {
        values: vec![1, 2, 3, 4, 5],
        count: 42,
    };

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mem_case = unsafe { TestData::read_mem(cursor, buffer.len())? };
    let deserialized = mem_case.uncase();

    assert_eq!(deserialized.values, data.values);
    assert_eq!(deserialized.count, data.count);
    Ok(())
}

#[cfg(feature = "mmap")]
#[test]
fn test_read_mmap() -> anyhow::Result<()> {
    // Create test data

    let data = TestData {
        values: vec![10, 20, 30, 40, 50],
        count: 123,
    };

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mmap_case = unsafe { TestData::read_mmap(cursor, buffer.len(), Flags::empty())? };
    let deserialized = mmap_case.uncase();

    assert_eq!(deserialized.values, data.values);
    assert_eq!(deserialized.count, data.count);
    Ok(())
}

#[test]
fn test_into_iter() -> anyhow::Result<()> {
    let data = vec![10, 20, 30, 40, 50];

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mem_case = unsafe { Vec::<i32>::read_mem(cursor, buffer.len())? };
    let deserialized = *mem_case.uncase();

    zip(data.iter(), deserialized).for_each(|(v, w)| {
        assert_eq!(w, v);
    });
    Ok(())
}

#[test]
fn test_deref() -> anyhow::Result<()> {
    let data = vec![100, 200, 300, 400, 500];

    let mut buffer = Vec::new();
    unsafe { data.serialize(&mut buffer)? };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mem_case: MemCase<Vec<i32>> = unsafe { Vec::<i32>::read_mem(cursor, buffer.len())? };

    assert_eq!(&*mem_case, &data[..]);
    for (d, m) in data.iter().zip(mem_case.iter()) {
        assert_eq!(m, d);
    }

    let mem_case: MemCase<Owned<Vec<i32>>> = data.clone().into();

    assert_eq!(&*mem_case, &data[..]);
    for (d, m) in data.iter().zip(mem_case.iter()) {
        assert_eq!(m, d);
    }

    let _uncase: &Vec<i32> = mem_case.uncase();
    Ok(())
}
