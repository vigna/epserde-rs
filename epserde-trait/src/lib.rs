/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use anyhow::Result;
use std::{
    fmt,
    io::{Seek, Write},
};
#[cfg(target_endian = "little")]
const ENDIANNESS_MARKER: u64 = 0xdeadbeefdeadf00d;
#[cfg(target_endian = "big")]
const ENDIANNESS_MARKER: u64 = 0xbadf00dc0ffed0d;

#[derive(Debug, Clone)]
struct EndiannessError;

impl std::error::Error for EndiannessError {}

impl fmt::Display for EndiannessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mismatched endianness marker")
    }
}

pub trait Serialize {
    fn serialize<F: Write + Seek>(&self, backend: &mut F) -> Result<usize>;

    /// Write an [`ENDIANNESS_MARKER`] at the start of the struct.
    fn write_endianness_marker<F: Write + Seek>(backend: &mut F) -> Result<usize> {
        backend.write_all(&ENDIANNESS_MARKER.to_ne_bytes())?;
        Ok(std::mem::size_of_val(&ENDIANNESS_MARKER))
    }
}

pub trait Deserialize<'a>: Sized {
    fn deserialize(backend: &'a [u8]) -> Result<(Self, &'a [u8])>;

    /// Check that the [`ENDIANNESS_MARKER`] is correct; return an error otherwise.
    fn check_endianness_marker(backend: &'a [u8]) -> Result<&'a [u8]> {
        let (marker, backend) = u64::deserialize(backend)?;
        match marker {
            ENDIANNESS_MARKER => Ok(backend),
            _ => Err(EndiannessError {}.into()),
        }
    }
}

/// Compute the padding needed for alignement, i.e., the number so that
/// `((value + pad_align_to(value, bits) & (bits - 1) == 0`.
///
/// ```
/// use epserde_trait::pad_align_to;
/// assert_eq!(7 + pad_align_to(7, 8), 8);
/// assert_eq!(8 + pad_align_to(8, 8), 8);
/// assert_eq!(9 + pad_align_to(9, 8), 16);
/// ```
pub fn pad_align_to(value: usize, bits: usize) -> usize {
    value.wrapping_neg() & (bits - 1)
}

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(

impl Serialize for $ty {
    #[inline(always)]
    fn serialize<F: Write>(&self, backend: &mut F) -> Result<usize> {
        Ok(backend.write(&self.to_ne_bytes())?)
    }
}

impl<'a> Deserialize<'a> for $ty {
    #[inline(always)]
    fn deserialize(backend: &'a [u8]) -> Result<(Self, &'a [u8])> {
        Ok((
            <$ty>::from_ne_bytes(backend[..core::mem::size_of::<$ty>()].try_into().unwrap()),
            &backend[core::mem::size_of::<$ty>()..],
        ))
    }
}
        impl<'a> Deserialize<'a> for &'a [$ty] {
            fn deserialize(backend: &'a [u8]) -> Result<(Self, &'a [u8])> {
                let (len, backend) = usize::deserialize(backend)?;
                let bytes = len * core::mem::size_of::<$ty>();
                let (_pre, data, after) = unsafe { backend[..bytes].align_to() };
                // TODO make error / we added padding so it's ok
                assert!(after.is_empty());
                Ok((data, &backend[bytes..]))
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128);

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize<F: Write + Seek>(&self, backend: &mut F) -> Result<usize> {
        let len = self.len();
        let mut bytes = 0;
        bytes += backend.write(&len.to_ne_bytes())?;
        // ensure alignement
        let file_pos = backend.stream_position()? as usize;
        for _ in 0..pad_align_to(file_pos, core::mem::size_of::<T>()) {
            bytes += backend.write(&[0])?;
        }
        // write the values
        for item in self {
            bytes += item.serialize(backend)?;
        }
        Ok(bytes)
    }
}
