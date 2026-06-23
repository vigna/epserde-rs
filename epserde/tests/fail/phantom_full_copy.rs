/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// The same parameter cannot be declared both phantom and full-copy
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(full_copy(T), phantom(T))]
struct Conflict<T> {
    inner: T,
}

fn main() {
    let _ = Conflict::<u32> { inner: 0 };
}
