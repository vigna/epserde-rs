/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// Reject non-existent attributes
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Unknown<T> {
    #[epserde(force_full)]
    inner: T,
}

fn main() {
    let _ = Unknown::<u32> { inner: 0 };
}
