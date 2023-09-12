/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for arrays.

*/
use core::mem::MaybeUninit;

use crate::des::{self, DeserializeHelper};
use crate::ser;
use crate::ser::SerializeHelper;
use crate::{
    CopyType, DeserializeInner, FieldWrite, Full, FullCopy, ReadWithPos, SerializeInner,
    SliceWithPos, TypeHash, Zero, ZeroCopy,
};
use core::hash::Hash;

impl<T: CopyType, const N: usize> CopyType for [T; N] {
    type Copy = T::Copy;
}

impl<T: TypeHash, const N: usize> TypeHash for [T; N] {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[;]".hash(hasher);
        T::type_hash(hasher);
        N.hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        core::mem::align_of::<Self>().hash(hasher);
        core::mem::size_of::<Self>().hash(hasher);
        T::type_repr_hash(hasher);
    }
}

impl<T: CopyType + SerializeInner + TypeHash, const N: usize> SerializeInner for [T; N]
where
    [T; N]: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = T::IS_ZERO_COPY;
    const ZERO_COPY_MISMATCH: bool = T::ZERO_COPY_MISMATCH;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner, const N: usize> SerializeHelper<Zero> for [T; N] {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_field_zero("items", self)
    }
}

impl<T: FullCopy + SerializeInner, const N: usize> SerializeHelper<Full> for [T; N] {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> ser::Result<F> {
        for item in self.iter() {
            backend = backend.write_field("item", item)?;
        }
        Ok(backend)
    }
}

impl<T: CopyType + DeserializeInner + 'static, const N: usize> DeserializeInner for [T; N]
where
    [T; N]: DeserializeHelper<<T as CopyType>::Copy, FullType = [T; N]>,
{
    type DeserType<'a> = <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<([T; N], R)> {
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_copy_inner_impl(
            backend,
        )
    }

    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'_>,
        SliceWithPos,
    )> {
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_copy_inner_impl(
            backend,
        )
    }
}

impl<T: ZeroCopy + DeserializeInner + 'static, const N: usize> DeserializeHelper<Zero> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = &'a [T; N];
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(mut backend: R) -> des::Result<(Self, R)> {
        backend = backend.align::<T>()?;
        let mut res = MaybeUninit::<[T; N]>::uninit();
        // SAFETY: read_exact guarantees that the array will be filled with data.
        unsafe {
            backend.read_exact(res.assume_init_mut().align_to_mut::<u8>().1)?;
            Ok((res.assume_init(), backend))
        }
    }
    #[inline(always)]

    fn _deserialize_eps_copy_inner_impl(
        mut backend: SliceWithPos,
    ) -> des::Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        let bytes = std::mem::size_of::<[T; N]>();
        backend = backend.align::<T>()?;
        let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<[T; N]>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        Ok((&data[0], backend.skip(bytes)))
    }
}

impl<T: FullCopy + DeserializeInner + 'static, const N: usize> DeserializeHelper<Full> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = [<T as DeserializeInner>::DeserType<'a>; N];
    #[inline(always)]
    fn _deserialize_full_copy_inner_impl<R: ReadWithPos>(mut backend: R) -> des::Result<(Self, R)> {
        let mut res = MaybeUninit::<[T; N]>::uninit();
        unsafe {
            for item in &mut res.assume_init_mut().iter_mut() {
                let (elem, new_backend) = T::_deserialize_full_copy_inner(backend)?;
                std::ptr::write(item, elem);
                backend = new_backend;
            }
            Ok((res.assume_init(), backend))
        }
    }
    #[inline(always)]
    fn _deserialize_eps_copy_inner_impl(
        mut backend: SliceWithPos,
    ) -> des::Result<(<Self as DeserializeInner>::DeserType<'_>, SliceWithPos)> {
        let mut res = MaybeUninit::<<Self as DeserializeInner>::DeserType<'_>>::uninit();
        unsafe {
            for item in &mut res.assume_init_mut().iter_mut() {
                let (elem, new_backend) = T::_deserialize_eps_copy_inner(backend)?;
                std::ptr::write(item, elem);
                backend = new_backend;
            }
            Ok((res.assume_init(), backend))
        }
    }
}
