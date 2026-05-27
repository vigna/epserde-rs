/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_full_copy cannot appear inside a zero-copy
 * type. Zero-copy structs are (de)serialized as raw bytes with no
 * per-field choice between full and eps deserialization, so the
 * marker has no operational meaning there.
 */

use epserde::prelude::*;

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
