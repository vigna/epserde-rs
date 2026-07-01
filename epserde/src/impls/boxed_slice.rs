/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for boxed slices.

use crate::deser::helpers::*;
use crate::prelude::*;
use core::hash::Hash;
use deser::*;
use ser::*;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

unsafe impl<T> CopyType for Box<[T]> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for Box<[T]> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box<[]>".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: AlignHash> AlignHash for Box<[T]> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        T::align_hash(hasher, &mut 0);
    }
}

impl<T: CopyType + SerInner> SerInner for Box<[T]>
where
    Box<[T]>: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T::SerType]>;
    const IS_ZERO_COPY: bool = false;
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { SerHelper::_ser_inner(self, backend) }
    }
}

impl<T: ZeroCopy> SerHelper<Zero> for Box<[T]> {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_zero(backend, self)
    }
}

impl<T: DeepCopy> SerHelper<Deep> for Box<[T]> {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_deep(backend, self)
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + DeserInner> DeserInner for Box<[T]>
where
    Box<[T]>: DeserHelper<<T as CopyType>::Copy, FullType = Box<[T]>>,
{
    type DeserType<'a> = <Box<[T]> as DeserHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    // SAFETY: In the Zero case, DeserType<'a> = &'a [T], which is covariant.
    // In the Deep case, DeserType<'a> = Box<[T::DeserType<'a>]>; Box and
    // slices are covariant, and T::DeserType is covariant
    // (enforced by T's own __check_covariance).
    crate::unsafe_assume_covariance!(T);
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        unsafe { <Box<[T]> as DeserHelper<<T as CopyType>::Copy>>::_deser_full_inner_impl(backend) }
    }

    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<Box<[T]> as DeserHelper<<T as CopyType>::Copy>>::DeserType<'a>> {
        unsafe { <Box<[T]> as DeserHelper<<T as CopyType>::Copy>>::_deser_eps_inner_impl(backend) }
    }
}

impl<T: ZeroCopy + DeserInner> DeserHelper<Zero> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(unsafe { deser_full_vec_zero::<T>(backend) }?.into_boxed_slice())
    }
    #[inline(always)]
    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<DeserType<'a, Self>> {
        unsafe { deser_eps_slice_zero(backend) }
    }
}

impl<T: DeepCopy + DeserInner> DeserHelper<Deep> for Box<[T]> {
    type FullType = Self;
    type DeserType<'a> = Box<[DeserType<'a, T>]>;
    #[inline(always)]
    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(deser_full_vec_deep(backend)?.into_boxed_slice())
    }
    #[inline(always)]
    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<DeserType<'a, Self>> {
        Ok(deser_eps_vec_deep::<T>(backend)?.into_boxed_slice())
    }
}
