/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde)]
#[epserde(enforce_repl(X))]
struct G<T>(T);

fn main() {
    let _ = G::<u32>(0);
}
