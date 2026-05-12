/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: force_full on a PhantomDeserData field has no
 * operational effect (the PhantomDeserData branch of the derive emits
 * its own special call regardless of the marker), so the derive
 * rejects this combination to avoid surprising the user.
 */

#![allow(deprecated)]

use core::marker::PhantomData;
use epserde::PhantomDeserData;
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct OnPhantomDeserData<T> {
    data: T,
    #[epserde(force_full)]
    phantom: PhantomDeserData<T>,
}

fn main() {
    let _: OnPhantomDeserData<u32> = OnPhantomDeserData {
        data: 0,
        phantom: PhantomDeserData(PhantomData),
    };
}
