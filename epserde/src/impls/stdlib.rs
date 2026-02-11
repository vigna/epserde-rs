/*
 * SPDX-FileCopyrightText: 2023 Tommaso Fontana
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementation for structures from the standard library.
//!
//! Note that none of this types can be zero-copy (unless they are empty, as in
//! the case of [`RangeFull`], because they are not `repr(C)`.
use ser::WriteWithNames;

use crate::prelude::*;
use core::hash::{BuildHasherDefault, Hash};
use core::ops::{
    Bound, ControlFlow, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive,
};

#[cfg(feature = "std")]
use std::hash::DefaultHasher;

// This implementation makes it possible to serialize
// PhantomData<DefaultHasher>.
#[cfg(feature = "std")]
impl TypeHash for DefaultHasher {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "std::hash::DefaultHasher".hash(hasher);
    }
}

unsafe impl<H> CopyType for BuildHasherDefault<H> {
    type Copy = Zero;
}

impl<H> AlignTo for BuildHasherDefault<H> {
    fn align_to() -> usize {
        0
    }
}

impl<H: TypeHash> TypeHash for BuildHasherDefault<H> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "core::hash::BuildHasherDefault".hash(hasher);
        H::type_hash(hasher);
    }
}

impl<H> AlignHash for BuildHasherDefault<H> {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl<H> SerInner for BuildHasherDefault<H> {
    type SerType = BuildHasherDefault<H>;
    const IS_ZERO_COPY: bool = true;

    unsafe fn _ser_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

impl<H> DeserInner for BuildHasherDefault<H> {
    unsafe fn _deser_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(BuildHasherDefault::default())
    }

    type DeserType<'a> = BuildHasherDefault<H>;

    unsafe fn _deser_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(BuildHasherDefault::default())
    }
}

macro_rules! impl_ranges {
    ($ty:ident) => {
        unsafe impl<Idx> CopyType for $ty<Idx> {
            type Copy = Deep;
        }

        impl<Idx: TypeHash> TypeHash for $ty<Idx> {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                stringify!(core::ops::$ty).hash(hasher);
                Idx::type_hash(hasher);
            }
        }

        impl<Idx: AlignHash> AlignHash for $ty<Idx> {
            fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                Idx::align_hash(hasher, offset_of);
                Idx::align_hash(hasher, offset_of);
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

unsafe impl CopyType for RangeFull {
    type Copy = Zero;
}

impl TypeHash for RangeFull {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::RangeFull).hash(hasher);
    }
}

impl AlignHash for RangeFull {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl AlignTo for RangeFull {
    fn align_to() -> usize {
        0
    }
}

impl<Idx: SerInner> SerInner for Range<Idx> {
    type SerType = Range<Idx::SerType>;
    const IS_ZERO_COPY: bool = false;

    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", &self.start)?;
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: DeserInner> DeserInner for Range<Idx> {
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = unsafe { Idx::_deser_full_inner(backend) }?;
        let end = unsafe { Idx::_deser_full_inner(backend) }?;
        Ok(Range { start, end })
    }
    type DeserType<'a> = Range<DeserType<'a, Idx>>;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = unsafe { Idx::_deser_eps_inner(backend) }?;
        let end = unsafe { Idx::_deser_eps_inner(backend) }?;
        Ok(Range { start, end })
    }
}

impl<Idx: SerInner> SerInner for RangeFrom<Idx> {
    type SerType = RangeFrom<Idx::SerType>;
    const IS_ZERO_COPY: bool = false;

    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", &self.start)?;
        Ok(())
    }
}

impl<Idx: DeserInner> DeserInner for RangeFrom<Idx> {
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = unsafe { Idx::_deser_full_inner(backend) }?;
        Ok(RangeFrom { start })
    }
    type DeserType<'a> = RangeFrom<DeserType<'a, Idx>>;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = unsafe { Idx::_deser_eps_inner(backend) }?;
        Ok(RangeFrom { start })
    }
}

impl<Idx: SerInner> SerInner for RangeInclusive<Idx> {
    type SerType = RangeInclusive<Idx::SerType>;
    const IS_ZERO_COPY: bool = false;

    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", self.start())?;
        backend.write("end", self.end())?;
        backend.write("exhausted", &matches!(self.end_bound(), Bound::Excluded(_)))?;
        Ok(())
    }
}

impl<Idx: DeserInner> DeserInner for RangeInclusive<Idx> {
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = unsafe { Idx::_deser_full_inner(backend) }?;
        let end = unsafe { Idx::_deser_full_inner(backend) }?;
        let exhausted = unsafe { bool::_deser_full_inner(backend) }?;
        assert!(!exhausted, "cannot deserialize an exhausted range");
        Ok(start..=end)
    }
    type DeserType<'a> = RangeInclusive<DeserType<'a, Idx>>;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = unsafe { Idx::_deser_eps_inner(backend) }?;
        let end = unsafe { Idx::_deser_eps_inner(backend) }?;
        let exhausted = unsafe { bool::_deser_full_inner(backend) }?;
        assert!(!exhausted, "cannot deserialize an exhausted range");
        Ok(start..=end)
    }
}

impl<Idx: SerInner> SerInner for RangeTo<Idx> {
    type SerType = RangeTo<Idx::SerType>;
    const IS_ZERO_COPY: bool = false;

    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: DeserInner> DeserInner for RangeTo<Idx> {
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let end = unsafe { Idx::_deser_full_inner(backend) }?;
        Ok(..end)
    }
    type DeserType<'a> = RangeTo<DeserType<'a, Idx>>;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let end = unsafe { Idx::_deser_eps_inner(backend) }?;
        Ok(..end)
    }
}

impl<Idx: SerInner> SerInner for RangeToInclusive<Idx> {
    type SerType = RangeToInclusive<Idx::SerType>;
    const IS_ZERO_COPY: bool = false;

    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: DeserInner> DeserInner for RangeToInclusive<Idx> {
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let end = unsafe { Idx::_deser_full_inner(backend) }?;
        Ok(..=end)
    }
    type DeserType<'a> = RangeToInclusive<DeserType<'a, Idx>>;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let end = unsafe { Idx::_deser_eps_inner(backend) }?;
        Ok(..=end)
    }
}

impl SerInner for RangeFull {
    type SerType = RangeFull;
    const IS_ZERO_COPY: bool = false;

    #[inline(always)]
    unsafe fn _ser_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

impl DeserInner for RangeFull {
    #[inline(always)]
    unsafe fn _deser_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(RangeFull)
    }
    type DeserType<'a> = RangeFull;
    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(RangeFull)
    }
}

unsafe impl<T> CopyType for Bound<T> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for Bound<T> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::Bound).hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T> AlignHash for Bound<T> {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl<T: SerInner> SerInner for Bound<T> {
    type SerType = Bound<T::SerType>;
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        match self {
            Bound::Unbounded => backend.write("Tag", &0_u8),
            Bound::Included(val) => {
                backend.write("Tag", &1_u8)?;
                backend.write("Included", val)
            }
            Bound::Excluded(val) => {
                backend.write("Tag", &2_u8)?;
                backend.write("Excluded", val)
            }
        }
    }
}

impl<T: DeserInner> DeserInner for Bound<T> {
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = unsafe { u8::_deser_full_inner(backend) }?;
        match tag {
            0 => Ok(Bound::Unbounded),
            1 => Ok(Bound::Included(unsafe { T::_deser_full_inner(backend) }?)),
            2 => Ok(Bound::Excluded(unsafe { T::_deser_full_inner(backend) }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }

    type DeserType<'a> = Bound<DeserType<'a, T>>;

    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = unsafe { u8::_deser_full_inner(backend) }?;
        match tag {
            0 => Ok(Bound::Unbounded),
            1 => Ok(Bound::Included(unsafe { T::_deser_eps_inner(backend) }?)),
            2 => Ok(Bound::Excluded(unsafe { T::_deser_eps_inner(backend) }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
}

unsafe impl<B, C> CopyType for ControlFlow<B, C> {
    type Copy = Deep;
}

impl<B: TypeHash, C: TypeHash> TypeHash for ControlFlow<B, C> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::ControlFlow).hash(hasher);
        B::type_hash(hasher);
        C::type_hash(hasher);
    }
}

impl<B: AlignHash, C: AlignHash> AlignHash for ControlFlow<B, C> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        B::align_hash(hasher, &mut 0);
        C::align_hash(hasher, &mut 0);
    }
}

impl<B: SerInner, C: SerInner> SerInner for ControlFlow<B, C> {
    type SerType = ControlFlow<B::SerType, C::SerType>;
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        match self {
            ControlFlow::Break(br) => {
                backend.write("Tag", &0_u8)?;
                backend.write("Break", br)
            }
            ControlFlow::Continue(val) => {
                backend.write("Tag", &1_u8)?;
                backend.write("Continue", val)
            }
        }
    }
}

impl<B: DeserInner, C: DeserInner> DeserInner for ControlFlow<B, C> {
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = unsafe { u8::_deser_full_inner(backend) }?;
        match tag {
            0 => Ok(ControlFlow::Break(unsafe {
                B::_deser_full_inner(backend)
            }?)),
            1 => Ok(ControlFlow::Continue(unsafe {
                C::_deser_full_inner(backend)
            }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }

    type DeserType<'a> = ControlFlow<DeserType<'a, B>, DeserType<'a, C>>;

    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = unsafe { u8::_deser_full_inner(backend) }?;
        match tag {
            0 => Ok(ControlFlow::Break(unsafe { B::_deser_eps_inner(backend) }?)),
            1 => Ok(ControlFlow::Continue(unsafe {
                C::_deser_eps_inner(backend)
            }?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
}
