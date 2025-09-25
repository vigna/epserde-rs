/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

fn main() {
    let v = vec![0, 10, 20, 30, 40];

    let mut buffer = Vec::new();
    unsafe { v.serialize(&mut buffer).unwrap() };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mem_case = unsafe { Vec::<i32>::read_mem(cursor, buffer.len()).unwrap() };
    let r = mem_case.uncase().as_ref();
    let r = r.clone();
    drop(mem_case);
    let _r = r;
}
