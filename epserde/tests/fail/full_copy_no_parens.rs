/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: at the type level, full_copy requires a
 * parenthesized list of type parameters, e.g. #[epserde(full_copy(T))].
 * Writing it bare is rejected by parse_epserde_attrs.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(full_copy)]
struct OnItem<T>(T);

fn main() {
    let _ = OnItem::<u32>(0);
}
