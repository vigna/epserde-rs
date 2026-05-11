/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_irrepl is only valid on a field whose
 * type is a single-segment struct generic. Applying it to a concrete
 * wrapper is rejected at derive time.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct OnNonParam<T> {
    #[epserde(force_irrepl)]
    bad: Vec<T>,
}

fn main() {
    let _: OnNonParam<u32> = OnNonParam { bad: vec![] };
}
