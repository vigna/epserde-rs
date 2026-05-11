/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[repr(C)]
struct H<T: Copy> {
    #[epserde(force_repl)]
    inner: T,
}

fn main() {
    let _ = H::<u32> { inner: 0 };
}
