/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// full_copy(...) expects a type parameter
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(full_copy(N))]
struct ConstParam<const N: usize> {
    inner: [u32; N],
}

fn main() {
    let _ = ConstParam::<3> { inner: [0; 3] };
}
