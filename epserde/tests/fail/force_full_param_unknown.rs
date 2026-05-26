/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_full(...) must name a declared type
 * parameter of the item.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_full(Q))]
struct Unknown<T> {
    inner: T,
}

fn main() {
    let _ = Unknown::<u32> { inner: 0 };
}
