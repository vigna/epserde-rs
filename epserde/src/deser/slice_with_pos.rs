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

    /// Return a reference, backed by the `data` field, to a zero-copy type.
    pub fn deserialize_eps_zero<T: ZeroCopy>(&mut self) -> deser::Result<&'a T> {
        let bytes = core::mem::size_of::<T>();
        // a slice can onlgity be deserialized with zero copy
        // outerwise you need a vec, TODO!: how do we enforce this at compile time?
        self.align::<T>()?;
        let (pre, data, after) = unsafe { self.data[..bytes].align_to::<T>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        let res = &data[0];
        self.skip(bytes);
        Ok(res)
    }

    /// Return a reference, backed by the `data` field,
    /// to a slice whose elements are of zero-copy type.
    pub fn deserialize_eps_slice_zero<T: ZeroCopy>(&mut self) -> deser::Result<&'a [T]> {
        let len = usize::_deserialize_full_inner(self)?;
        let bytes = len * core::mem::size_of::<T>();
        // a slice can only be deserialized with zero copy
        // outerwise you need a vec, TODO!: how do we enforce this at compile time?
        self.align::<T>()?;
        let (pre, data, after) = unsafe { self.data[..bytes].align_to::<T>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        self.skip(bytes);
        Ok(data)
    }

    /// Return a fully deserialized vector of elements
    pub fn deserialize_eps_vec_deep<T: DeepCopy + DeserializeInner>(
        &mut self,
    ) -> deser::Result<Vec<<T as DeserializeInner>::DeserType<'a>>> {
        let len = usize::_deserialize_full_inner(self)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            res.push(T::_deserialize_eps_inner(self)?);
        }
        Ok(res)
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
