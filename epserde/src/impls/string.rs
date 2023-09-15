/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for strings.

*/

use crate::prelude::*;
use core::hash::Hash;
use des::*;
use ser::*;

impl CopyType for String {
    type Copy = Full;
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

#[cfg(feature = "alloc")]
impl TypeHash for String {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "String".hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(_hasher: &mut impl core::hash::Hasher) {
        // We save a slice of bytes. No alignment.
    }
}

impl TypeHash for str {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "str".hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(_hasher: &mut impl core::hash::Hasher) {
        // We save a slice of bytes. No alignment.
    }
}

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice_zero(self.as_bytes())
    }
}

impl DeserializeInner for String {
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        let (slice, backend) = backend.deserialize_vec_full_zero()?;
        let res = String::from_utf8(slice).unwrap();
        Ok((res, backend))
    }
    type DeserType<'a> = &'a str;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        let (slice, backend) = backend.deserialize_slice_zero()?;
        Ok((
            unsafe {
                #[allow(clippy::transmute_bytes_to_str)]
                core::mem::transmute::<&'_ [u8], &'_ str>(slice)
            },
            backend,
        ))
    }
}

impl CopyType for Box<str> {
    type Copy = Full;
}

impl SerializeInner for Box<str> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
        backend.write_slice_zero(self.as_bytes())
    }
}

impl DeserializeInner for Box<str> {
    #[inline(always)]
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
        String::_deserialize_full_copy_inner(backend).map(|(d, a)| (d.into_boxed_str(), a))
    }
    type DeserType<'a> = &'a str;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
        String::_deserialize_eps_copy_inner(backend).map(|(d, a)| (d, a))
    }
}
