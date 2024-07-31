/*
 * SPDX-FileCopyrightText: 2023 Tommaso Fontana
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementation of traits for struts from the std library
use ser::WriteWithNames;

use crate::prelude::*;
use core::{
    hash::Hash,
    ops::{Bound, RangeBounds},
};
use std::collections::hash_map::DefaultHasher;

impl TypeHash for DefaultHasher {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "std::hash::DefaultHasher".hash(hasher);
    }
}

impl ReprHash for DefaultHasher {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        crate::traits::std_repr_hash::<Self>(hasher, offset_of)
    }
}

impl MaxSizeOf for DefaultHasher {
    fn max_size_of() -> usize {
        core::mem::size_of::<Self>()
    }
}

macro_rules! impl_ranges {
    ($ty:ident) => {
        impl<Idx: CopyType> CopyType for core::ops::$ty<Idx> {
            type Copy = Zero;
        }

        impl<Idx: TypeHash> TypeHash for core::ops::$ty<Idx> {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                stringify!(core::ops::$ty).hash(hasher);
                Idx::type_hash(hasher);
            }
        }

        impl<Idx: ReprHash> ReprHash for core::ops::$ty<Idx> {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
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

impl<Idx: SerializeInner> SerializeInner for core::ops::Range<Idx> {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", &self.start)?;
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: DeserializeInner> DeserializeInner for core::ops::Range<Idx> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = Idx::_deserialize_full_inner(backend)?;
        let end = Idx::_deserialize_full_inner(backend)?;
        Ok(core::ops::Range { start, end })
    }
    type DeserType<'a> = core::ops::Range<<Idx as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = Idx::_deserialize_eps_inner(backend)?;
        let end = Idx::_deserialize_eps_inner(backend)?;
        Ok(core::ops::Range { start, end })
    }
}

impl<Idx: SerializeInner> SerializeInner for core::ops::RangeFrom<Idx> {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", &self.start)?;
        Ok(())
    }
}

impl<Idx: DeserializeInner> DeserializeInner for core::ops::RangeFrom<Idx> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = Idx::_deserialize_full_inner(backend)?;
        Ok(core::ops::RangeFrom { start })
    }
    type DeserType<'a> = core::ops::RangeFrom<<Idx as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = Idx::_deserialize_eps_inner(backend)?;
        Ok(core::ops::RangeFrom { start })
    }
}

impl<Idx: SerializeInner> SerializeInner for core::ops::RangeInclusive<Idx> {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("start", self.start())?;
        backend.write("end", self.end())?;
        backend.write("exhausted", &matches!(self.end_bound(), Bound::Excluded(_)))?;
        Ok(())
    }
}

impl<Idx: DeserializeInner> DeserializeInner for core::ops::RangeInclusive<Idx> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let start = Idx::_deserialize_full_inner(backend)?;
        let end = Idx::_deserialize_full_inner(backend)?;
        let exhausted = bool::_deserialize_full_inner(backend)?;
        assert!(!exhausted, "cannot deserialize an exhausted range");
        Ok(start..=end)
    }
    type DeserType<'a> = core::ops::RangeInclusive<<Idx as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let start = Idx::_deserialize_eps_inner(backend)?;
        let end = Idx::_deserialize_eps_inner(backend)?;
        let exhausted = bool::_deserialize_full_inner(backend)?;
        assert!(!exhausted, "cannot deserialize an exhausted range");
        return Ok(start..=end);
    }
}

impl<Idx: SerializeInner> SerializeInner for core::ops::RangeTo<Idx> {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: DeserializeInner> DeserializeInner for core::ops::RangeTo<Idx> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let end = Idx::_deserialize_full_inner(backend)?;
        Ok(..end)
    }
    type DeserType<'a> = core::ops::RangeTo<<Idx as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let end = Idx::_deserialize_eps_inner(backend)?;
        Ok(..end)
    }
}

impl<Idx: SerializeInner> SerializeInner for core::ops::RangeToInclusive<Idx> {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        backend.write("end", &self.end)?;
        Ok(())
    }
}

impl<Idx: DeserializeInner> DeserializeInner for core::ops::RangeToInclusive<Idx> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let end = Idx::_deserialize_full_inner(backend)?;
        Ok(..=end)
    }
    type DeserType<'a> = core::ops::RangeToInclusive<<Idx as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let end = Idx::_deserialize_eps_inner(backend)?;
        Ok(..=end)
    }
}

impl CopyType for core::ops::RangeFull {
    type Copy = Zero;
}

impl TypeHash for core::ops::RangeFull {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::RangeFull).hash(hasher);
    }
}

impl ReprHash for core::ops::RangeFull {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        crate::traits::std_repr_hash::<Self>(hasher, offset_of)
    }
}

impl MaxSizeOf for core::ops::RangeFull {
    fn max_size_of() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl SerializeInner for core::ops::RangeFull {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

impl DeserializeInner for core::ops::RangeFull {
    #[inline(always)]
    fn _deserialize_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(core::ops::RangeFull)
    }
    type DeserType<'a> = core::ops::RangeFull;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(core::ops::RangeFull)
    }
}

impl<T: CopyType> CopyType for core::ops::Bound<T> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for core::ops::Bound<T> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::Bound).hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: ReprHash> ReprHash for core::ops::Bound<T> {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        crate::traits::std_repr_hash::<Self>(hasher, offset_of);
        T::repr_hash(hasher, offset_of);
    }
}

impl<T: MaxSizeOf> MaxSizeOf for core::ops::Bound<T> {
    fn max_size_of() -> usize {
        core::mem::size_of::<Self>() - core::mem::size_of::<T>() + T::max_size_of()
    }
}

impl<T: SerializeInner> SerializeInner for core::ops::Bound<T> {
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
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

impl<T: DeserializeInner> DeserializeInner for core::ops::Bound<T> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = u8::_deserialize_full_inner(backend)?;
        match tag {
            0 => Ok(core::ops::Bound::Unbounded),
            1 => Ok(core::ops::Bound::Included(T::_deserialize_full_inner(
                backend,
            )?)),
            2 => Ok(core::ops::Bound::Excluded(T::_deserialize_full_inner(
                backend,
            )?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
    type DeserType<'a> = core::ops::Bound<<T as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = u8::_deserialize_full_inner(backend)?;
        match tag {
            0 => Ok(core::ops::Bound::Unbounded),
            1 => Ok(core::ops::Bound::Included(T::_deserialize_eps_inner(
                backend,
            )?)),
            2 => Ok(core::ops::Bound::Excluded(T::_deserialize_eps_inner(
                backend,
            )?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
}

impl<B: CopyType, C: CopyType> CopyType for core::ops::ControlFlow<B, C> {
    type Copy = Deep;
}

impl<B: TypeHash, C: TypeHash> TypeHash for core::ops::ControlFlow<B, C> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!(core::ops::ControlFlow).hash(hasher);
        B::type_hash(hasher);
        C::type_hash(hasher);
    }
}

impl<B: ReprHash, C: ReprHash> ReprHash for core::ops::ControlFlow<B, C> {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        crate::traits::std_repr_hash::<Self>(hasher, offset_of);
        B::repr_hash(hasher, offset_of);
        C::repr_hash(hasher, offset_of);
    }
}

impl<B: MaxSizeOf, C: MaxSizeOf> MaxSizeOf for core::ops::ControlFlow<B, C> {
    fn max_size_of() -> usize {
        let b_max_size = B::max_size_of() - core::mem::size_of::<B>();
        let c_max_size = C::max_size_of() - core::mem::size_of::<C>();
        core::mem::size_of::<Self>() + b_max_size.max(c_max_size)
    }
}

impl<B: SerializeInner, C: SerializeInner> SerializeInner for core::ops::ControlFlow<B, C> {
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
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

impl<B: DeserializeInner, C: DeserializeInner> DeserializeInner for core::ops::ControlFlow<B, C> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = u8::_deserialize_full_inner(backend)?;
        match tag {
            1 => Ok(core::ops::ControlFlow::Break(B::_deserialize_full_inner(
                backend,
            )?)),
            2 => Ok(core::ops::ControlFlow::Continue(
                C::_deserialize_full_inner(backend)?,
            )),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
    type DeserType<'a> = core::ops::ControlFlow<
        <B as DeserializeInner>::DeserType<'a>,
        <C as DeserializeInner>::DeserType<'a>,
    >;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = u8::_deserialize_full_inner(backend)?;
        match tag {
            1 => Ok(core::ops::ControlFlow::Break(B::_deserialize_eps_inner(
                backend,
            )?)),
            2 => Ok(core::ops::ControlFlow::Continue(C::_deserialize_eps_inner(
                backend,
            )?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
}
