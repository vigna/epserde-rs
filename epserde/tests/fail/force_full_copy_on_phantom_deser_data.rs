/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![allow(deprecated)]

use core::marker::PhantomData;
use epserde::PhantomDeserData;
use epserde::prelude::*;

// force_full_copy on a PhantomDeserData has no effect
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct OnPhantomDeserData<T> {
    data: T,
    #[epserde(force_full_copy)]
    phantom: PhantomDeserData<T>,
}

fn main() {
    let _: OnPhantomDeserData<u32> = OnPhantomDeserData {
        data: 0,
        phantom: PhantomDeserData(PhantomData),
    };
}
