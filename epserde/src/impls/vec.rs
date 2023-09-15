/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for vectors.

*/
use crate::des;
use crate::des::*;
use crate::ser;
use crate::ser::*;
use crate::traits::*;
use core::hash::Hash;

impl<T> CopyType for Vec<T> {
    type Copy = Full;
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
impl<T: TypeHash> TypeHash for Vec<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Vec".hash(hasher);
        T::type_hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        // We align to the size of T.
        core::mem::size_of::<T>().hash(hasher);
        T::type_repr_hash(hasher);
    }
}

impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Vec<T>
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Vec<T> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice_zero(self.as_slice())
    }
}

impl<T: FullCopy + SerializeInner> SerializeHelper<Full> for Vec<T> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice(self.as_slice())
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + DeserializeInner + 'static> DeserializeInner for Vec<T>
where
    Vec<T>: DeserializeHelper<<T as CopyType>::Copy, FullType = Vec<T>>,
{
    type DeserType<'a> = <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_copy_inner_impl(
            backend,
        )
    }

    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(
        <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'_>,
        SliceWithPos,
    )> {
        <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_copy_inner_impl(
            backend,
        )
    }
}

impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero> for Vec<T> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        backend.deserialize_vec_full_zero()
    }
    #[inline(always)]
    fn _deserialize_eps_copy_inner_impl(
        backend: SliceWithPos,
    ) -> des::Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        backend.deserialize_slice_zero()
    }
}

impl<T: FullCopy + DeserializeInner + 'static> DeserializeHelper<Full> for Vec<T> {
    type FullType = Self;
    type DeserType<'a> = Vec<<T as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        backend.deserialize_vec_full_eps()
    }
    #[inline(always)]
    fn _deserialize_eps_copy_inner_impl(
        backend: SliceWithPos,
    ) -> des::Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        backend.deserialize_vec_eps_eps::<T>()
    }
}
