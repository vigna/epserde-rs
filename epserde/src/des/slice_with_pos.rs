/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::traits::{FullCopy, ZeroCopy};
use crate::{des, DeserializeError, DeserializeInner, ReadNoStd, ReadWithPos};

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

    pub fn skip(&self, bytes: usize) -> Self {
        Self {
            data: &self.data[bytes..],
            pos: self.pos + bytes,
        }
    }

    /// Return a reference, backed by the `data` field, to a zero-copy type.
    pub fn deserialize_eps_zero<T: ZeroCopy>(mut self) -> des::Result<(&'a T, Self)> {
        let bytes = core::mem::size_of::<T>();
        // a slice can only be deserialized with zero copy
        // outerwise you need a vec, TODO!: how do we enforce this at compile time?
        self = self.align::<T>()?;
        let (pre, data, after) = unsafe { self.data[..bytes].align_to::<T>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        Ok((&data[0], self.skip(bytes)))
    }

    /// Return a reference, backed by the `data` field,
    /// to a slice whose elements are of zero-copy type.
    pub fn deserialize_slice_zero<T: ZeroCopy>(self) -> des::Result<(&'a [T], Self)> {
        let (len, mut res_self) = usize::_deserialize_full_copy_inner(self)?;
        let bytes = len * core::mem::size_of::<T>();
        // a slice can only be deserialized with zero copy
        // outerwise you need a vec, TODO!: how do we enforce this at compile time?
        res_self = res_self.align::<T>()?;
        let (pre, data, after) = unsafe { res_self.data[..bytes].align_to::<T>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        Ok((data, res_self.skip(bytes)))
    }

    /// Return a fully deserialized vector of elements
    pub fn deserialize_vec_eps_eps<T: FullCopy + DeserializeInner>(
        self,
    ) -> des::Result<(Vec<<T as DeserializeInner>::DeserType<'a>>, Self)> {
        let (len, mut res_self) = usize::_deserialize_full_copy_inner(self)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, temp_self) = T::_deserialize_eps_copy_inner(res_self)?;
            res.push(elem);
            res_self = temp_self;
        }
        Ok((res, res_self))
    }
}

impl<'a> ReadNoStd for SliceWithPos<'a> {
    fn read_exact(&mut self, buf: &mut [u8]) -> des::Result<()> {
        let len = buf.len();
        if len > self.data.len() {
            return Err(DeserializeError::ReadError);
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
    fn align<T>(mut self) -> des::Result<Self> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, core::mem::align_of::<T>());
        self = self.skip(padding);
        // Check that the ptr is indeed aligned
        if self.data.as_ptr() as usize % std::mem::align_of::<T>() != 0 {
            Err(DeserializeError::AlignmentError)
        } else {
            Ok(self)
        }
    }
}
