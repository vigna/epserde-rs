/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

// phantom(...) requires a list of type parameters
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(phantom)]
struct OnItem<T>(std::marker::PhantomData<T>);

fn main() {
    let _ = OnItem::<u32>(std::marker::PhantomData);
}
