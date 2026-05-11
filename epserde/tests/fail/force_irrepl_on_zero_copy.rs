/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_irrepl on a field of a zero-copy type
 * is rejected.
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
#[epserde(zero_copy)]
#[repr(C)]
struct OnZeroCopy<T: Copy> {
    #[epserde(force_irrepl)]
    inner: T,
}

fn main() {
    let _ = OnZeroCopy::<u32> { inner: 0 };
}
