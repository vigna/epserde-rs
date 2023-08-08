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

// Re-export epserde_derive conditional to the "derive" feature.
//
// The reason re-exporting is not enabled by default is that disabling it would
// be annoying for crates that provide handwritten impls or data formats. They
// would need to disable default features and then explicitly re-enable std.
#[cfg(feature = "derive")]
extern crate epserde_derive;
#[cfg(feature = "derive")]
pub use epserde_derive::{Deserialize, MemDbg, MemSize, Serialize, TypeName};

pub mod des;
pub mod ser;

pub use des::*;
pub use ser::*;

/// (Major, Minor) version of the file format, this follows semantic versioning
pub const VERSION: (u32, u32) = (0, 0);

/// Magic + endianess marker
pub const MAGIC: u64 = u64::from_ne_bytes(*b"epserde ");
/// What we will read if the endianness is mismatched
pub const MAGIC_REV: u64 = u64::from_le_bytes(MAGIC.to_be_bytes());

mod type_name;
pub use type_name::*;

mod mem_size;
pub use mem_size::*;

mod mem_dbg;
pub use mem_dbg::*;

mod mem_case;
pub use mem_case::*;

mod zero_copy;
pub use zero_copy::*;

pub(crate) mod utils;

/// Compute the padding needed for alignment, i.e., the number so that
/// `((value + pad_align_to(value, bits) & (bits - 1) == 0`.
///
/// ```
/// use epserde::pad_align_to;
/// assert_eq!(7 + pad_align_to(7, 8), 8);
/// assert_eq!(8 + pad_align_to(8, 8), 8);
/// assert_eq!(9 + pad_align_to(9, 8), 16);
/// assert_eq!(36 + pad_align_to(36, 16), 48);
/// ```
pub fn pad_align_to(value: usize, bits: usize) -> usize {
    value.wrapping_neg() & (bits - 1)
}

/// A trait padding a cursor to the correct alignment
/// and checking that the resulting pointer is aligned
/// correctly.
trait Align: Sized {
    /// Pad the cursor to the correct alignment and check that the resulting
    /// pointer is aligned correctly.
    fn pad_align_and_check(
        mut backend: des::Cursor,
    ) -> core::result::Result<des::Cursor, des::DeserializeError> {
        // Skip bytes as needed
        let padding = pad_align_to(backend.pos, core::mem::align_of::<Self>());
        backend = backend.skip(padding);
        // Check that the ptr is indeed aligned
        if backend.data.as_ptr() as usize % std::mem::align_of::<Self>() != 0 {
            Err(des::DeserializeError::AlignmentError)
        } else {
            Ok(backend)
        }
    }
}
impl<T: Sized> Align for T {}
