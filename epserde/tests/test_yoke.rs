/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

use yoke::Yokeable;
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone, Yokeable)]
struct PersonVec<A, B> {
    a: A,
    b: B,
    test: isize,
}

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone, Yokeable)]
struct Data<A> {
    a: A,
    b: Vec<i32>,
}

type Person = PersonVec<Vec<usize>, Data<Vec<u16>>>;

#[cfg(feature = "mmap")]
#[test]
fn test_yoke() {
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
    assert_eq!(person.test, res.get().test);
    assert_eq!(person.a, res.get().a);
    assert_eq!(person.b.a, res.get().b.a);
    assert_eq!(person.b.b, res.get().b.b);

    let res = unsafe { Person::load_mmap("test.bin", Flags::empty()).unwrap() };
    assert_eq!(person.test, res.get().test);
    assert_eq!(person.a, res.get().a);
    assert_eq!(person.b.a, res.get().b.a);
    assert_eq!(person.b.b, res.get().b.b);

    let res = unsafe { Person::load_mem("test.bin").unwrap() };
    assert_eq!(person.test, res.get().test);
    assert_eq!(person.a, res.get().a);
    assert_eq!(person.b.a, res.get().b.a);
    assert_eq!(person.b.b, res.get().b.b);

    let res = unsafe { Person::load_full("test.bin").unwrap() };
    assert_eq!(person.test, res.test);
    assert_eq!(person.a, res.a);
    assert_eq!(person.b.a, res.b.a);
    assert_eq!(person.b.b, res.b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty()).unwrap() };
    assert_eq!(person.test, res.get().test);
    assert_eq!(person.a, res.get().a);
    assert_eq!(person.b.a, res.get().b.a);
    assert_eq!(person.b.b, res.get().b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::TRANSPARENT_HUGE_PAGES).unwrap() };
    assert_eq!(person.test, res.get().test);
    assert_eq!(person.a, res.get().a);
    assert_eq!(person.b.a, res.get().b.a);
    assert_eq!(person.b.b, res.get().b.b);

    let res = unsafe { Person::mmap("test.bin", Flags::empty()).unwrap() };
    assert_eq!(person.test, res.get().test);
    assert_eq!(person.a, res.get().a);
    assert_eq!(person.b.a, res.get().b.a);
    assert_eq!(person.b.b, res.get().b.b);

    // cleanup the file
    std::fs::remove_file("test.bin").unwrap();
}
