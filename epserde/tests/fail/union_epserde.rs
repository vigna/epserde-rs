/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// Unions are not supported
#[derive(Epserde)]
union Union {
    a: usize,
    b: i64,
}

fn main() {
    let _ = Union { a: 0 };
}
