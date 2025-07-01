/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::slice;
use std::io::{Read, Seek, SeekFrom, Write};

use maligned::{Alignment, A16};
use mem_dbg::{MemDbg, MemSize};

/// An aligned version of [`Cursor`](std::io::Cursor).
///
/// The standard library implementation of a [cursor](std::io::Cursor) is not
/// aligned, and thus cannot be used to create examples or unit tests for
/// ε-serde. This version has a [settable alignment](maligned::Alignment) that
/// is guaranteed to be respected by the underlying storage.
///
/// Note that length and position are stored as `usize` values, so the maximum
/// length and position are `usize::MAX`. This is different from
/// [`Cursor`](std::io::Cursor), which uses a `u64`.
#[derive(Debug, Clone, MemDbg, MemSize)]
pub struct AlignedCursor<T: Alignment = A16> {
    vec: Vec<T>,
    pos: usize,
    len: usize,
}

impl<T: Alignment> AlignedCursor<T> {
    /// Return a new empty [`AlignedCursor`].
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            pos: 0,
            len: 0,
        }
    }

    /// Return a new empty [`AlignedCursor`] with a specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity.div_ceil(std::mem::size_of::<T>())),
            pos: 0,
            len: 0,
        }
    }

    /// Consume this cursor, returning the underlying storage and the length of
    /// the data in bytes.
    pub fn into_parts(self) -> (Vec<T>, usize) {
        (self.vec, self.len)
    }

    /// Return a reference to the underlying storage as bytes.
    ///
    /// Only the first [len](AlignedCursor::len) bytes are valid.
    ///
    /// Note that the reference is always to the whole storage,
    /// independently of the current [position](AlignedCursor::position).
    pub fn as_bytes(&mut self) -> &[u8] {
        let ptr = self.vec.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts(ptr, self.len) }
    }

    /// Return a mutable reference to the underlying storage as bytes.
    ///
    /// Only the first [len](AlignedCursor::len) bytes are valid.
    ///
    /// Note that the reference is always to the whole storage,
    /// independently of the current [position](AlignedCursor::position).
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = self.vec.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts_mut(ptr, self.len) }
    }

    /// Return the length in bytes of the data in this cursor.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return whether this cursor contains no data.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the current position of this cursor.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Set the current position of this cursor.
    ///
    /// Valid positions are all `usize` values.
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Set the length of this cursor.
    ///
    /// The underlying vector will be enlarged if necessary.
    pub fn set_len(&mut self, len: usize) {
        if len > self.vec.len() * std::mem::size_of::<T>() {
            self.vec
                .resize(len.div_ceil(std::mem::size_of::<T>()), T::default());
        }
        self.len = len;
    }
}

impl<T: Alignment> Default for AlignedCursor<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Alignment> Read for AlignedCursor<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let pos = self.pos;
        let rem = self.len - pos;
        let to_copy = std::cmp::min(buf.len(), rem) as usize;
        buf[..to_copy].copy_from_slice(&self.as_bytes()[pos..pos + to_copy]);
        self.pos += to_copy;
        Ok(to_copy)
    }
}

impl<T: Alignment> Write for AlignedCursor<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = buf.len().min(usize::MAX - self.pos);
        if !buf.is_empty() && len == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "write operation overflows usize::MAX length limit",
            ));
        }

        let cap = self.vec.len().saturating_mul(std::mem::size_of::<T>());
        let rem = cap - self.pos;
        if rem < len {
            self.vec.resize(
                (self.pos + len).div_ceil(std::mem::size_of::<T>()),
                T::default(),
            );
        }

        let pos = self.pos;

        // SAFETY: we now have enough space in the vec.
        let bytes = unsafe {
            slice::from_raw_parts_mut(
                self.vec.as_mut_ptr() as *mut u8,
                self.vec.len() * std::mem::size_of::<T>(),
            )
        };
        bytes[pos..pos + len].copy_from_slice(buf);
        self.pos += len;
        self.len = self.len.max(self.pos);
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<T: Alignment> Seek for AlignedCursor<T> {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) if n > usize::MAX as u64 => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "cursor length would be greater than usize::MAX",
                ))
            }
            SeekFrom::Start(n) => {
                self.pos = n as usize;
                return Ok(self.pos as u64);
            }
            SeekFrom::End(n) => (self.len as u64, n),
            SeekFrom::Current(n) => (self.pos as u64, n),
        };

        match base_pos.checked_add_signed(offset) {
            Some(n) if n <= usize::MAX as u64 => {
                self.pos = n as usize;
                Ok(n)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.pos as u64)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_aligned_cursor() -> Result<(), Box<dyn Error>> {
        let mut cursor = AlignedCursor::<A16>::new();
        for i in 0_usize..1000 {
            cursor.write_all(&i.to_ne_bytes()).unwrap();
        }

        for i in (0..1000).rev() {
            let mut buf = [0; std::mem::size_of::<usize>()];
            cursor.set_position(i * std::mem::size_of::<usize>());
            cursor.read_exact(&mut buf).unwrap();
            assert_eq!(i.to_ne_bytes(), buf);
        }

        for i in (0..1000).rev() {
            let mut buf = [0; std::mem::size_of::<usize>()];
            let pos = cursor.seek(SeekFrom::Start(i * std::mem::size_of::<usize>() as u64))?;
            assert_eq!(pos, cursor.position() as u64);
            cursor.read_exact(&mut buf).unwrap();
            assert_eq!(i.to_ne_bytes(), buf);
        }

        for i in (0..1000).rev() {
            let mut buf = [0; std::mem::size_of::<usize>()];
            let pos = cursor.seek(SeekFrom::End(
                (-i - 1) * std::mem::size_of::<usize>() as i64,
            ))?;
            assert_eq!(pos, cursor.position() as u64);
            cursor.read_exact(&mut buf).unwrap();
            assert_eq!((999 - i).to_ne_bytes(), buf);
        }

        cursor.set_position(0);

        for i in 0_usize..500 {
            let mut buf = [0; std::mem::size_of::<usize>()];
            let pos = cursor.seek(SeekFrom::Current(std::mem::size_of::<usize>() as i64))?;
            assert_eq!(pos, cursor.position() as u64);
            cursor.read_exact(&mut buf).unwrap();
            assert_eq!((i * 2 + 1).to_ne_bytes(), buf);
        }

        assert!(cursor
            .seek(SeekFrom::End(-1001 * std::mem::size_of::<usize>() as i64,))
            .is_err());

        Ok(())
    }
}
