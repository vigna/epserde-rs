/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Implementations of [`SerInner`] and [`DeserInner`] for standard Rust types.
//!
//! [`SerInner`]: crate::ser::SerInner
//! [`DeserInner`]: crate::deser::DeserInner

pub mod array;
pub mod iter;
pub mod prim;
pub mod slice;
pub mod tuple;
pub mod wrapper;

pub mod boxed_slice;
pub mod string;
pub mod vec;

pub mod stdlib;
