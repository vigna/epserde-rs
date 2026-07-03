/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! A cursor over a byte slice keeping track of the current position.

use super::*;
use crate::prelude::*;

/// `std::io::Cursor`-like structure for deserialization that does not
/// depend on [`std`].
///
/// Hand-written [`DeserInner`] implementations can read the still-unread
/// backing bytes with `data()` starting at the current position `pos()`, and
/// advance the cursor with `skip()`, to build ε-copy references into the
/// backing slice.
///
/// [`DeserInner`]: super::DeserInner
///
/// [`std`]: https://doc.rust-lang.org/std/
#[derive(Debug, Clone)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
pub struct SliceWithPos<'a> {
    /// The still-unread suffix of the backing slice; its first byte is at
    /// position `pos`.
    pub(crate) data: &'a [u8],
    /// The number of bytes already consumed from the start of the backing
    /// slice.
    pub(crate) pos: usize,
}

impl<'a> SliceWithPos<'a> {
    /// Returns a new [`SliceWithPos`] reading from `backend`, positioned at its
    /// start.
    pub const fn new(backend: &'a [u8]) -> Self {
        Self {
            data: backend,
            pos: 0,
        }
    }

    /// Returns the still-unread suffix of the backing slice.
    ///
    /// Its first byte is at the current position `pos()`.
    #[inline(always)]
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Returns the number of bytes already consumed from the start of the
    /// backing slice.
    #[inline(always)]
    pub const fn pos(&self) -> usize {
        self.pos
    }

    /// Advances the position by `bytes`, discarding them.
    ///
    /// Returns [`Error::ReadError`] if fewer than `bytes` bytes remain.
    ///
    /// [`Error::ReadError`]: super::Error::ReadError
    pub fn skip(&mut self, bytes: usize) -> deser::Result<()> {
        self.data = self.data.get(bytes..).ok_or(Error::ReadError)?;
        self.pos += bytes;
        Ok(())
    }
}

impl ReadNoStd for SliceWithPos<'_> {
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
        let len = buf.len();
        if len > self.data.len() {
            return Err(Error::ReadError);
        }
        buf.copy_from_slice(&self.data[..len]);
        self.data = &self.data[len..];
        self.pos += len;
        Ok(())
    }
}

impl ReadWithPos for SliceWithPos<'_> {
    fn pos(&self) -> usize {
        self.pos
    }

    /// Pads the cursor to the correct alignment.
    ///
    /// Note that this method also checks that the absolute memory position is
    /// properly aligned.
    fn align<T: AlignTo>(&mut self) -> deser::Result<()> {
        let align_to = T::align_to();
        // Zero-sized types impose no alignment
        if align_to == 0 {
            return Ok(());
        }
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, align_to);
        self.skip(padding)?;
        // Check that the ptr is indeed aligned
        if self.data.as_ptr() as usize % align_to != 0 {
            Err(Error::AlignmentError)
        } else {
            Ok(())
        }
    }
}
