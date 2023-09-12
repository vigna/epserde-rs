/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::{des, DeserializeError, DeserializeInner};
use crate::{EpsCopy, ZeroCopy};
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
    /// Return the current position.
    fn pos(&self) -> usize;

    /// Pad the cursor to the correct alignment.
    fn align<T>(self) -> des::Result<Self>;

    /// Fully deserialize a zero-copy type from the backend after aligning it.
    fn deserialize_full_zero<T: ZeroCopy>(mut self) -> des::Result<(T, Self)> {
        self = self.align::<T>()?;
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

    /// Fully deserialize a vector of [`ZeroCopy`] types.
    ///
    /// Note that this method uses a single [`ReadNoStd::read_exact`]
    /// call to read the entire vector.
    fn deserialize_vec_full_zero<T: DeserializeInner + ZeroCopy>(
        self,
    ) -> des::Result<(Vec<T>, Self)> {
        let (len, mut res_self) = usize::_deserialize_full_copy_inner(self)?;
        res_self = res_self.align::<T>()?;
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

    /// Deserializes fully a vector of [`EpsCopy`] types.
    fn deserialize_vec_full_eps<T: DeserializeInner + EpsCopy>(
        self,
    ) -> des::Result<(Vec<T>, Self)> {
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
    /// Create a new [`ReadWithPos`] on top of a generic [`ReadNoStd`].
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

    fn align<T>(mut self) -> des::Result<Self> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, core::mem::align_of::<T>());
        self.read_exact(&mut vec![0; padding])?;
        // No alignment check, we are fully deserializing
        Ok(self)
    }
}
