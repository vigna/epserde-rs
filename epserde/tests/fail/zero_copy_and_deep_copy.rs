/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// A type cannot be declared both zero-copy and deep-copy
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
#[epserde(zero_copy)]
#[epserde(deep_copy)]
struct Both {
    a: usize,
}

fn main() {
    let _ = Both { a: 0 };
}
