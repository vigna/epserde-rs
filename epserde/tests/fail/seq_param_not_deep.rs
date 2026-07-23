/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// Not ε-copy stable (provides error message)
#[derive(Epserde)]
struct SeqParam<A> {
    payload: Vec<A>,
}

fn main() {}
