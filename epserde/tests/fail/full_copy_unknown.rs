/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// full_copy(...) must name a type parameter
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(full_copy(Q))]
struct Unknown<T> {
    inner: T,
}

fn main() {
    let _ = Unknown::<u32> { inner: 0 };
}
