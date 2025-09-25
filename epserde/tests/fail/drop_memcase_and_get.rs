/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

fn main() {
    let vec = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];

    let mut buffer = Vec::new();
    unsafe { vec.serialize(&mut buffer).unwrap() };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mem_case = unsafe { <Vec<String>>::read_mem(cursor, buffer.len()).unwrap() };

    let s = mem_case.uncase().get(0);
    drop(mem_case);
    assert_eq!(s, Some(&"foo"));
}
