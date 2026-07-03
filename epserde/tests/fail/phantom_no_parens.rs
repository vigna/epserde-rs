/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// phantom(...) requires a list of type parameters
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(phantom)]
struct OnItem<T>(std::marker::PhantomData<T>);

fn main() {
    let _ = OnItem::<u32>(std::marker::PhantomData);
}
