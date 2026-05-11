/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_repl is a field-level marker. Placing
 * it on the item itself is rejected by parse_epserde_attrs (force_repl
 * is no longer one of the recognised item-level keywords).
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(force_repl)]
struct OnItem<T>(T);

fn main() {
    let _ = OnItem::<u32>(0);
}
