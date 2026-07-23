/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Helpers for serialization.

use super::{SerInner, WriteWithNames};
use crate::ser;
use crate::traits::*;

/// Checks that the type is [zero-copy at runtime], panicking otherwise.
///
/// # Panics
///
/// Panics if `V` is declared as zero-copy but its [`SerInner::IS_ZERO_COPY`]
/// constant is false.
///
/// [zero-copy at runtime]: SerInner::IS_ZERO_COPY
#[inline]
pub fn check_zero_copy<V: SerInner>() {
    if !V::IS_ZERO_COPY {
        panic!(
            "Cannot serialize type {} declared as zero-copy as it is not zero-copy",
            core::any::type_name::<V>()
        );
    }
}

/// Serialize a zero-copy structure checking [that the type is actually
/// zero-copy] and [aligning the stream beforehand].
///
/// This function makes the appropriate checks, writes the necessary padding and
/// then calls [`ser_zero_unchecked`].
///
/// # Safety
///
/// See the documentation of [`Serialize`].
///
/// [that the type is actually zero-copy]: SerInner::IS_ZERO_COPY
/// [aligning the stream beforehand]: WriteWithNames::align
/// [`Serialize`]: super::Serialize
#[inline]
pub unsafe fn ser_zero<V: ZeroCopy>(
    backend: &mut impl WriteWithNames,
    value: &V,
) -> ser::Result<()> {
    check_zero_copy::<V>();
    backend.align::<V>()?;
    unsafe { ser_zero_unchecked(backend, value) }
}

/// Serialize a zero-copy structure without checking [that the type is actually
/// zero-copy] and without [aligning the stream].
///
/// Note that this method uses a single [`write_all`] call to write the entire
/// structure.
///
/// # Safety
///
/// See the documentation of [`Serialize`].
///
/// [that the type is actually zero-copy]: SerInner::IS_ZERO_COPY
/// [aligning the stream]: WriteWithNames::align
/// [`write_all`]: super::WriteNoStd::write_all
/// [`Serialize`]: super::Serialize
#[inline]
pub unsafe fn ser_zero_unchecked<V: ZeroCopy>(
    backend: &mut impl WriteWithNames,
    value: &V,
) -> ser::Result<()> {
    // SAFETY: V is zero-copy, so its memory representation is a valid
    // sequence of bytes, except possibly for padding, whose bytes might be
    // uninitialized (this is why this function is unsafe).
    let buffer = unsafe {
        core::slice::from_raw_parts(value as *const V as *const u8, core::mem::size_of::<V>())
    };
    backend.write_bytes::<V>(buffer)
}

/// Serialize a slice of zero-copy structures by encoding its length first, and
/// then its bytes properly [aligned].
///
/// Note that this method uses a single `write_all`
/// call to write the entire slice.
///
/// Here we check [that the type is actually zero-copy].
///
/// # Safety
///
/// See the documentation of [`Serialize`].
///
/// [aligned]: WriteWithNames::align
/// [that the type is actually zero-copy]: SerInner::IS_ZERO_COPY
/// [`Serialize`]: super::Serialize
#[inline]
pub unsafe fn ser_slice_zero<V: ZeroCopy>(
    backend: &mut impl WriteWithNames,
    data: &[V],
) -> ser::Result<()> {
    check_zero_copy::<V>();

    let len = data.len();
    unsafe { backend.write("len", &len)? };
    let num_bytes = core::mem::size_of_val(data);
    // SAFETY: V is zero-copy, so the slice's memory representation is a valid
    // sequence of bytes, except possibly for padding, whose bytes might be
    // uninitialized (this is why this function is unsafe).
    let buffer = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, num_bytes) };
    backend.align::<V>()?;
    backend.write_bytes::<V>(buffer)
}

/// Serialize a slice of deep-copy structures by encoding
/// its length first, and then the contents item by item.
///
/// # Safety
///
/// See the documentation of [`Serialize`].
///
/// [`Serialize`]: super::Serialize
#[inline]
pub unsafe fn ser_slice_deep<V: SerInner>(
    backend: &mut impl WriteWithNames,
    data: &[V],
) -> ser::Result<()> {
    let len = data.len();
    unsafe {
        backend.write("len", &len)?;
        for item in data.iter() {
            backend.write("item", item)?;
        }
    }
    Ok(())
}
