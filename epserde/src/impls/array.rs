/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for arrays.

use crate::prelude::*;
use core::hash::Hash;
use core::mem::MaybeUninit;
use deser::*;
use ser::*;

unsafe impl<T: CopyType, const N: usize> CopyType for [T; N] {
    type Copy = T::Copy;
}

impl<T: TypeHash, const N: usize> TypeHash for [T; N] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        hasher.write_usize(N);
        T::type_hash(hasher);
    }
}

impl<T: AlignHash, const N: usize> AlignHash for [T; N] {
    fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        if N == 0 {
            return;
        }
        T::align_hash(hasher, offset_of);
        *offset_of += (N - 1) * size_of::<T>();
    }
}

impl<T: AlignTo, const N: usize> AlignTo for [T; N] {
    fn align_to() -> usize {
        T::align_to()
    }
}

impl<T: CopyType + SerInner, const N: usize> SerInner for [T; N]
where
    [T; N]: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = [T::SerType; N];
    const IS_ZERO_COPY: bool = T::IS_ZERO_COPY;
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { SerHelper::_ser_inner(self, backend) }
    }
}

impl<T: ZeroCopy, const N: usize> SerHelper<Zero> for [T; N] {
    #[inline(always)]
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_zero(backend, self)
    }
}

impl<T: DeepCopy, const N: usize> SerHelper<Deep> for [T; N] {
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        for item in self.iter() {
            backend.write("item", item)?;
        }
        Ok(())
    }
}

impl<T: CopyType + DeserInner, const N: usize> DeserInner for [T; N]
where
    [T; N]: DeserHelper<<T as CopyType>::Copy, FullType = [T; N]>,
{
    type DeserType<'a> = <[T; N] as DeserHelper<<T as CopyType>::Copy>>::DeserType<'a>;
    // SAFETY: In the Zero case, DeserType<'a> = &'a [T; N], which is covariant.
    // In the Deep case, DeserType<'a> = [T::DeserType<'a>; N]; arrays are
    // covariant in their element type, and T::DeserType is covariant
    // (enforced by T's own __check_covariance).
    crate::unsafe_assume_covariance!(T);

    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        unsafe { <[T; N] as DeserHelper<<T as CopyType>::Copy>>::_deser_full_inner_impl(backend) }
    }

    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<<[T; N] as DeserHelper<<T as CopyType>::Copy>>::DeserType<'a>> {
        unsafe { <[T; N] as DeserHelper<<T as CopyType>::Copy>>::_deser_eps_inner_impl(backend) }
    }
}

impl<T: ZeroCopy + DeserInner, const N: usize> DeserHelper<Zero> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = &'a [T; N];

    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let mut res = MaybeUninit::<[T; N]>::uninit();
        backend.align::<T>()?;
        // SAFETY: we read exactly size_of::<[T; N]>() bytes into res, and
        // read_exact guarantees that the array will be filled with data.
        unsafe {
            let slice = core::slice::from_raw_parts_mut(
                res.as_mut_ptr() as *mut u8,
                core::mem::size_of::<[T; N]>(),
            );
            backend.read_exact(slice)?;
            Ok(res.assume_init())
        }
    }

    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<DeserType<'a, Self>> {
        let bytes = core::mem::size_of::<[T; N]>();
        if bytes == 0 {
            // SAFETY: [T; N] is zero-sized (see the NonNull::dangling docs)
            return Ok(unsafe { core::ptr::NonNull::<[T; N]>::dangling().as_ref() });
        }
        backend.align::<T>()?;
        let block = backend.data.get(..bytes).ok_or(deser::Error::ReadError)?;
        let (pre, data, after) = unsafe { block.align_to::<[T; N]>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        let res = &data[0];
        backend.skip(bytes)?;
        Ok(res)
    }
}

/// Initializes an array element by element, dropping the already-initialized
/// prefix and returning the error if the initialization of an element fails.
fn try_init_array<T, const N: usize>(
    mut init: impl FnMut() -> deser::Result<T>,
) -> deser::Result<[T; N]> {
    let mut res = MaybeUninit::<[T; N]>::uninit();
    let first = res.as_mut_ptr() as *mut T;
    for i in 0..N {
        match init() {
            // SAFETY: the i-th slot of the array is in bounds
            Ok(v) => unsafe { first.add(i).write(v) },
            Err(e) => {
                // SAFETY: the first i slots of the array have been initialized
                for j in 0..i {
                    unsafe { first.add(j).drop_in_place() };
                }
                return Err(e);
            }
        }
    }
    // SAFETY: all N slots of the array have been initialized
    Ok(unsafe { res.assume_init() })
}

impl<T: DeepCopy + DeserInner, const N: usize> DeserHelper<Deep> for [T; N] {
    type FullType = Self;
    type DeserType<'a> = [DeserType<'a, T>; N];

    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        try_init_array(|| unsafe { T::_deser_full_inner(backend) })
    }

    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<DeserType<'a, Self>> {
        try_init_array(|| unsafe { T::_deser_eps_inner(backend) })
    }
}
