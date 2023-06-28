/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use epserde_derive::{Deserialize, Serialize};
use epserde_trait::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
struct Person<'a, A: Eq, B>
where
    B: PartialEq,
{
    name: A,
    age: B,
    test: isize,
    slice: &'a [u8],
}

fn main() {
    let person0 = Person {
        name: 0xdeadbeed_u32,
        age: 0xdeadbeefdeadf00d_u64,
        test: -0xbadf00d,
        slice: b"Hello, world!",
    };
    let mut v = vec![0; 100];
    let mut buf = std::io::Cursor::new(&mut v);
    person0.serialize(&mut buf).unwrap();
    let (person1, _rest) = Person::<u32, u64>::deserialize(&v).unwrap();
    assert_eq!(person0, person1);
}
