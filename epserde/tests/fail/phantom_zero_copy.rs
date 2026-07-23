/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// phantom(...) is meaningless on a zero-copy type
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
#[epserde(zero_copy, phantom(T))]
struct Zc<T> {
    inner: T,
}

fn main() {
    let _ = Zc::<u32> { inner: 0 };
}
