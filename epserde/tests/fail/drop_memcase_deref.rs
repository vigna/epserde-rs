/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use std::io::Cursor;
use std::ops::Deref;

#[allow(suspicious_double_ref_op)]
fn main() {
    let v = vec![0, 10, 20, 30, 40];

    let mut buffer = Vec::new();
    unsafe { v.serialize(&mut buffer).unwrap() };
    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { Vec::<i32>::read_mem(cursor, buffer.len()).unwrap() };
    let d = mem_case.deref();
    let d = d.clone();
    drop(mem_case);
    let _d = d;
}
