/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for vectors.

use crate::deser;
use crate::deser::helpers::*;
use crate::deser::*;
use crate::ser;
use crate::ser::helpers::*;
use crate::ser::*;
use crate::traits::*;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

unsafe impl<T> CopyType for Vec<T> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for Vec<T> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        <Box<[T]>>::type_hash(hasher);
    }
}

impl<T: AlignHash> AlignHash for Vec<T> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        T::align_hash(hasher, &mut 0);
    }
}

impl<T: CopyType + SerInner + TypeHash + AlignHash> SerInner for Vec<T>
where
    Vec<T>: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T]>;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { SerHelper::_ser_inner(self, backend) }
    }
}

impl<T: ZeroCopy + SerInner> SerHelper<Zero> for Vec<T> {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_zero(backend, self.as_slice())
    }
}

impl<T: DeepCopy + SerInner> SerHelper<Deep> for Vec<T> {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_deep(backend, self.as_slice())
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + DeserInner> DeserInner for Vec<T>
where
    Vec<T>: DeserHelper<<T as CopyType>::Copy, FullType = Vec<T>>,
{
    type DeserType<'a> = <Vec<T> as DeserHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        unsafe { <Vec<T> as DeserHelper<<T as CopyType>::Copy>>::_deser_full_inner_impl(backend) }
    }

    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Vec<T> as DeserHelper<<T as CopyType>::Copy>>::DeserType<'a>> {
        unsafe { <Vec<T> as DeserHelper<<T as CopyType>::Copy>>::_deser_eps_inner_impl(backend) }
    }
}

impl<T: ZeroCopy + DeserInner> DeserHelper<Zero> for Vec<T> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        unsafe { deser_full_vec_zero(backend) }
    }
    #[inline(always)]
    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<DeserType<'a, Self>> {
        unsafe { deser_eps_slice_zero(backend) }
    }
}

impl<T: DeepCopy + DeserInner> DeserHelper<Deep> for Vec<T> {
    type FullType = Self;
    type DeserType<'a> = Vec<DeserType<'a, T>>;
    #[inline(always)]
    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        deser_full_vec_deep::<T>(backend)
    }
    #[inline(always)]
    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<DeserType<'a, Self>> {
        deser_eps_vec_deep::<T>(backend)
    }
}
