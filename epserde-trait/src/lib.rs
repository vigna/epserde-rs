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
pub mod ser;

pub use des::*;
pub use ser::*;

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

mod epcopy;
pub use epcopy::*;

pub(crate) mod utils;

/// Compute the padding needed for alignement, i.e., the number so that
/// `((value + pad_align_to(value, bits) & (bits - 1) == 0`.
///
/// ```
/// use epserde_trait::pad_align_to;
/// assert_eq!(7 + pad_align_to(7, 8), 8);
/// assert_eq!(8 + pad_align_to(8, 8), 8);
/// assert_eq!(9 + pad_align_to(9, 8), 16);
/// ```
fn pad_align_to(value: usize, bits: usize) -> usize {
    value.wrapping_neg() & (bits - 1)
}

/// A trait to make it easier to check and pad alignement
pub trait CheckAlignement: Sized {
    /// Inner function used to check that the given slice is aligned to
    /// deserialize the current type
    fn check_alignement<'a>(
        mut backend: des::Cursor<'a>,
    ) -> core::result::Result<des::Cursor<'a>, des::DeserializeError> {
        // skip the bytes as needed
        let padding = pad_align_to(backend.pos, core::mem::size_of::<Self>());
        backend = backend.skip(padding);
        // check that the ptr is aligned
        if backend.data.as_ptr() as usize % std::mem::align_of::<Self>() != 0 {
            Err(des::DeserializeError::AlignementError)
        } else {
            Ok(backend)
        }
    }

    /// Write 0 as padding to align to the size of `T`.
    fn pad_align_to<F: ser::WriteWithPosNoStd>(mut backend: F) -> ser::Result<F> {
        let file_pos = backend.get_pos();
        let padding = pad_align_to(file_pos, core::mem::size_of::<Self>());
        for _ in 0..padding {
            backend.write(&[0])?;
        }
        Ok(backend)
    }
}
impl<T: Sized> CheckAlignement for T {}
