/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! No-std support for writing while keeping track of the current position.

use crate::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// [`std::io::Write`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly
/// this trait as we provide a blanket
/// implementation that implements [`WriteNoStd`] for all types that implement
/// [`std::io::Write`]. In particular, in such a context you can use [`std::io::Cursor`]
/// for in-memory serialization.
///
/// [`std::io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`std`]: https://doc.rust-lang.org/std/
/// [`std::io::Cursor`]: https://doc.rust-lang.org/std/io/struct.Cursor.html
pub trait WriteNoStd {
    /// See [`write_all`] for more details.
    ///
    /// [`write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()>;

    /// See [`flush`] for more details.
    ///
    /// [`flush`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.flush
    fn flush(&mut self) -> ser::Result<()>;
}

#[cfg(feature = "std")]
use std::io::Write;

#[cfg(feature = "std")]
impl<W: Write> WriteNoStd for W {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        Write::write_all(self, buf).map_err(ser::Error::IoError)
    }
    #[inline(always)]
    fn flush(&mut self) -> ser::Result<()> {
        Write::flush(self).map_err(ser::Error::IoError)
    }
}

#[cfg(not(feature = "std"))]
impl WriteNoStd for Vec<u8> {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }
    #[inline(always)]
    fn flush(&mut self) -> ser::Result<()> {
        Ok(())
    }
}

/// A trait for [`WriteNoStd`] that also keeps track of the current position.
///
/// This is needed because the [`Write`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`std::io::Seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html
pub trait WriteWithPos: WriteNoStd {
    fn pos(&self) -> usize;
}

/// A wrapper for a [`WriteNoStd`] that implements [`WriteWithPos`] by keeping
/// track of the current position.
#[derive(Debug)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
pub struct WriterWithPos<'a, F: WriteNoStd> {
    /// What we actually write on.
    backend: &'a mut F,
    /// How many bytes we have written from the start.
    pos: usize,
}

impl<'a, F: WriteNoStd> WriterWithPos<'a, F> {
    #[inline(always)]
    /// Create a new [`WriterWithPos`] on top of a generic [`WriteNoStd`] `F`.
    pub const fn new(backend: &'a mut F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: WriteNoStd> WriteNoStd for WriterWithPos<'_, F> {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        self.backend.write_all(buf)?;
        // Checked: on a backend accepting more than usize::MAX bytes (e.g.,
        // a stream sink on a 32-bit target) a wrapped position would corrupt
        // padding computations, schema offsets, and the value returned by
        // Serialize::serialize.
        self.pos = self
            .pos
            .checked_add(buf.len())
            .ok_or(ser::Error::WriteError)?;
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
