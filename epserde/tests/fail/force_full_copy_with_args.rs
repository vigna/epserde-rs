/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 */

use epserde::prelude::*;

// force_full_copy has no arguments
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct WithArgs<T> {
    #[epserde(force_full_copy(T))]
    inner: T,
}

fn main() {
    let _ = WithArgs::<u32> { inner: 0 };
}
