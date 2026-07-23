/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// phantom(...) cannot name a const parameter
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(phantom(N))]
struct Konst<T, const N: usize> {
    inner: T,
}

fn main() {
    let _ = Konst::<u32, 3> { inner: 0 };
}
