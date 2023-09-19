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

fn check_zero_copy<V: SerializeInner>() {
    if !V::IS_ZERO_COPY {
        panic!(
            "Cannot serialize type {} declared as zero-copy as it is not zero-copy",
            core::any::type_name::<V>()
        );
    }
}

/// Serialize a zero-copy structure by writing its bytes properly [aligned](WriteWithNames::align).
///
/// Note that this method uses a single `write_all` call to write the entire structure.
///
/// Here we check [that the type is actually zero-copy](SerializeInner::IS_ZERO_COPY).
pub fn serialize_zero<V: ZeroCopy + SerializeInner>(
    backend: &mut impl WriteWithNames,
    value: &V,
) -> ser::Result<()> {
    check_zero_copy::<V>();
    let buffer = unsafe {
        #[allow(clippy::manual_slice_size_calculation)]
        core::slice::from_raw_parts(value as *const V as *const u8, core::mem::size_of::<V>())
    };
    backend.align::<V>()?;
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
    backend._serialize_inner("len", &len)?;
    let buffer = unsafe {
        #[allow(clippy::manual_slice_size_calculation)]
        core::slice::from_raw_parts(data.as_ptr() as *const u8, len * core::mem::size_of::<V>())
    };
    backend.align::<V>()?;
    backend.write_bytes::<V>(buffer)
}

fn check_mismatch<V: SerializeInner>() {
    if V::ZERO_COPY_MISMATCH {
        eprintln!("Type {} is zero-copy, but it has not declared as such; use the #deep_copy attribute to silence this warning", core::any::type_name::<V>());
    }
}

/// Serialize a deep-copy structure by delegating to [`SerializeInner::_serialize_inner`].
///
/// Here we warn [that the type might actually be zero-copy](SerializeInner::ZERO_COPY_MISMATCH).
pub fn serialize_deep<V: SerializeInner>(
    backend: &mut impl WriteWithNames,
    data: V,
) -> ser::Result<()> {
    check_mismatch::<V>();
    data._serialize_inner(backend)
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
    backend._serialize_inner("len", &len)?;
    for item in data.iter() {
        backend._serialize_inner("item", item)?;
    }
    Ok(())
}
