/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
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
    traits::{AlignHash, AlignTo, CopyType, TypeHash, Zero},
};

pub mod deser;
pub mod impls;
pub mod ser;
pub mod traits;
pub mod utils;

pub mod prelude {
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
pub const VERSION: (u16, u16) = (1, 1);

/// Magic cookie, also used as endian ess marker.
pub const MAGIC: u64 = u64::from_ne_bytes(*b"epserde ");
/// What we will read if the endianness is mismatched.
pub const MAGIC_REV: u64 = u64::from_le_bytes(MAGIC.to_be_bytes());

/// A 128-bit aligned type.
///
/// This is useful for creating [`AlignedCursor`](crate::utils::AlignedCursor)
/// and [`MemBackend::Memory`](crate::deser::MemBackend::Memory)
/// instances with 128-bit alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
#[repr(align(16))]
pub struct Aligned16(pub [u8; 16]);

impl Default for Aligned16 {
    fn default() -> Self {
        Aligned16([0u8; 16])
    }
}

/// A 64-bit aligned type.
///
/// This is useful for creating [`AlignedCursor`](crate::utils::AlignedCursor)
/// and [`MemBackend::Memory`](crate::deser::MemBackend::Memory)
/// instances with 64-bit alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
#[repr(align(64))]
pub struct Aligned64(pub [u8; 64]);

impl Default for Aligned64 {
    fn default() -> Self {
        Aligned64([0u8; 64])
    }
}

/// Computes the padding needed for alignment, that is, the smallest
/// number such that `((value + pad_align_to(value, align_to) & (align_to - 1) == 0`.
pub fn pad_align_to(value: usize, align_to: usize) -> usize {
    value.wrapping_neg() & (align_to - 1)
}

/// A type semantically equivalent to [`PhantomData`], but whose type parameter
/// is replaced with its associated deserialization type.
///
/// In some case, you might find yourself with a deep-copy type that has a type
/// parameter `T` appearing both in a field and in a [`PhantomData`]. In this
/// case, the type will not compile, as in its associated deserialization type
/// `T` will be replaced by `T::DeserType`, but the [`PhantomData`] field will
/// still contain `T`. To fix this issue, you can use [`PhantomDeserData`]
/// instead.
///
/// Note that `T` must be sized.
///
/// # Examples
///
/// This code will not compile:
/// ```compile_fail
/// use epserde::prelude::*;
/// #[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
/// struct Data<T> {
///     data: T,
///     phantom: PhantomData<T>,
/// }
/// ```
///
/// This code, instead, will compile:
/// ```
/// use epserde::prelude::*;
/// #[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
/// struct Data<T> {
///     data: T,
///     phantom: PhantomDeserData<T>,
/// }
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhantomDeserData<T: ?Sized>(pub PhantomData<T>);

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
        // SAFETY: types are zero-length
        Ok(unsafe {
            transmute::<DeserType<'a, PhantomDeserData<T>>, PhantomDeserData<T::DeserType<'a>>>(
                PhantomDeserData(PhantomData),
            )
        })
    }
}

unsafe impl<T> CopyType for PhantomDeserData<T> {
    type Copy = Zero;
}

impl<T> AlignTo for PhantomDeserData<T> {
    #[inline(always)]
    fn align_to() -> usize {
        0
    }
}

impl<T: TypeHash> TypeHash for PhantomDeserData<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "PhantomDeserData".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T> AlignHash for PhantomDeserData<T> {
    #[inline(always)]
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl<T> SerInner for PhantomDeserData<T> {
    // This type is nominal only; nothing will be serialized
    // or deserialized.
    type SerType = Self;
    const IS_ZERO_COPY: bool = true;

    #[inline(always)]
    unsafe fn _ser_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

impl<T: DeserInner> DeserInner for PhantomDeserData<T> {
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

#[test]

fn test_pad_align_to() {
    assert_eq!(7 + pad_align_to(7, 8), 8);
    assert_eq!(8 + pad_align_to(8, 8), 8);
    assert_eq!(9 + pad_align_to(9, 8), 16);
    assert_eq!(36 + pad_align_to(36, 16), 48);
}
