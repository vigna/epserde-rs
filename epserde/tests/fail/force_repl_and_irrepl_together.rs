/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: the two markers are mutually exclusive on
 * the same field.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Both<T> {
    #[epserde(force_repl)]
    #[epserde(force_irrepl)]
    inner: T,
}

fn main() {
    let _ = Both::<u32> { inner: 0 };
}
