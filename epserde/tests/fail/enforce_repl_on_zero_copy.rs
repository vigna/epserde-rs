/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[epserde(enforce_repl(T))]
#[repr(C)]
struct H<T: Copy>(T);

fn main() {
    let _ = H::<u32>(0);
}
