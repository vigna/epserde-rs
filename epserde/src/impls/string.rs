/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Implementations for strings.
//!
//! The string types `String`, `Box<str>`, and `&str` have the same
//! serialization type, `Box<str>`; the deserializable owners `String` and
//! `Box<str>` have deserialization type `&str`. Thus, you can serialize a
//! `String` and fully deserialize it as `Box<str>`.
//!
//! Similarly to the case of [slices], the [`SerInner`] implementation for
//! `&str` is a serialization-only convenience.
//!
//! We implement [`TypeHash`] for `str` so that it can be used in
//! [`PhantomData`].
//!
//! [slices]: crate::impls::slice
//! [`PhantomData`]: core::marker::PhantomData

use crate::{check_covariance, prelude::*};
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

impl TypeHash for String {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "String".hash(hasher);
    }
}

impl SerInner for String {
    type SerType = Box<str>;
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { ser_slice_zero(backend, self.as_bytes()) }
    }
}

impl DeserInner for String {
    check_covariance!();
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let slice = unsafe { deser_full_vec_zero(backend) }?;
        // SAFETY: the bytes are valid UTF-8 because the data comes from a
        // correct serialization (see the Deserialize contract).
        Ok(unsafe { String::from_utf8_unchecked(slice) })
    }

    type DeserType<'a> = &'a str;

    unsafe fn _deser_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let slice = unsafe { deser_eps_slice_zero(backend) }?;
        // SAFETY: the bytes are valid UTF-8 because the data comes from a
        // correct serialization (see the Deserialize contract).
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
    // The bytes of a Box<str> are written as a zero-copy slice, but
    // Box<str> itself is not zero-copy.
    const IS_ZERO_COPY: bool = false;

    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { ser_slice_zero(backend, self.as_bytes()) }
    }
}

impl DeserInner for Box<str> {
    check_covariance!();
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
        unsafe { ser_slice_zero(backend, self.as_bytes()) }
    }
}
