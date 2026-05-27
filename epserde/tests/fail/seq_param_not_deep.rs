/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 *
 * Compile-fail fixture: a type parameter that occurs as the element of a
 * literal vector (or boxed slice, or array) in an eps-copy field must be
 * deep-copy. Were it zero-copy, the sequence would eps-copy deserialize to a
 * slice reference, a type not expressible as the original sequence, so the
 * type would not be eps-copy stable. The derive surfaces this through the
 * DeepCopyInSeq assertion; the fix is to bound the parameter with DeepCopy or
 * to mark the field with #[epserde(force_full_copy)].
 */

use epserde::prelude::*;

#[derive(Epserde)]
struct SeqParam<A> {
    payload: Vec<A>,
}

fn main() {}
