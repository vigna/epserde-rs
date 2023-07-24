/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]
#![deny(unconditional_recursion)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

pub mod des;
pub use des::*;

pub mod des_impl;
pub use des_impl::*;

pub mod ser;
pub use ser::*;

pub mod ser_impl;
pub use ser_impl::*;

#[macro_use]
extern crate lazy_static;
lazy_static! {
    /// (Major, Minor) version of the format.
    /// this uses the environment variable `CARGO_PKG_VERSION_MAJOR` and
    /// `CARGO_PKG_VERSION_MINOR` to get the major and minor version number of the
    /// package.
    /// This way the serialization format is tied to the version of the library,
    /// and thus it follows the semver rules.
    pub static ref VERSION: (u32, u32) = (
        env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
    );
}

/// Magic + endianess marker
pub const MAGIC: u64 = u64::from_ne_bytes(*b"epserdes");
/// What we will read if the endianness is mismatched
pub const MAGIC_REV: u64 = u64::from_le_bytes(MAGIC.to_be_bytes());

mod type_name;
pub use type_name::*;

mod memsize;
pub use memsize::*;

mod memcase;
pub use memcase::*;

pub(crate) mod utils;
