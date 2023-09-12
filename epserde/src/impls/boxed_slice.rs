/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for boxed slices.

*/
use crate::des;
use crate::des::*;
use crate::ser;
use crate::ser::*;
use crate::{CopyType, Eps, EpsCopy, TypeHash, Zero, ZeroCopy};
use core::hash::Hash;

impl<T> CopyType for Box<[T]> {
    type Copy = Eps;
}

impl<T: TypeHash> TypeHash for [T] {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        T::type_hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        core::mem::align_of::<T>().hash(hasher);
        core::mem::size_of::<T>().hash(hasher);
        T::type_repr_hash(hasher);
    }
}

impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Box<[T]>
where
    Box<[T]>: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice_zero(self)
    }
}

impl<T: EpsCopy + SerializeInner> SerializeHelper<Eps> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice(self)
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + DeserializeInner + 'static> DeserializeInner for Box<[T]>
where
    Box<[T]>: DeserializeHelper<<T as CopyType>::Copy, FullType = Box<[T]>>,
{
    type DeserType<'a> = <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_copy_inner_impl(
            backend,
        )
    }

    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(
        <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'_>,
        SliceWithPos,
    )> {
        <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_copy_inner_impl(
            backend,
        )
    }
}

impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        backend
            .deserialize_vec_full_zero()
            .map(|(v, b)| (v.into_boxed_slice(), b))
    }
    #[inline(always)]
    fn _deserialize_eps_copy_inner_impl(
        backend: SliceWithPos,
    ) -> des::Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        backend.deserialize_slice_zero()
    }
}

impl<T: EpsCopy + DeserializeInner + 'static> DeserializeHelper<Eps> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = Box<[<T as DeserializeInner>::DeserType<'a>]>;
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        backend
            .deserialize_vec_full_eps()
            .map(|(v, b)| (v.into_boxed_slice(), b))
    }
    #[inline(always)]
    fn _deserialize_eps_copy_inner_impl(
        backend: SliceWithPos,
    ) -> des::Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        backend
            .deserialize_vec_eps_eps::<T>()
            .map(|(v, b)| (v.into_boxed_slice(), b))
    }
}
