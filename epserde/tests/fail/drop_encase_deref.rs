/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use std::ops::Deref;

fn main() {
    let v = vec![0, 10, 20, 30, 40];
    let mem_case = MemCase::encase(v);
    let d = mem_case.deref();
    drop(mem_case);
    let _d = d;
}
