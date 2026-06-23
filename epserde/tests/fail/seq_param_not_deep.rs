/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// Not ε-copy stable (provides error message)
#[derive(Epserde)]
struct SeqParam<A> {
    payload: Vec<A>,
}

fn main() {}
