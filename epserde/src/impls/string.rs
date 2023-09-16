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
    type Copy = Deep;
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

#[cfg(feature = "alloc")]
impl TypeHash for String {
    fn type_hash(
        type_hasher: &mut impl core::hash::Hasher,
        _repr_hasher: &mut impl core::hash::Hasher,
        _offset_of: &mut usize,
    ) {
        "String".hash(type_hasher);
    }
}

impl TypeHash for str {
    fn type_hash(
        type_hasher: &mut impl core::hash::Hasher,
        _repr_hasher: &mut impl core::hash::Hasher,
        _offset_of: &mut usize,
    ) {
        "str".hash(type_hasher);
    }
}

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: &mut F) -> ser::Result<()> {
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
    type Copy = Deep;
}

impl SerializeInner for Box<str> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: &mut F) -> ser::Result<()> {
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
