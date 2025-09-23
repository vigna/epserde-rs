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
use deser::*;
use ser::*;

impl CopyType for String {
    type Copy = Deep;
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

impl TypeHash for String {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "String".hash(hasher);
    }
}

impl AlignHash for String {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl TypeHash for Box<str> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box<str>".hash(hasher);
    }
}

impl AlignHash for Box<str> {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl TypeHash for str {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "str".hash(hasher);
    }
}

impl AlignHash for str {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl SerializeInner for String {
    type SerType = Self;
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_slice_zero(backend, self.as_bytes())
    }
}

impl DeserializeInner for String {
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let slice = unsafe { deserialize_full_vec_zero(backend) }?;
        Ok(String::from_utf8(slice).unwrap())
    }

    type DeserType<'a> = &'a str;

    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let slice = unsafe { deserialize_eps_slice_zero(backend) }?;
        // SAFETY: Actually this is unsafe if the data we read is not valid UTF-8
        Ok({
            unsafe {
                #[allow(clippy::transmute_bytes_to_str)]
                core::mem::transmute::<&'_ [u8], &'_ str>(slice)
            }
        })
    }
}

impl CopyType for Box<str> {
    type Copy = Deep;
}

impl SerializeInner for Box<str> {
    type SerType = Self;
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        serialize_slice_zero(backend, self.as_bytes())
    }
}

impl DeserializeInner for Box<str> {
    #[inline(always)]
    unsafe fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(unsafe { String::_deserialize_full_inner(backend) }?.into_boxed_str())
    }

    type DeserType<'a> = &'a str;

    #[inline(always)]
    unsafe fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        unsafe { String::_deserialize_eps_inner(backend) }
    }
}
