/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_full_copy as a field marker takes no
 * arguments; using it with arguments is rejected by the per-field
 * validator.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct WithArgs<T> {
    #[epserde(force_full_copy(T))]
    inner: T,
}

fn main() {
    let _ = WithArgs::<u32> { inner: 0 };
}
