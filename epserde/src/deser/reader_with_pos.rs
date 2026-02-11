/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::prelude::*;

use super::ReadNoStd;

#[cfg(not(feature = "std"))]
use alloc::vec;

/// A wrapper for a [`ReadNoStd`] that implements [`ReadWithPos`]
/// by keeping track of the current position.
#[derive(Debug)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
pub struct ReaderWithPos<'a, F: ReadNoStd> {
    /// What we actually read from
    backend: &'a mut F,
    /// How many bytes we have read from the start
    pos: usize,
}

impl<'a, F: ReadNoStd> ReaderWithPos<'a, F> {
    /// Create a new [`ReadWithPos`] on top of a generic [`ReadNoStd`].
    #[inline(always)]
    pub fn new(backend: &'a mut F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: ReadNoStd> ReadNoStd for ReaderWithPos<'_, F> {
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
        self.backend.read_exact(buf)?;
        self.pos += buf.len();
        Ok(())
    }
}

impl<F: ReadNoStd> ReadWithPos for ReaderWithPos<'_, F> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn align<T: AlignTo>(&mut self) -> deser::Result<()> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, T::align_to());
        self.read_exact(&mut vec![0; padding])?;
        // No alignment check, we are fully deserializing
        Ok(())
    }
}
