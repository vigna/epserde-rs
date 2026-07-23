/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![cfg_attr(any(all(feature="std", feature="mmap"), not(doctest)), doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]
#![deny(unconditional_recursion)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
extern crate alloc;

use core::{hash::Hash, marker::PhantomData, mem::transmute};

#[cfg(feature = "derive")]
pub use epserde_derive::{Epserde, TypeInfo};

use crate::{
    deser::{DeserInner, DeserType, ReadWithPos, SliceWithPos},
    ser::{SerInner, WriteWithNames},
    traits::{AlignHash, CopyType, PadTo, TypeHash, Zero},
};

pub mod deser;
pub mod impls;
pub mod ser;
pub mod traits;
pub mod utils;

/// Re-exports of the traits, types, and macros commonly needed to use ε-serde.
///
/// Glob-import it (`use epserde::prelude::*;`) to bring the (de)serialization
/// traits and helpers into scope.
pub mod prelude {
    #[allow(deprecated)]
    pub use crate::PhantomDeserData;
    pub use crate::deser;
    pub use crate::deser::DeserHelper;
    pub use crate::deser::DeserInner;
    pub use crate::deser::DeserType;
    pub use crate::deser::Deserialize;
    pub use crate::deser::Flags;
    pub use crate::deser::MemCase;
    pub use crate::deser::ReadWithPos;
    pub use crate::deser::SliceWithPos;
    pub use crate::impls::iter::SerIter;
    pub use crate::ser;
    pub use crate::ser::SerHelper;
    pub use crate::ser::SerInner;
    pub use crate::ser::Serialize;
    pub use crate::traits::*;
    #[allow(unused_imports)] // with some features utils is empty
    pub use crate::utils::*;
    #[cfg(feature = "derive")]
    pub use epserde_derive::Epserde;
    pub use {crate::Aligned16, crate::Aligned64};
}

/// (Major, Minor) version of the file format, this follows semantic versioning
pub const VERSION: (u16, u16) = (2, 0);

/// Magic cookie, also used as endianness marker.
///
/// The value is defined with a fixed (little-endian) byte order, so it is the
/// same number on every platform. Since it is serialized in native byte order,
/// a file written on a platform with the opposite endianness is read back as
/// [`MAGIC_REV`]. Defining it with `from_ne_bytes` would instead make the
/// marker ineffective: the two native conversions would cancel out, every
/// platform would write the same bytes, and every platform would read them
/// back as its own [`MAGIC`].
pub const MAGIC: u64 = u64::from_le_bytes(*b"epserde ");
/// What we will read if the endianness is mismatched.
pub const MAGIC_REV: u64 = MAGIC.swap_bytes();

/// A 16-byte (128-bit) aligned type.
///
/// This is useful for creating [`AlignedCursor`] and [`MemBackend::Memory`]
/// instances with 128-bit alignment.
///
/// [`AlignedCursor`]: crate::utils::AlignedCursor
/// [`MemBackend::Memory`]: crate::deser::MemBackend::Memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
#[cfg_attr(feature = "mem_dbg", mem_size(flat))]
#[repr(align(16))]
#[derive(Default)]
pub struct Aligned16(pub [u8; 16]);

/// A 64-byte (512-bit) aligned type.
///
/// This is useful for creating [`AlignedCursor`] and [`MemBackend::Memory`]
/// instances with 512-bit alignment.
///
/// [`AlignedCursor`]: crate::utils::AlignedCursor
/// [`MemBackend::Memory`]: crate::deser::MemBackend::Memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
#[cfg_attr(feature = "mem_dbg", mem_size(flat))]
#[repr(align(64))]
pub struct Aligned64(pub [u8; 64]);

impl Default for Aligned64 {
    fn default() -> Self {
        Aligned64([0u8; 64])
    }
}

/// Computes the padding needed for alignment, that is, the smallest
/// number such that `value + pad_align_to(value, pad_to)` is a multiple
/// of `pad_to`.
///
/// A `pad_to` equal to zero (the [`PadTo`] value of
/// zero-sized types) requests no alignment and returns zero.
///
/// `pad_to` must be zero or a power of two, otherwise this function panics.
pub const fn pad_align_to(value: usize, pad_to: usize) -> usize {
    assert!(pad_to == 0 || pad_to.is_power_of_two());
    value.wrapping_neg() & pad_to.saturating_sub(1)
}

/// **Deprecated.** Use plain [`PhantomData`] instead.
///
/// This type used to be a workaround for the case where a deep-copy
/// `Epserde`-derived struct had a type parameter `T` appearing both in a field
/// and in a [`PhantomData<T>`] field, which would otherwise fail to compile
/// because [`PhantomData<T>`] does not substitute its parameter. The `Epserde`
/// derive now handles [`PhantomData<T>`] natively, substituting `T` inside the
/// derived `Self::DeserType<'_>`, so [`PhantomDeserData`] is no longer needed
/// for new code.
///
/// Migrating an existing struct from [`PhantomDeserData<T>`] to
/// [`PhantomData<T>`] changes the struct's type hash, so previously-serialized
/// files will fail to deserialize against the new definition; re-serialize the
/// data after migration.
///
/// Note that `T` must be sized because of a trait bound on [`DeserInner`].
#[deprecated(
    since = "0.13.0",
    note = "use plain `PhantomData<T>` instead: the `Epserde` derive now substitutes \
its parameter natively. Note: switching an existing struct from \
`PhantomDeserData<T>` to `PhantomData<T>` changes the struct's type hash, so \
previously-serialized files will fail to deserialize against the new definition."
)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhantomDeserData<T>(pub PhantomData<T>);

#[allow(deprecated)]
impl<T: DeserInner> PhantomDeserData<T> {
    /// A custom deserialization method for [`PhantomDeserData`] that transmutes
    /// the inner type.
    ///
    /// # Safety
    ///
    /// See [`DeserInner::_deser_eps_inner`].
    #[inline(always)]
    pub unsafe fn _deser_eps_inner_special<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<PhantomDeserData<T::DeserType<'a>>> {
        // SAFETY: it is a zero-sized type
        Ok(unsafe {
            transmute::<DeserType<'a, PhantomDeserData<T>>, PhantomDeserData<T::DeserType<'a>>>(
                PhantomDeserData(PhantomData),
            )
        })
    }
}

#[allow(deprecated)]
unsafe impl<T> CopyType for PhantomDeserData<T> {
    type Copy = Zero;
}

#[allow(deprecated)]
impl<T> PadTo for PhantomDeserData<T> {
    #[inline(always)]
    fn pad_to() -> usize {
        0
    }
}

#[allow(deprecated)]
impl<T: TypeHash> TypeHash for PhantomDeserData<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "PhantomDeserData".hash(hasher);
        T::type_hash(hasher);
    }
}

#[allow(deprecated)]
impl<T> AlignHash for PhantomDeserData<T> {
    #[inline(always)]
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

#[allow(deprecated)]
impl<T> SerInner for PhantomDeserData<T> {
    // This type is nominal only; nothing will be serialized or deserialized.
    type SerType = Self;
    const IS_ZERO_COPY: bool = true;

    #[inline(always)]
    unsafe fn _ser_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

#[allow(deprecated)]
impl<T: DeserInner> DeserInner for PhantomDeserData<T> {
    // SAFETY: it is a zero-sized type
    unsafe_assume_covariance!();
    #[inline(always)]
    unsafe fn _deser_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(PhantomDeserData(PhantomData))
    }
    type DeserType<'a> = PhantomDeserData<T::DeserType<'a>>;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(PhantomDeserData(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::pad_align_to;

    #[test]
    fn test_pad_align_to() {
        assert_eq!(7 + pad_align_to(7, 8), 8);
        assert_eq!(8 + pad_align_to(8, 8), 8);
        assert_eq!(9 + pad_align_to(9, 8), 16);
        assert_eq!(36 + pad_align_to(36, 16), 48);
    }

    #[test]
    #[should_panic]
    fn test_pad_align_to_rejects_non_power_of_two() {
        let _ = pad_align_to(2, 3);
    }
}
