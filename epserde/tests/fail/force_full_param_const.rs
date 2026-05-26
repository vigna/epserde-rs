/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_full(...) expects a type parameter, not a
 * const parameter.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_full(N))]
struct ConstParam<const N: usize> {
    inner: [u32; N],
}

fn main() {
    let _ = ConstParam::<3> { inner: [0; 3] };
}
