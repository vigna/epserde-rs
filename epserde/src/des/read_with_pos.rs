/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Deserialization traits and types

[`Deserialize`] is the main deserialization trait, providing methods
[`Deserialize::deserialize_eps_copy`] and [`Deserialize::deserialize_full_copy`]
which implement ε-copy and full-copy deserialization, respectively,
starting from a slice of bytes. The implementation of this trait
is based on [`DeserializeInner`], which is automatically derived
with `#[derive(Deserialize)]`.

Note that [`Deserialize::deserialize_full_copy`] is internally necessary
to deserialize fields whose type is not a parameter, but technically
it could be hidden from the user interface. It can however be useful
for debugging and in cases in which a full copy is necessary.

*/

use crate::ZeroCopy;
use crate::{des, DeserializeError, DeserializeInner};
use core::mem::MaybeUninit;

/// [`std::io::Read`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly
/// this trait as we provide a blanket
/// implementation that implements [`ReadNoStd`] for all types that implement
/// [`std::io::Read`]. In particular, in such a context you can use [`std::io::Cursor`]
/// for in-memory deserialization.
pub trait ReadNoStd {
    /// Read some bytes and return the number of bytes read
    fn read(&mut self, buf: &mut [u8]) -> des::Result<usize>;

    fn read_exact(&mut self, buf: &mut [u8]) -> des::Result<()>;
}

#[cfg(feature = "std")]
use std::io::Read;
#[cfg(feature = "std")]
impl<W: Read> ReadNoStd for W {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> des::Result<usize> {
        Read::read(self, buf).map_err(|_| DeserializeError::ReadError)
    }

    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> des::Result<()> {
        Read::read_exact(self, buf).map_err(|_| DeserializeError::ReadError)
    }
}

/// A trait for [`ReadNoStd`] that also keeps track of the current position.
///
/// This is needed because the [`Read`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
pub trait ReadWithPos: ReadNoStd + Sized {
    fn pos(&self) -> usize;

    /// Pad the cursor to the correct alignment and check that the resulting
    /// pointer is aligned correctly.
    fn pad_align_and_check<T>(self) -> des::Result<Self>;

    /// Read a zero-copy type from the backend.
    fn read_full_zero_copy<T: ZeroCopy>(mut self) -> des::Result<(T, Self)> {
        self = self.pad_align_and_check::<Self>()?;
        unsafe {
            #[allow(clippy::uninit_assumed_init)]
            let mut buf: T = MaybeUninit::uninit().assume_init();
            let slice = core::slice::from_raw_parts_mut(
                &mut buf as *mut T as *mut u8,
                core::mem::size_of::<T>(),
            );
            self.read_exact(slice)?;
            Ok((buf, self))
        }
    }

    fn deserialize_vec_full_zero<T: DeserializeInner>(self) -> des::Result<(Vec<T>, Self)> {
        let (len, mut res_self) = usize::_deserialize_full_copy_inner(self)?;
        res_self = res_self.pad_align_and_check::<T>()?;
        let mut res = Vec::with_capacity(len);
        // SAFETY: we just allocated this vector so it is safe to set the length.
        // read_exact guarantees that the vector will be filled with data.
        #[allow(clippy::uninit_vec)]
        unsafe {
            res.set_len(len);
            res_self.read_exact(res.align_to_mut::<u8>().1)?;
        }

        Ok((res, res_self))
    }

    fn deserialize_vec_full_eps<T: DeserializeInner>(self) -> des::Result<(Vec<T>, Self)> {
        let (len, mut res_self) = usize::_deserialize_full_copy_inner(self)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, temp_self) = T::_deserialize_full_copy_inner(res_self)?;
            res.push(elem);
            res_self = temp_self;
        }
        Ok((res, res_self))
    }
}

/// A wrapper for a [`ReadNoStd`] that implements [`ReadWithPos`]
/// by keeping track of the current position.
pub struct ReaderWithPos<F: ReadNoStd> {
    /// What we actually readfrom
    backend: F,
    /// How many bytes we have read from the start
    pos: usize,
}

impl<F: ReadNoStd> ReaderWithPos<F> {
    #[inline(always)]
    /// Create a new [`ReadWithPos`] on top of a generic Reader `F`
    pub fn new(backend: F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: ReadNoStd> ReadNoStd for ReaderWithPos<F> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> des::Result<usize> {
        let res = self.backend.read(buf)?;
        self.pos += res;
        Ok(res)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> des::Result<()> {
        self.backend.read_exact(buf)?;
        self.pos += buf.len();
        Ok(())
    }
}

impl<F: ReadNoStd> ReadWithPos for ReaderWithPos<F> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn pad_align_and_check<T>(mut self) -> des::Result<Self> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, core::mem::align_of::<T>());
        self.read_exact(&mut vec![0; padding])?;
        // No alignment check, we are fully deserializing
        Ok(self)
    }
}
