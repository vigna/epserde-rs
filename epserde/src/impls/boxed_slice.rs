/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for boxed slices.

*/
use crate::prelude::*;
use core::hash::Hash;
use des::*;
use ser::*;

impl<T> CopyType for Box<[T]> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for [T] {
    fn type_hash(
        type_hasher: &mut impl core::hash::Hasher,
        repr_hasher: &mut impl core::hash::Hasher,
        offset_of: &mut usize,
    ) {
        "Box[]".hash(type_hasher);
        T::type_hash(type_hasher, repr_hasher, offset_of);
    }
}

impl<T: CopyType + TypeHash + SerializeInner> SerializeInner for Box<[T]>
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

impl<T: DeepCopy + SerializeInner> SerializeHelper<Deep> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice(self)
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: DeserializeInner + CopyType + TypeHash + 'static> DeserializeInner for Box<[T]>
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

impl<T: DeepCopy + DeserializeInner + 'static> DeserializeHelper<Deep> for Box<[T]> {
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
