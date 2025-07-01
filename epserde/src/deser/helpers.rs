/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Helpers for deserialization.

*/

use super::SliceWithPos;
use super::{read::*, DeserializeInner};
use crate::deser;
use crate::traits::*;
use core::mem::MaybeUninit;

/// Full-copy deserialize a zero-copy structure.
///
/// # Safety
///
/// See the documentation of [`Deserialize`](super::Deserialize).
pub unsafe fn deserialize_full_zero<T: ZeroCopy>(
    backend: &mut impl ReadWithPos,
) -> deser::Result<T> {
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
/// Note that this method uses a single [`ReadNoStd::read_exact`]
/// call to read the entire vector.
///
/// # Safety
///
/// See the documentation of [`Deserialize`](super::Deserialize).
pub unsafe fn deserialize_full_vec_zero<T: DeserializeInner + ZeroCopy>(
    backend: &mut impl ReadWithPos,
) -> deser::Result<Vec<T>> {
    let len = usize::_deserialize_full_inner(backend)?;
    backend.align::<T>()?;
    let mut res = Vec::with_capacity(len);
    // SAFETY: we just allocated this vector so it is safe to set the length.
    // read_exact guarantees that the vector will be filled with data.
    #[allow(clippy::uninit_vec)]
    unsafe {
        res.set_len(len);
        backend.read_exact(res.align_to_mut::<u8>().1)?;
    }

    Ok(res)
}

/// Full-copy deserialize a vector of deep-copy structures.
pub fn deserialize_full_vec_deep<T: DeserializeInner + DeepCopy>(
    backend: &mut impl ReadWithPos,
) -> deser::Result<Vec<T>> {
    let len = usize::_deserialize_full_inner(backend)?;
    let mut res = Vec::with_capacity(len);
    for _ in 0..len {
        res.push(T::_deserialize_full_inner(backend)?);
    }
    Ok(res)
}

/// ε-copy deserialize a reference to a zero-copy structure
/// backed by the `data` field of `backend`.
///
/// # Safety
///
/// See the documentation of [`Deserialize`](super::Deserialize).
pub unsafe fn deserialize_eps_zero<'a, T: ZeroCopy>(
    backend: &mut SliceWithPos<'a>,
) -> deser::Result<&'a T> {
    let bytes = core::mem::size_of::<T>();
    if bytes == 0 {
        // SAFETY: T is zero-sized and `assume_init` is safe.
        #[allow(invalid_value)]
        #[allow(clippy::uninit_assumed_init)]
        return Ok(unsafe { MaybeUninit::uninit().assume_init() });
    }
    backend.align::<T>()?;
    let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
    debug_assert!(pre.is_empty());
    debug_assert!(after.is_empty());
    let res = &data[0];
    backend.skip(bytes);
    Ok(res)
}

/// ε-copy deserialize a reference to a slice of zero-copy structures
/// backed by the `data` field of `backend`.
///
/// # Safety
///
/// See the documentation of [`Deserialize`](super::Deserialize).
pub unsafe fn deserialize_eps_slice_zero<'a, T: ZeroCopy>(
    backend: &mut SliceWithPos<'a>,
) -> deser::Result<&'a [T]> {
    let len = usize::_deserialize_full_inner(backend)?;
    let bytes = len * core::mem::size_of::<T>();
    if core::mem::size_of::<T>() == 0 {
        // SAFETY: T is zero-sized and `assume_init` is safe.
        #[allow(invalid_value)]
        #[allow(clippy::uninit_assumed_init)]
        return Ok(unsafe { std::slice::from_raw_parts(MaybeUninit::uninit().assume_init(), len) });
    }
    backend.align::<T>()?;
    let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
    debug_assert!(pre.is_empty());
    debug_assert!(after.is_empty());
    backend.skip(bytes);
    Ok(data)
}

/// ε-copy deserialize a vector of deep-copy structures.
pub fn deserialize_eps_vec_deep<'a, T: DeepCopy + DeserializeInner>(
    backend: &mut SliceWithPos<'a>,
) -> deser::Result<Vec<<T as DeserializeInner>::DeserType<'a>>> {
    let len = usize::_deserialize_full_inner(backend)?;
    let mut res = Vec::with_capacity(len);
    for _ in 0..len {
        res.push(T::_deserialize_eps_inner(backend)?);
    }
    Ok(res)
}
