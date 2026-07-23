/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// phantom(...) must name a type parameter
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(phantom(Q))]
struct Unknown<T> {
    inner: T,
}

fn main() {
    let _ = Unknown::<u32> { inner: 0 };
}
