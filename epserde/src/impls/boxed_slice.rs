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
use deser::*;
use ser::*;

impl<T> CopyType for Box<[T]> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for [T] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box[]".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: CopyType + TypeHash + SerializeInner> SerializeInner for Box<[T]>
where
    Box<[T]>: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> ser::Result<()> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> ser::Result<()> {
        backend.write_slice_zero(self)
    }
}

impl<T: DeepCopy + SerializeInner> SerializeHelper<Deep> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> ser::Result<()> {
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

impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(backend.deserialize_vec_full_zero()?.into_boxed_slice())
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        backend.deserialize_slice_zero()
    }
}

impl<T: DeepCopy + DeserializeInner + 'static> DeserializeHelper<Deep> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = Box<[<T as DeserializeInner>::DeserType<'a>]>;
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(backend.deserialize_vec_full_eps()?.into_boxed_slice())
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        Ok(backend.deserialize_vec_eps_eps::<T>()?.into_boxed_slice())
    }
}
