/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// full_copy(...) requires a list of type parameters
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(full_copy)]
struct OnItem<T>(T);

fn main() {
    let _ = OnItem::<u32>(0);
}
