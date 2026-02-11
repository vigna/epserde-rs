/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for strings.
//!
//! All string types have the same serialization type, `Box<str>`, and the same
//! deserialization type, `&str`. Thus, you can serialize a `String` and fully
//! deserialize it as `Box<str>`.
//!
//! Similarly to the case of [slices](crate::impls::slice), there is
//! a convenience [`SerInner`] implementation for `&str` that
//! serializes it as `Box<str>`.
//!
//! We provide type hashes for `String` and `str` so that they can be used
//! in [`PhantomData`](core::marker::PhantomData).

use crate::prelude::*;
use core::hash::Hash;
use deser::*;
use ser::*;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

unsafe impl CopyType for String {
    type Copy = Deep;
}

#[cfg(not(feature = "std"))]
use alloc::string::String;

// For use with PhantomData
impl TypeHash for String {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "String".hash(hasher);
    }
}

impl SerInner for String {
    type SerType = Box<str>;
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_zero(backend, self.as_bytes())
    }
}

impl DeserInner for String {
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let slice = unsafe { deser_full_vec_zero(backend) }?;
        Ok(String::from_utf8(slice).unwrap())
    }

    type DeserType<'a> = &'a str;

    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let slice = unsafe { deser_eps_slice_zero(backend) }?;
        // SAFETY: Actually this is unsafe if the data we read is not valid UTF-8
        Ok({
            unsafe {
                #[allow(clippy::transmute_bytes_to_str)]
                core::mem::transmute::<&'_ [u8], &'_ str>(slice)
            }
        })
    }
}

unsafe impl CopyType for Box<str> {
    type Copy = Deep;
}

impl TypeHash for Box<str> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box<str>".hash(hasher);
    }
}

impl AlignHash for Box<str> {
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl SerInner for Box<str> {
    type SerType = Self;
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_zero(backend, self.as_bytes())
    }
}

impl DeserInner for Box<str> {
    #[inline(always)]
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(unsafe { String::_deser_full_inner(backend) }?.into_boxed_str())
    }

    type DeserType<'a> = &'a str;

    #[inline(always)]
    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        unsafe { String::_deser_eps_inner(backend) }
    }
}

// For use with PhantomData
impl TypeHash for str {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "str".hash(hasher);
    }
}

// For use with PhantomData
impl TypeHash for &str {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "&str".hash(hasher);
    }
}
unsafe impl CopyType for &str {
    type Copy = Deep;
}

impl SerInner for &str {
    type SerType = Box<str>;
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        ser_slice_zero(backend, self.as_bytes())
    }
}
