/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::{deser::Owned, prelude::*};
use std::ops::Deref;

fn main() {
    let v = vec![0, 10, 20, 30, 40];
    let mem_case = <MemCase<Owned<Vec<i32>>>>::encase(v);
    let d = mem_case.deref();
    let d = d.clone();
    drop(mem_case);
    let _d = d;
}
