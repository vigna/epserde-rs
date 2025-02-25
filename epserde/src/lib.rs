/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![deny(unconditional_recursion)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "derive")]
pub use epserde_derive::{Epserde, TypeInfo};

pub mod deser;
pub mod impls;
pub mod ser;
pub mod traits;
pub mod utils;

pub mod prelude {
    pub use crate::deser;
    pub use crate::deser::DeserType;
    pub use crate::deser::Deserialize;
    pub use crate::deser::DeserializeHelper;
    pub use crate::deser::DeserializeInner;
    pub use crate::deser::Flags;
    pub use crate::deser::MemCase;
    pub use crate::deser::ReadWithPos;
    pub use crate::deser::SliceWithPos;
    pub use crate::impls::iter::SerIter;
    pub use crate::ser;
    pub use crate::ser::Serialize;
    pub use crate::ser::SerializeHelper;
    pub use crate::ser::SerializeInner;
    pub use crate::traits::*;
    pub use crate::utils::*;
    #[cfg(feature = "derive")]
    pub use epserde_derive::Epserde;
}

/// (Major, Minor) version of the file format, this follows semantic versioning
pub const VERSION: (u16, u16) = (1, 1);

/// Magic cookie, also used as endianess marker.
pub const MAGIC: u64 = u64::from_ne_bytes(*b"epserde ");
/// What we will read if the endianness is mismatched.
pub const MAGIC_REV: u64 = u64::from_le_bytes(MAGIC.to_be_bytes());

/// Compute the padding needed for alignment, that is, the smallest
/// number such that `((value + pad_align_to(value, align_to) & (align_to - 1) == 0`.
pub fn pad_align_to(value: usize, align_to: usize) -> usize {
    value.wrapping_neg() & (align_to - 1)
}

#[test]

fn test_pad_align_to() {
    assert_eq!(7 + pad_align_to(7, 8), 8);
    assert_eq!(8 + pad_align_to(8, 8), 8);
    assert_eq!(9 + pad_align_to(9, 8), 16);
    assert_eq!(36 + pad_align_to(36, 16), 48);
}
