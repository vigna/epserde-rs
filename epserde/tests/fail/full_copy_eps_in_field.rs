/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

// A full_copy(...)-pinned parameter F shares a field with an ε-copy parameter
// E, and the field type (ControlFlow) ε-copy deserializes F. The slot the
// derive emits keeps F verbatim, so it disagrees with the field's real
// deserialization type. The FullCopyConsistent assertion surfaces an
// actionable diagnostic instead of a raw slot mismatch.

use epserde::prelude::*;

// The field type is written with a qualified path on purpose: the diagnostic
// must point at the constructor `ControlFlow`, not the leading `std` qualifier.
#[derive(Epserde)]
#[epserde(full_copy(F))]
struct S<F, E>(std::ops::ControlFlow<F, E>);

fn main() {}
