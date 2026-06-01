/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde)]
struct Inner<T: DeepCopy>(#[epserde(force_full_copy)] Vec<T>);

#[derive(Epserde)]
struct Outer<T: DeepCopy> {
    a: T,
    #[epserde(force_full_copy)]
    b: Inner<T>,
}

fn main() {
    let _ = Outer::<Vec<i32>> {
        a: vec![],
        b: Inner(vec![]),
    };
}
