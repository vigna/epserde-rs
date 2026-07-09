/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! `AlignedCursor` seals its type parameter to `AlignmentBlock`, so a type that
//! merely happens to be `Default + Clone` must be rejected. `String` has drop
//! glue and invalid bit patterns; permitting `AlignedCursor<String>` would let
//! safe code overwrite its fields with arbitrary bytes (undefined behavior).

use epserde::prelude::*;

fn main() {
    let _cursor = AlignedCursor::<String>::new();
}
