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

#[cfg(feature = "alloc")]
impl TypeHash for String {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "String".hash(hasher);
    }
}

impl ReprHash for String {}

impl TypeHash for str {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "str".hash(hasher);
    }
}

impl ReprHash for str {}

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> ser::Result<()> {
        backend.write_slice_zero(self.as_bytes())
    }
}

impl DeserializeInner for String {
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let slice = backend.deserialize_vec_full_zero()?;
        Ok(String::from_utf8(slice).unwrap())
    }
    type DeserType<'a> = &'a str;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let slice = backend.deserialize_slice_zero()?;
        Ok(unsafe {
            #[allow(clippy::transmute_bytes_to_str)]
            core::mem::transmute::<&'_ [u8], &'_ str>(slice)
        })
    }
}

impl CopyType for Box<str> {
    type Copy = Deep;
}

impl SerializeInner for Box<str> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> ser::Result<()> {
        backend.write_slice_zero(self.as_bytes())
    }
}

impl DeserializeInner for Box<str> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(String::_deserialize_full_inner(backend)?.into_boxed_str())
    }
    type DeserType<'a> = &'a str;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        String::_deserialize_eps_inner(backend)
    }
}
