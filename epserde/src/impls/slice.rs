/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Convenience implementation for references to slices.
//!
//! In theory all types serialized by ε-serde must not contain references.
//! However, we provide a convenience implementation that serializes references
//! to slices as boxed slices.
//!
//! Note, however, that you must deserialize the slice as a vector, even when it
//! appears a type parameter; see the example in the [crate-level
//! documentation](crate).
//!
//! We provide a type hash for `[T]` so that it can be used in
//! [`PhantomData`](`core::marker::PhantomData`).

use core::hash::Hash;

use crate::prelude::*;
use ser::*;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

// For use with PhantomData
impl<T: TypeHash> TypeHash for [T] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        T::type_hash(hasher);
    }
}

// For use with PhantomData
impl<T: TypeHash> TypeHash for &[T] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "&[]".hash(hasher);
        T::type_hash(hasher);
    }
}

// For use with PhantomData
impl<T: TypeHash> TypeHash for &mut [T] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "&mut[]".hash(hasher);
        T::type_hash(hasher);
    }
}

unsafe impl<T> CopyType for &[T] {
    type Copy = Deep;
}

impl<T: ZeroCopy> SerHelper<Zero> for [T] {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_zero(backend, self)
    }
}

impl<T: DeepCopy + SerInner> SerHelper<Deep> for [T] {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_deep(backend, self)
    }
}

impl<T: CopyType + SerInner> SerInner for &[T]
where
    [T]: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T::SerType]>;
    const IS_ZERO_COPY: bool = false;
    const MIGHT_BE_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { SerHelper::_ser_inner(*self, backend) }
    }
}

unsafe impl<T> CopyType for &mut [T] {
    type Copy = Deep;
}

impl<T: CopyType + SerInner> SerInner for &mut [T]
where
    [T]: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T::SerType]>;
    const IS_ZERO_COPY: bool = false;
    const MIGHT_BE_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { SerHelper::_ser_inner(&**self, backend) }
    }
}
