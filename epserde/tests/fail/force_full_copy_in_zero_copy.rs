/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// No force_full_copy inside a zero-copy type
#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[repr(C)]
struct InZeroCopy<T: Copy> {
    #[epserde(force_full_copy)]
    inner: T,
}

fn main() {
    let _ = InZeroCopy::<u32> { inner: 0 };
}
