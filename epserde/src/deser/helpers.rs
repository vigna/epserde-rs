/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Helpers for deserialization.

use super::SliceWithPos;
use super::{DeserInner, read::*};
use crate::deser;
use crate::traits::*;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use crate::deser::DeserType;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Full-copy deserialize a zero-copy structure.
///
/// # Safety
///
/// See the documentation of [`Deserialize`].
///
/// [`Deserialize`]: super::Deserialize
pub unsafe fn deser_full_zero<T: ZeroCopy>(backend: &mut impl ReadWithPos) -> deser::Result<T> {
    backend.align::<T>()?;
    unsafe {
        let mut buf: MaybeUninit<T> = MaybeUninit::uninit();
        let slice = core::slice::from_raw_parts_mut(
            &mut buf as *mut MaybeUninit<T> as *mut u8,
            core::mem::size_of::<T>(),
        );
        backend.read_exact(slice)?;
        Ok(buf.assume_init())
    }
}

/// Full-copy deserialize a vector of zero-copy structures.
///
/// Note that this method uses a single [`ReadNoStd::read_exact`] call to read
/// the entire vector.
///
/// # Safety
///
/// See the documentation of [`Deserialize`].
///
/// [`Deserialize`]: super::Deserialize
pub unsafe fn deser_full_vec_zero<T: ZeroCopy>(
    backend: &mut impl ReadWithPos,
) -> deser::Result<Vec<T>> {
    let len = unsafe { usize::_deser_full_inner(backend) }?;
    backend.align::<T>()?;
    // Compute the byte count explicitly so we never rely on a panicking
    // allocator to reject an implausible length. For zero-sized types the
    // product is zero.
    let num_bytes = len
        .checked_mul(core::mem::size_of::<T>())
        .ok_or(deser::Error::CapacityOverflow)?;
    let mut res: Vec<T> = Vec::new();
    res.try_reserve_exact(len)
        .map_err(|_| deser::Error::CapacityOverflow)?;
    // Read into the spare capacity, so no reference to uninitialized values
    // of type T is ever created, and set the length only afterwards.
    let spare = res.spare_capacity_mut();
    // SAFETY: the spare capacity contains at least len elements, and
    // MaybeUninit<T> has the same layout as T. Note that the byte buffer is
    // uninitialized, which is covered by the read_exact caveat in the
    // Deserialize contract.
    let bytes =
        unsafe { core::slice::from_raw_parts_mut(spare.as_mut_ptr() as *mut u8, num_bytes) };
    backend.read_exact(bytes)?;
    // SAFETY: read_exact filled all len elements.
    unsafe { res.set_len(len) };

    Ok(res)
}

/// Full-copy deserialize a vector of deep-copy structures.
///
/// # Safety
///
/// See the documentation of [`Deserialize`].
///
/// [`Deserialize`]: super::Deserialize
pub unsafe fn deser_full_vec_deep<T: DeepCopy + DeserInner>(
    backend: &mut impl ReadWithPos,
) -> deser::Result<Vec<T>> {
    let len = unsafe { usize::_deser_full_inner(backend)? };
    let mut res = Vec::new();
    res.try_reserve_exact(len)
        .map_err(|_| deser::Error::CapacityOverflow)?;
    for _ in 0..len {
        res.push(unsafe { T::_deser_full_inner(backend)? });
    }
    Ok(res)
}

/// ε-copy deserialize a reference to a zero-copy structure backed by the `data`
/// field of `backend`.
///
/// # Safety
///
/// See the documentation of [`Deserialize`].
///
/// [`Deserialize`]: super::Deserialize
pub unsafe fn deser_eps_zero<'a, T: for<'b> ZeroCopy<DeserType<'b> = &'b T>>(
    backend: &mut SliceWithPos<'a>,
) -> deser::Result<&'a T> {
    let bytes = core::mem::size_of::<T>();
    backend.align::<T>()?;
    if bytes == 0 {
        // SAFETY: T is zero-sized (see the from_raw_parts docs)
        #[allow(invalid_value)]
        #[allow(clippy::uninit_assumed_init)]
        return Ok(unsafe { NonNull::<T>::dangling().as_ref() });
    }
    let block = backend.data.get(..bytes).ok_or(deser::Error::ReadError)?;
    let (pre, data, after) = unsafe { block.align_to::<T>() };
    if !pre.is_empty() {
        return Err(deser::Error::AlignmentError);
    }
    debug_assert!(after.is_empty());
    let res = &data[0];
    backend.skip(bytes)?;
    Ok(res)
}

/// ε-copy deserialize a reference to a slice of zero-copy structures backed by
/// the `data` field of `backend`.
///
/// # Safety
///
/// See the documentation of [`Deserialize`].
///
/// [`Deserialize`]: super::Deserialize
pub unsafe fn deser_eps_slice_zero<'a, T: ZeroCopy>(
    backend: &mut SliceWithPos<'a>,
) -> deser::Result<&'a [T]> {
    let len = unsafe { usize::_deser_full_inner(backend) }?;
    backend.align::<T>()?;
    if core::mem::size_of::<T>() == 0 {
        // SAFETY: T is zero-sized (see the from_raw_parts docs)
        #[allow(invalid_value)]
        #[allow(clippy::uninit_assumed_init)]
        return Ok(unsafe { core::slice::from_raw_parts(NonNull::dangling().as_ref(), len) });
    }
    let bytes = len
        .checked_mul(core::mem::size_of::<T>())
        .ok_or(deser::Error::ReadError)?;
    let block = backend.data.get(..bytes).ok_or(deser::Error::ReadError)?;
    let (pre, data, after) = unsafe { block.align_to::<T>() };
    if !pre.is_empty() {
        return Err(deser::Error::AlignmentError);
    }
    debug_assert!(after.is_empty());
    backend.skip(bytes)?;
    Ok(data)
}

/// ε-copy deserialize a vector of deep-copy structures.
///
/// # Safety
///
/// See the documentation of [`Deserialize`].
///
/// [`Deserialize`]: super::Deserialize
pub unsafe fn deser_eps_vec_deep<'a, T: DeepCopy + DeserInner>(
    backend: &mut SliceWithPos<'a>,
) -> deser::Result<Vec<DeserType<'a, T>>> {
    let len = unsafe { usize::_deser_full_inner(backend)? };
    let mut res = Vec::new();
    res.try_reserve_exact(len)
        .map_err(|_| deser::Error::CapacityOverflow)?;
    for _ in 0..len {
        res.push(unsafe { T::_deser_eps_inner(backend)? });
    }
    Ok(res)
}
