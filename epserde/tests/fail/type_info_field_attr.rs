/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::TypeInfo;

// Field-level #[epserde(...)] attributes only affect derive(Epserde);
// derive(TypeInfo) must reject them instead of silently ignoring them.
#[derive(TypeInfo)]
struct Data {
    #[epserde(force_full_copy)]
    a: Vec<u32>,
}

fn main() {}
