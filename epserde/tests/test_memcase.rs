/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

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
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_mmap("test.bin", Flags::empty()).unwrap() };
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::load_mem("test.bin").unwrap() };
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
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::TRANSPARENT_HUGE_PAGES).unwrap() };
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty()).unwrap() };
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    // cleanup the file
    std::fs::remove_file("test.bin").unwrap();
}

#[cfg(feature = "mmap")]
#[test]
fn test_memcase_lifetime_safety() {
    let v = vec![0u64, 10, 20, 30, 40];
    let path = std::path::PathBuf::from("/tmp/test_lifetime_safety.vector");

    // Serialize
    unsafe { v.store(&path) }.expect("Could not write vector");

    // Memory map the vector
    let memcase =
        unsafe { <Vec<u64>>::mmap(&path, Flags::RANDOM_ACCESS) }.expect("Could not mmap vector");

    // This should work - borrowing within the same scope
    {
        let slice: &[u64] = &*memcase;
        assert_eq!(slice, &[0u64, 10, 20, 30, 40]);
    }

    // The following code would NOT compile due to lifetime constraints:
    // let slice: &[u64] = &*memcase;
    // drop(memcase);
    // println!("{:?}", slice); // <- This would be a compile error

    // Clean up
    std::fs::remove_file(&path).ok();
}

#[cfg(feature = "mmap")]
#[test]
fn test_memcase_deref_still_works() {
    let v = vec![1u32, 2, 3, 4, 5];
    let path = std::path::PathBuf::from("/tmp/test_deref_works.vector");

    // Serialize
    unsafe { v.store(&path) }.expect("Could not write vector");

    // Memory map
    let memcase =
        unsafe { <Vec<u32>>::mmap(&path, Flags::RANDOM_ACCESS) }.expect("Could not mmap vector");

    // Deref should still work for immediate use
    assert_eq!(*memcase, &[1u32, 2, 3, 4, 5]);
    assert_eq!(memcase.len(), 5);
    assert_eq!(memcase[0], 1);

    // Clean up
    std::fs::remove_file(&path).ok();
}
