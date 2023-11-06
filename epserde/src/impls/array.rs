/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for arrays.

*/
use crate::prelude::*;
use core::hash::Hash;
use core::mem::MaybeUninit;
use deser::*;
use ser::*;

impl<T: CopyType, const N: usize> CopyType for [T; N] {
    type Copy = T::Copy;
}

impl<T: TypeHash, const N: usize> TypeHash for [T; N] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        hasher.write_usize(N);
        T::type_hash(hasher);
    }
}

impl<T: Sized, const N: usize> ReprHash for [T; N] {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        crate::traits::std_repr_hash::<Self>(hasher, offset_of)
    }
}

impl<T: MaxSizeOf, const N: usize> MaxSizeOf for [T; N] {
    fn max_size_of() -> usize {
        T::max_size_of()
    }
}

impl<T: CopyType + SerializeInner + TypeHash, const N: usize> SerializeInner for [T; N]
where
    [T; N]: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = T::IS_ZERO_COPY;
    const ZERO_COPY_MISMATCH: bool = T::ZERO_COPY_MISMATCH;
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner + TypeHash, const N: usize> SerializeHelper<Zero> for [T; N] {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_zero(backend, self)
    }
}

impl<T: DeepCopy + SerializeInner, const N: usize> SerializeHelper<Deep> for [T; N] {
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        for item in self.iter() {
            backend.write("item", item)?;
        }
        Ok(())
    }
}

impl<T: CopyType + DeserializeInner + 'static, const N: usize> DeserializeInner for [T; N]
where
    [T; N]: DeserializeHelper<<T as CopyType>::Copy, FullType = [T; N]>,
{
    type DeserType<'a> = <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_full_inner_impl(backend)
    }

    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>> {
        <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::_deserialize_eps_inner_impl(backend)
    }
}

impl<T: ZeroCopy + DeserializeInner + 'static, const N: usize> DeserializeHelper<Zero> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = &'a [T; N];
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let mut res = MaybeUninit::<[T; N]>::uninit();
        backend.align::<T>()?;
        // SAFETY: read_exact guarantees that the array will be filled with data.
        unsafe {
            backend.read_exact(res.assume_init_mut().align_to_mut::<u8>().1)?;
            Ok(res.assume_init())
        }
    }
    #[inline(always)]

    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        backend.align::<T>()?;
        let bytes = std::mem::size_of::<[T; N]>();
        let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<[T; N]>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        let res = &data[0];
        backend.skip(bytes);
        Ok(res)
    }
}

impl<T: DeepCopy + DeserializeInner + 'static, const N: usize> DeserializeHelper<Deep> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = [<T as DeserializeInner>::DeserType<'a>; N];
    #[inline(always)]
    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let mut res = MaybeUninit::<[T; N]>::uninit();
        unsafe {
            for item in &mut res.assume_init_mut().iter_mut() {
                std::ptr::write(item, T::_deserialize_full_inner(backend)?);
            }
            Ok(res.assume_init())
        }
    }
    #[inline(always)]
    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
        let mut res = MaybeUninit::<<Self as DeserializeInner>::DeserType<'_>>::uninit();
        unsafe {
            for item in &mut res.assume_init_mut().iter_mut() {
                std::ptr::write(item, T::_deserialize_eps_inner(backend)?);
            }
            Ok(res.assume_init())
        }
    }
}
