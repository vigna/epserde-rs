/*
 * SPDX-FileCopyrightText: 2023 Tommaso Fontana
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementation for structures from the standard library.
//!
//! Note that none of this types can be zero-copy (unless they are empty, as in
//! the case of [`RangeFull`](core::ops::RangeFull)), because they are not
//! `repr(C)`.
//!
use ser::WriteWithNames;

use crate::prelude::*;
use core::{
    hash::Hash,
    ops::{Bound, RangeBounds},
};
use std::collections::hash_map::DefaultHasher;

// This implementation makes it possible to serialize
// PhantomData<DefaultHasher>.

impl TypeHash for DefaultHasher {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "std::hash::DefaultHasher".hash(hasher);
    }
}

macro_rules! impl_ranges {
    ($ty:ident) => {
        unsafe impl<Idx: ZeroCopy> CopyType for core::ops::$ty<Idx> {
            type Copy = Deep;
        }

        impl<Idx: ZeroCopy + TypeHash> TypeHash for core::ops::$ty<Idx> {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                stringify!(core::ops::$ty).hash(hasher);
                Idx::type_hash(hasher);
            }
        }

        impl<Idx: ZeroCopy + AlignHash> AlignHash for core::ops::$ty<Idx> {
            fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_align_hash::<Idx>(hasher, offset_of);
                crate::traits::std_align_hash::<Idx>(hasher, offset_of);
            }
        }

        impl<Idx: MaxSizeOf> MaxSizeOf for core::ops::$ty<Idx> {
            fn max_size_of() -> usize {
                core::mem::size_of::<Self>()
            }
        }
    };
}

impl_ranges!(Range);
impl_ranges!(RangeFrom);
impl_ranges!(RangeInclusive);
impl_ranges!(RangeTo);
impl_ranges!(RangeToInclusive);

// RangeFull is a zero-sized type, so it is always zero-copy.

unsafe impl CopyType for core::ops::RangeFull {
    type Copy = Zero;
}

impl TypeHash for core::ops::RangeFull {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::RangeFull).hash(hasher);
    }
}

impl AlignHash for core::ops::RangeFull {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl MaxSizeOf for core::ops::RangeFull {
    fn max_size_of() -> usize {
        0
    }
}

impl<Idx: ZeroCopy + SerInner + TypeHash + AlignHash> SerInner for core::ops::Range<Idx> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", &self.start)?;
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: ZeroCopy + DeserInner> DeserInner for core::ops::Range<Idx> {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = unsafe { Idx::_deserialize_full_inner(backend) }?;
        let end = unsafe { Idx::_deserialize_full_inner(backend) }?;
        Ok(core::ops::Range { start, end })
    }
    type DeserType<'a> = core::ops::Range<<Idx as DeserInner>::DeserType<'a>>;
    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        let end = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        Ok(core::ops::Range { start, end })
    }
}

impl<Idx: ZeroCopy + SerInner + TypeHash + AlignHash> SerInner for core::ops::RangeFrom<Idx> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", &self.start)?;
        Ok(())
    }
}

impl<Idx: ZeroCopy + DeserInner> DeserInner for core::ops::RangeFrom<Idx> {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = unsafe { Idx::_deserialize_full_inner(backend) }?;
        Ok(core::ops::RangeFrom { start })
    }
    type DeserType<'a> = core::ops::RangeFrom<<Idx as DeserInner>::DeserType<'a>>;
    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        Ok(core::ops::RangeFrom { start })
    }
}

impl<Idx: ZeroCopy + SerInner + TypeHash + AlignHash> SerInner for core::ops::RangeInclusive<Idx> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", self.start())?;
        backend.write("end", self.end())?;
        backend.write("exhausted", &matches!(self.end_bound(), Bound::Excluded(_)))?;
        Ok(())
    }
}

impl<Idx: ZeroCopy + DeserInner> DeserInner for core::ops::RangeInclusive<Idx> {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = unsafe { Idx::_deserialize_full_inner(backend) }?;
        let end = unsafe { Idx::_deserialize_full_inner(backend) }?;
        let exhausted = unsafe { bool::_deserialize_full_inner(backend) }?;
        assert!(!exhausted, "cannot deserialize an exhausted range");
        Ok(start..=end)
    }
    type DeserType<'a> = core::ops::RangeInclusive<<Idx as DeserInner>::DeserType<'a>>;
    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        let end = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        let exhausted = unsafe { bool::_deserialize_full_inner(backend) }?;
        assert!(!exhausted, "cannot deserialize an exhausted range");
        Ok(start..=end)
    }
}

impl<Idx: ZeroCopy + SerInner + TypeHash + AlignHash> SerInner for core::ops::RangeTo<Idx> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: ZeroCopy + DeserInner> DeserInner for core::ops::RangeTo<Idx> {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let end = unsafe { Idx::_deserialize_full_inner(backend) }?;
        Ok(..end)
    }
    type DeserType<'a> = core::ops::RangeTo<<Idx as DeserInner>::DeserType<'a>>;
    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let end = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        Ok(..end)
    }
}

impl<Idx: ZeroCopy + SerInner + TypeHash + AlignHash> SerInner
    for core::ops::RangeToInclusive<Idx>
{
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: ZeroCopy + DeserInner> DeserInner for core::ops::RangeToInclusive<Idx> {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let end = unsafe { Idx::_deserialize_full_inner(backend) }?;
        Ok(..=end)
    }
    type DeserType<'a> = core::ops::RangeToInclusive<<Idx as DeserInner>::DeserType<'a>>;
    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let end = unsafe { Idx::_deserialize_eps_inner(backend) }?;
        Ok(..=end)
    }
}

impl SerInner for core::ops::RangeFull {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    unsafe fn _serialize_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

impl DeserInner for core::ops::RangeFull {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(core::ops::RangeFull)
    }
    type DeserType<'a> = core::ops::RangeFull;
    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(core::ops::RangeFull)
    }
}

unsafe impl<T: CopyType> CopyType for core::ops::Bound<T> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for core::ops::Bound<T> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::Bound).hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T> AlignHash for core::ops::Bound<T> {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl<T: SerInner + TypeHash + AlignHash> SerInner for core::ops::Bound<T> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        match self {
            core::ops::Bound::Unbounded => backend.write("Tag", &0_u8),
            core::ops::Bound::Included(val) => {
                backend.write("Tag", &1_u8)?;
                backend.write("Included", val)
            }
            core::ops::Bound::Excluded(val) => {
                backend.write("Tag", &2_u8)?;
                backend.write("Excluded", val)
            }
        }
    }
}

impl<T: DeserInner> DeserInner for core::ops::Bound<T> {
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = unsafe { u8::_deserialize_full_inner(backend) }?;
        match tag {
            0 => Ok(core::ops::Bound::Unbounded),
            1 => Ok(core::ops::Bound::Included(unsafe {
                T::_deserialize_full_inner(backend)
            }?)),
            2 => Ok(core::ops::Bound::Excluded(unsafe {
                T::_deserialize_full_inner(backend)
            }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }

    type DeserType<'a> = core::ops::Bound<<T as DeserInner>::DeserType<'a>>;

    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = unsafe { u8::_deserialize_full_inner(backend) }?;
        match tag {
            0 => Ok(core::ops::Bound::Unbounded),
            1 => Ok(core::ops::Bound::Included(unsafe {
                T::_deserialize_eps_inner(backend)
            }?)),
            2 => Ok(core::ops::Bound::Excluded(unsafe {
                T::_deserialize_eps_inner(backend)
            }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
}

unsafe impl<B: CopyType, C: CopyType> CopyType for core::ops::ControlFlow<B, C> {
    type Copy = Deep;
}

impl<B: TypeHash, C: TypeHash> TypeHash for core::ops::ControlFlow<B, C> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::ControlFlow).hash(hasher);
        B::type_hash(hasher);
        C::type_hash(hasher);
    }
}

impl<B: AlignHash, C: AlignHash> AlignHash for core::ops::ControlFlow<B, C> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        B::align_hash(hasher, &mut 0);
        C::align_hash(hasher, &mut 0);
    }
}

impl<B: SerInner + TypeHash + AlignHash, C: SerInner + TypeHash + AlignHash> SerInner
    for core::ops::ControlFlow<B, C>
{
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        match self {
            core::ops::ControlFlow::Break(br) => {
                backend.write("Tag", &0_u8)?;
                backend.write("Break", br)
            }
            core::ops::ControlFlow::Continue(val) => {
                backend.write("Tag", &1_u8)?;
                backend.write("Continue", val)
            }
        }
    }
}

impl<B: DeserInner, C: DeserInner> DeserInner for core::ops::ControlFlow<B, C> {
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = unsafe { u8::_deserialize_full_inner(backend) }?;
        match tag {
            1 => Ok(core::ops::ControlFlow::Break(unsafe {
                B::_deserialize_full_inner(backend)
            }?)),
            2 => Ok(core::ops::ControlFlow::Continue(unsafe {
                C::_deserialize_full_inner(backend)
            }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }

    type DeserType<'a> =
        core::ops::ControlFlow<<B as DeserInner>::DeserType<'a>, <C as DeserInner>::DeserType<'a>>;

    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = unsafe { u8::_deserialize_full_inner(backend) }?;
        match tag {
            1 => Ok(core::ops::ControlFlow::Break(unsafe {
                B::_deserialize_eps_inner(backend)
            }?)),
            2 => Ok(core::ops::ControlFlow::Continue(unsafe {
                C::_deserialize_eps_inner(backend)
            }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
}
