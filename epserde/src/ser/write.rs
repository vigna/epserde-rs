/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! No-std support for writing while keeping track of the current position.

use crate::prelude::*;
use mem_dbg::{MemDbg, MemSize};

/// [`std::io::Write`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly
/// this trait as we provide a blanket
/// implementation that implements [`WriteNoStd`] for all types that implement
/// [`std::io::Write`]. In particular, in such a context you can use [`std::io::Cursor`]
/// for in-memory serialization.
pub trait WriteNoStd {
    /// Write some bytes.
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()>;

    /// Flush all changes to the underlying storage if applicable.
    fn flush(&mut self) -> ser::Result<()>;
}

#[cfg(feature = "std")]
use std::io::Write;

#[cfg(feature = "std")]
impl<W: Write> WriteNoStd for W {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        Write::write_all(self, buf).map_err(|_| ser::Error::WriteError)
    }
    #[inline(always)]
    fn flush(&mut self) -> ser::Result<()> {
        Write::flush(self).map_err(|_| ser::Error::WriteError)
    }
}

/// A trait for [`WriteNoStd`] that also keeps track of the current position.
///
/// This is needed because the [`Write`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
pub trait WriteWithPos: WriteNoStd {
    fn pos(&self) -> usize;
}

/// A wrapper for a [`WriteNoStd`] that implements [`WriteWithPos`]
/// by keeping track of the current position.
#[derive(Debug, MemDbg, MemSize)]
pub struct WriterWithPos<'a, F: WriteNoStd> {
    /// What we actually write on.
    backend: &'a mut F,
    /// How many bytes we have written from the start.
    pos: usize,
}

impl<'a, F: WriteNoStd> WriterWithPos<'a, F> {
    #[inline(always)]
    /// Create a new [`WriterWithPos`] on top of a generic [`WriteNoStd`] `F`.
    pub fn new(backend: &'a mut F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: WriteNoStd> WriteNoStd for WriterWithPos<'_, F> {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        self.backend.write_all(buf)?;
        self.pos += buf.len();
        Ok(())
    }

    #[inline(always)]
    fn flush(&mut self) -> ser::Result<()> {
        self.backend.flush()
    }
}

impl<F: WriteNoStd> WriteWithPos for WriterWithPos<'_, F> {
    #[inline(always)]
    fn pos(&self) -> usize {
        self.pos
    }
}
