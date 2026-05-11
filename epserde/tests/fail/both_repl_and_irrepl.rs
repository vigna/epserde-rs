/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: a parameter appearing both as a direct field
 * and inside an unmarked wrapper field is classified as both
 * replaceable and irreplaceable, triggering the derive's conflict
 * diagnostic.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy)]
struct A<T>(T);

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct BothReplAndIrrepl<T> {
    direct: T,
    wrapped: A<T>,
}

fn main() {
    let _ = BothReplAndIrrepl::<u32> { direct: 0, wrapped: A(0) };
}
