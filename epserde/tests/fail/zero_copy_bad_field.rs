/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Clone, Copy)]
struct NotZeroCopy(usize);

impl AlignTo for NotZeroCopy {
    fn align_to() -> usize {
        0
    }
}

#[derive(Epserde, Clone, Copy)]
#[repr(C)]
#[epserde_zero_copy]
struct Bad(NotZeroCopy);

fn main() {}
