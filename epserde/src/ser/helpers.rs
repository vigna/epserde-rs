/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Helpers for serialization.

*/

use super::{SerializeInner, WriteWithNames};
use crate::ser;
use crate::traits::*;

pub fn check_zero_copy<V: SerializeInner>() {
    if !V::IS_ZERO_COPY {
        panic!(
            "Cannot serialize type {} declared as zero-copy as it is not zero-copy",
            core::any::type_name::<V>()
        );
    }
}

/// Serialize a zero-copy structure checking [that the type is actually
/// zero-copy](SerializeInner::IS_ZERO_COPY) and [aligning the stream
/// beforehand](WriteWithNames::align).
///
/// This function makes the appropriate checks, write the necessary padding and
/// then calls [`serialize_zero_unchecked`](serialize_zero_unchecked).
pub fn serialize_zero<V: ZeroCopy + SerializeInner>(
    backend: &mut impl WriteWithNames,
    value: &V,
) -> ser::Result<()> {
    check_zero_copy::<V>();
    backend.align::<V>()?;
    serialize_zero_unchecked(backend, value)
}

/// Serialize a zero-copy structure without checking [that the type is actually
/// zero-copy](SerializeInner::IS_ZERO_COPY) and without [aligning the
/// stream](WriteWithNames::align).
///
/// Note that this method uses a single [`write_all`](std::io::Write::write_all)
/// call to write the entire structure.
#[inline(always)]
pub fn serialize_zero_unchecked<V: ZeroCopy + SerializeInner>(
    backend: &mut impl WriteWithNames,
    value: &V,
) -> ser::Result<()> {
    let buffer = unsafe {
        core::slice::from_raw_parts(value as *const V as *const u8, core::mem::size_of::<V>())
    };
    backend.write_bytes::<V>(buffer)
}

/// Serialize a slice of zero-copy structures by encoding
/// its length first, and then its bytes properly [aligned](WriteWithNames::align).
///
/// Note that this method uses a single `write_all`
/// call to write the entire slice.
///
/// Here we check [that the type is actually zero-copy](SerializeInner::IS_ZERO_COPY).
pub fn serialize_slice_zero<V: SerializeInner + ZeroCopy>(
    backend: &mut impl WriteWithNames,
    data: &[V],
) -> ser::Result<()> {
    check_zero_copy::<V>();

    let len = data.len();
    backend.write("len", &len)?;
    let num_bytes = core::mem::size_of_val(data);
    let buffer = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, num_bytes) };
    backend.align::<V>()?;
    backend.write_bytes::<V>(buffer)
}

pub fn check_mismatch<V: SerializeInner>() {
    if V::ZERO_COPY_MISMATCH {
        eprintln!("Type {} is zero-copy, but it has not declared as such; use the #[deep_copy] attribute to silence this warning", core::any::type_name::<V>());
    }
}

/// Serialize a slice of deep-copy structures by encoding
/// its length first, and then the contents item by item.
///
/// Here we warn [that the type might actually be zero-copy](SerializeInner::ZERO_COPY_MISMATCH).
pub fn serialize_slice_deep<V: SerializeInner>(
    backend: &mut impl WriteWithNames,
    data: &[V],
) -> ser::Result<()> {
    check_mismatch::<V>();
    let len = data.len();
    backend.write("len", &len)?;
    for item in data.iter() {
        backend.write("item", item)?;
    }
    Ok(())
}
