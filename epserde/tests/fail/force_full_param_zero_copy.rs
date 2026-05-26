/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_full(...) is meaningless on a zero-copy
 * type, whose deserialization type is a reference and substitutes no
 * parameter.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
#[epserde(zero_copy, force_full(T))]
struct Zc<T> {
    inner: T,
}

fn main() {
    let _ = Zc::<u32> { inner: 0 };
}
