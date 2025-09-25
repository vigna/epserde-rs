/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations of [`SerInner`](crate::ser::SerInner) and
//! [`DeserInner`](crate::deser::DeserInner) for standard Rust
//! types.

pub mod array;
pub mod iter;
pub mod pointer;
pub mod prim;
pub mod slice;
pub mod tuple;

pub mod boxed_slice;
#[cfg(feature = "std")]
pub mod stdlib;
pub mod string;
pub mod vec;
