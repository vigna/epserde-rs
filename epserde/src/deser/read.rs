/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

No-std support for reading while keeping track of the current position.

 */

use super::DeserializeInner;
use crate::prelude::*;
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
    /// Read some bytes
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
    /// Return the current position.
    fn pos(&self) -> usize;

    /// Pad the cursor to the provided `align_of` value.
    fn align<T: MaxSizeOf>(&mut self) -> deser::Result<()>;

    /// Fully deserialize a zero-copy type from the backend after aligning it.
    fn deserialize_full_zero<T: ZeroCopy>(&mut self) -> deser::Result<T> {
        self.align::<T>()?;
        unsafe {
            #[allow(clippy::uninit_assumed_init)]
            let mut buf: T = MaybeUninit::uninit().assume_init();
            let slice = core::slice::from_raw_parts_mut(
                &mut buf as *mut T as *mut u8,
                core::mem::size_of::<T>(),
            );
            self.read_exact(slice)?;
            Ok(buf)
        }
    }

    /// Fully deserialize a vector of [`ZeroCopy`] types.
    ///
    /// Note that this method uses a single [`ReadNoStd::read_exact`]
    /// call to read the entire vector.
    fn deserialize_vec_full_zero<T: DeserializeInner + ZeroCopy>(
        &mut self,
    ) -> deser::Result<Vec<T>> {
        let len = usize::_deserialize_full_inner(self)?;
        self.align::<T>()?;
        let mut res = Vec::with_capacity(len);
        // SAFETY: we just allocated this vector so it is safe to set the length.
        // read_exact guarantees that the vector will be filled with data.
        #[allow(clippy::uninit_vec)]
        unsafe {
            res.set_len(len);
            self.read_exact(res.align_to_mut::<u8>().1)?;
        }

        Ok(res)
    }

    /// Deserializes fully a vector of [`DeepCopy`] types.
    fn deserialize_vec_full_eps<T: DeserializeInner + DeepCopy>(
        &mut self,
    ) -> deser::Result<Vec<T>> {
        let len = usize::_deserialize_full_inner(self)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            res.push(T::_deserialize_full_inner(self)?);
        }
        Ok(res)
    }
}
