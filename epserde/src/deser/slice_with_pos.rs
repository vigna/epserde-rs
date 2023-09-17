/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use super::*;
use crate::prelude::*;

/// [`std::io::Cursor`]-like trait for deserialization that does not
/// depend on [`std`].
#[derive(Debug)]
pub struct SliceWithPos<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> SliceWithPos<'a> {
    pub fn new(backend: &'a [u8]) -> Self {
        Self {
            data: backend,
            pos: 0,
        }
    }

    pub fn skip(&mut self, bytes: usize) {
        self.data = &self.data[bytes..];
        self.pos += bytes;
    }
}

impl<'a> ReadNoStd for SliceWithPos<'a> {
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

impl<'a> ReadWithPos for SliceWithPos<'a> {
    fn pos(&self) -> usize {
        self.pos
    }

    /// Pad the cursor to the correct alignment.
    ///
    /// Note that this method also checks that
    /// the absolute memory position is properly aligned.
    fn align<T: MaxSizeOf>(&mut self) -> deser::Result<()> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, T::max_size_of());
        self.skip(padding);
        // Check that the ptr is indeed aligned
        if self.data.as_ptr() as usize % T::max_size_of() != 0 {
            Err(Error::AlignmentError)
        } else {
            Ok(())
        }
    }
}
