/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::slice;
use std::io::{Read, Seek, SeekFrom, Write};

use maligned::{Alignment, A16};

pub struct AlignedCursor<T: Alignment = A16> {
    vec: Vec<T>,
    pos: usize,
    len: usize,
}

impl<T: Alignment> AlignedCursor<T> {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            pos: 0,
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity.div_ceil(std::mem::size_of::<T>())),
            pos: 0,
            len: 0,
        }
    }

    pub fn as_bytes(&mut self) -> &[u8] {
        let ptr = self.vec.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts(ptr, self.len) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = self.vec.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts_mut(ptr, self.len) }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
}

impl<T: Alignment> Default for AlignedCursor<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Alignment> Read for AlignedCursor<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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
        let len = buf.len();
        // TODO: minimize with usize::MAX?
        let rem = self.vec.len() * std::mem::size_of::<T>() - self.pos;
        if rem < buf.len() {
            self.vec.resize(
                (self.pos + len).div_ceil(std::mem::size_of::<T>()),
                T::default(),
            );
        }

        let position = self.pos;

        // SAFETY: we now have enough space in the vec.
        let bytes = unsafe {
            slice::from_raw_parts_mut(
                self.vec.as_mut_ptr() as *mut u8,
                self.vec.len() * std::mem::size_of::<T>(),
            )
        };
        bytes[position..position + len].copy_from_slice(buf);
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
                    "invalid seek to a position greater than usize::MAX",
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
