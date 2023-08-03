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

// Re-export #[derive(Serialize, Deserialize, MemSize, TypeName)].
//
// The reason re-exporting is not enabled by default is that disabling it would
// be annoying for crates that provide handwritten impls or data formats. They
// would need to disable default features and then explicitly re-enable std.
#[cfg(feature = "derive")]
extern crate epserde_derive;

/// Derive macro available if serde is built with `features = ["derive"]`.
#[cfg(feature = "derive")]
pub use epserde_derive::{Deserialize, MemSize, Serialize, TypeName};

pub mod des;
pub mod ser;

pub use des::*;
pub use ser::*;

/// (Major, Minor) version of the file format, this follows semantic versioning
pub const VERSION: (u32, u32) = (0, 0);

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

mod epcopy;
pub use epcopy::*;

pub(crate) mod utils;

/// Compute the padding needed for alignment, i.e., the number so that
/// `((value + pad_align_to(value, bits) & (bits - 1) == 0`.
///
/// ```
/// use epserde::pad_align_to;
/// assert_eq!(7 + pad_align_to(7, 8), 8);
/// assert_eq!(8 + pad_align_to(8, 8), 8);
/// assert_eq!(9 + pad_align_to(9, 8), 16);
/// ```
pub fn pad_align_to(value: usize, bits: usize) -> usize {
    value.wrapping_neg() & (bits - 1)
}

/// A trait to make it easier to check alignment
pub trait CheckAlignment: Sized {
    /// Inner function used to check that the given slice is aligned to
    /// deserialize the current type
    fn check_alignment<'a>(
        mut backend: des::Cursor<'a>,
    ) -> core::result::Result<des::Cursor<'a>, des::DeserializeError> {
        // skip the bytes as needed
        let padding = pad_align_to(backend.pos, core::mem::size_of::<Self>());
        backend = backend.skip(padding);
        // check that the ptr is aligned
        if backend.data.as_ptr() as usize % std::mem::align_of::<Self>() != 0 {
            Err(des::DeserializeError::AlignmentError)
        } else {
            Ok(backend)
        }
    }
}
impl<T: Sized> CheckAlignment for T {}
