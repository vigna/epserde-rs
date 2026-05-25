/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: a type parameter cannot be both ε-copy
 * (appearing at a variable position in an unmarked field) and
 * full-copy (appearing at a variable position in a field marked
 * #[epserde(force_full)]). The derive rejects the conflict at derive
 * time so the user gets a clear diagnostic instead of an opaque slot
 * mismatch from the generated body.
 */

use epserde::prelude::*;

#[derive(Epserde)]
struct Inner<T: DeepCopy>(#[epserde(force_full)] Vec<T>);

#[derive(Epserde)]
struct Outer<T: DeepCopy> {
    a: T,
    #[epserde(force_full)]
    b: Inner<T>,
}

fn main() {
    let _ = Outer::<Vec<i32>> {
        a: vec![],
        b: Inner(vec![]),
    };
}
