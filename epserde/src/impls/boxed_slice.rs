/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for boxed slices.

*/
use crate::deser::helpers::*;
use crate::prelude::*;
use core::hash::Hash;
use deser::*;
use ser::*;

impl<T> CopyType for Box<[T]> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for Box<[T]> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box<[]>".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: ReprHash> ReprHash for Box<[T]> {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        // TODO: this implemention should be empty, as all deep-copy types
        // implementations should have an empty repr_hash implementation,
        // and need not implement MaxSizeOf.
        // We keep it temporarily to avoid breaking the file format.
        *offset_of = 0;
        T::repr_hash(hasher, offset_of);
    }
}

impl<T: CopyType + SerializeInner + TypeHash + ReprHash> SerializeInner for Box<[T]>
where
    Box<[T]>: SerializeHelper<<T as CopyType>::Copy>,
{
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_slice_zero(backend, self)
    }
}

impl<T: DeepCopy + SerializeInner> SerializeHelper<Deep> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_slice_deep(backend, self)
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: DeserializeInner + CopyType> DeserializeInner for Box<[T]>
where
    Box<[T]>: DeserializeHelper<<T as CopyType>::Copy, FullType = Box<[T]>>,
{
    type DeserType<'a> = <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_inner_impl(
            backend,
        )
    }

    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>> {
        <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_inner_impl(backend)
    }
}

impl<T: ZeroCopy + DeserializeInner> DeserializeHelper<Zero> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(deserialize_full_vec_zero::<T>(backend)?.into_boxed_slice())
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        deserialize_eps_slice_zero(backend)
    }
}

impl<T: DeepCopy + DeserializeInner> DeserializeHelper<Deep> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = Box<[<T as DeserializeInner>::DeserType<'a>]>;
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(deserialize_full_vec_deep(backend)?.into_boxed_slice())
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        Ok(deserialize_eps_vec_deep::<T>(backend)?.into_boxed_slice())
    }
}
