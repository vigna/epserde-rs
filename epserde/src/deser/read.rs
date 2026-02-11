/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! No-std support for reading while keeping track of the current position.

use crate::prelude::*;

/// [`std::io::Read`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly this trait as
/// we provide a blanket implementation that implements [`ReadNoStd`] for all
/// types that implement [`std::io::Read`]. In particular, in such a context you
/// can use [`AlignedCursor`] for ε-copy deserialization.
pub trait ReadNoStd {
    /// See [`read_exact`](http://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact) for more details.
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()>;
}

#[cfg(feature = "std")]
use std::io::Read;
#[cfg(feature = "std")]
impl<W: Read> ReadNoStd for W {
    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
        Read::read_exact(self, buf).map_err(|_| deser::Error::ReadError)
    }
}

/// A trait for [`ReadNoStd`] that also keeps track of the current position.
///
/// This is needed because the [`Read`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
pub trait ReadWithPos: ReadNoStd + Sized {
    /// Returns the current position.
    fn pos(&self) -> usize;

    /// Pads the cursor to the next multiple of [`AlignTo::align_to`] 'T'.
    fn align<T: AlignTo>(&mut self) -> deser::Result<()>;
}
