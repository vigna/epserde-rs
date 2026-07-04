/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! No-std support for reading while keeping track of the current position.

use crate::prelude::*;

/// [`std::io::Read`]-like trait for deserialization that does not depend on
/// [`std`].
///
/// In an [`std`] context, the user does not need to use directly this trait as
/// we provide a blanket implementation that implements [`ReadNoStd`] for all
/// types that implement [`std::io::Read`]. In particular, in such a context you
/// can use [`AlignedCursor`] for ε-copy deserialization.
///
/// [`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`std`]: https://doc.rust-lang.org/std/
pub trait ReadNoStd {
    /// See [`read_exact`] for more details.
    ///
    /// [`read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()>;
}

#[cfg(feature = "std")]
use std::io::Read;
#[cfg(feature = "std")]
impl<W: Read> ReadNoStd for W {
    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
        // A truncated input maps to ReadError, so truncation surfaces as the
        // same variant on all deserialization paths; other I/O failures keep
        // their cause in IoError.
        Read::read_exact(self, buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                deser::Error::ReadError
            } else {
                deser::Error::IoError(e)
            }
        })
    }
}

#[cfg(not(feature = "std"))]
/// Byte slices are readers consuming themselves from the front, mirroring the
/// [`std::io::Read`] implementation for `&[u8]`. This impl is gated like the
/// [`ReadNoStd`] impl for [`AlignedCursor`]: when `std` is available, byte
/// slices are covered by the blanket implementation, and a second
/// implementation would conflict with it.
///
/// [`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
impl ReadNoStd for &[u8] {
    fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
        if self.len() < buf.len() {
            return Err(deser::Error::ReadError);
        }
        let (head, tail) = self.split_at(buf.len());
        buf.copy_from_slice(head);
        *self = tail;
        Ok(())
    }
}

/// A trait for [`ReadNoStd`] that also keeps track of the current position.
///
/// This is needed because the [`Read`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`std::io::Seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html
pub trait ReadWithPos: ReadNoStd + Sized {
    /// Returns the current position.
    fn pos(&self) -> usize;

    /// Pads the cursor to the next multiple of [`PadTo::pad_to`] of `T`.
    fn align<T: PadTo>(&mut self) -> deser::Result<()>;
}
