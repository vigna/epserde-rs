/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde)]
struct Data<A>
where
    A: Eq,
{
    a: A,
}

fn main() {
    let _data = Data { a: 42 };
}
