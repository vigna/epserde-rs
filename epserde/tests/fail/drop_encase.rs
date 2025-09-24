/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::{deser::Owned, prelude::*};

fn main() {
    let v = vec![0, 10, 20, 30, 40];

    let mem_case = <MemCase<Owned<Vec<i32>>>>::encase(v);
    let u = mem_case.uncase();
    drop(mem_case);
    let _u = u;
}
