/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// A zero-copy type must be repr(C)
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Copy)]
#[epserde(zero_copy)]
struct NotReprC {
    a: usize,
}

fn main() {
    let _ = NotReprC { a: 0 };
}
