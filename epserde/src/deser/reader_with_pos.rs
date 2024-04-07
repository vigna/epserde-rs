/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::prelude::*;

use super::ReadNoStd;
use mem_dbg::{MemDbg, MemSize};

/// A wrapper for a [`ReadNoStd`] that implements [`ReadWithPos`]
/// by keeping track of the current position.
#[derive(Debug, Clone, MemDbg, MemSize)]
pub struct ReaderWithPos<'a, F: ReadNoStd> {
    /// What we actually readfrom
    backend: &'a mut F,
    /// How many bytes we have read from the start
    pos: usize,
}

impl<'a, F: ReadNoStd> ReaderWithPos<'a, F> {
    #[inline(always)]
    /// Create a new [`ReadWithPos`] on top of a generic [`ReadNoStd`].
    pub fn new(backend: &'a mut F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<'a, F: ReadNoStd> ReadNoStd for ReaderWithPos<'a, F> {
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
        self.backend.read_exact(buf)?;
        self.pos += buf.len();
        Ok(())
    }
}

impl<'a, F: ReadNoStd> ReadWithPos for ReaderWithPos<'a, F> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn align<T: MaxSizeOf>(&mut self) -> deser::Result<()> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, T::max_size_of());
        self.read_exact(&mut vec![0; padding])?;
        // No alignment check, we are fully deserializing
        Ok(())
    }
}
