/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Basic traits that must be implemented by all types using ε-serde.

If you use the procedural macro [`Epserde`](epserde_derive::Epserde), you do not
need to worry about these traits—they will be implemented for you.

*/

pub mod type_info;
pub use type_info::*;

pub mod copy_type;
pub use copy_type::*;
