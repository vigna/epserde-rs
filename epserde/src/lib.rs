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
use std::io::Cursor;

#[cfg(feature = "derive")]
pub use epserde_derive::{Epserde, TypeHash};

pub mod des;
pub mod impls;
pub mod ser;
pub mod traits;

pub mod prelude {
    pub use crate::des;
    pub use crate::des::Deserialize;
    pub use crate::des::DeserializeHelper;
    pub use crate::des::DeserializeInner;
    pub use crate::des::Flags;
    pub use crate::des::MemCase;
    pub use crate::des::ReadWithPos;
    pub use crate::des::SliceWithPos;
    pub use crate::ser;
    pub use crate::ser::Serialize;
    pub use crate::ser::SerializeHelper;
    pub use crate::ser::SerializeInner;
    pub use crate::traits::*;
    #[cfg(feature = "derive")]
    pub use epserde_derive::{Epserde, TypeHash};
}

/// (Major, Minor) version of the file format, this follows semantic versioning
pub const VERSION: (u16, u16) = (0, 0);

/// Magic cookie, also used as endianess marker.
pub const MAGIC: u64 = u64::from_ne_bytes(*b"epserde ");
/// What we will read if the endianness is mismatched.
pub const MAGIC_REV: u64 = u64::from_le_bytes(MAGIC.to_be_bytes());

/// Compute the padding needed for alignment, that is, the smallest
/// number such that `((value + pad_align_to(value, align_to) & (align_to - 1) == 0`.
pub fn pad_align_to(value: usize, align_to: usize) -> usize {
    value.wrapping_neg() & (align_to - 1)
}

/// Return a new cursor initialized with 1024 bytes of memory aligned to 128 bits.
///
/// Note that serialization to a memory-based cursor is mainly
/// useful for debugging purposes and for examples.
pub fn new_aligned_cursor() -> Cursor<Vec<u8>> {
    const INITIAL_SIZE: usize = 1024;
    Cursor::new(unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(
                std::alloc::Layout::from_size_align(INITIAL_SIZE, 128).unwrap(),
            ),
            0,
            INITIAL_SIZE,
        )
    })
}

#[test]

fn test_pad_align_to() {
    assert_eq!(7 + pad_align_to(7, 8), 8);
    assert_eq!(8 + pad_align_to(8, 8), 8);
    assert_eq!(9 + pad_align_to(9, 8), 16);
    assert_eq!(36 + pad_align_to(36, 16), 48);
}
