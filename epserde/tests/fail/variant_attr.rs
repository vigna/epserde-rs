/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// #[epserde(...)] attributes are not supported on enum variants: field-level
// markers such as force_full_copy must be placed on the fields of the variant.
#[derive(Epserde)]
enum Enum {
    #[epserde(force_full_copy)]
    A(Vec<u32>),
    B,
}

fn main() {}
