/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for exact-size iterators.
//!
//! In theory all types serialized by ε-serde must be immutable. However, we
//! provide a convenience implementation that serializes [exact-size
//! iterators](core::iter::ExactSizeIterator) returning references to `T` as
//! vectors of `T`.
//!
//! More precisely, we provide a [`SerIter`] type that [wraps](SerIter::new) an
//! iterator into a serializable type. We provide a [`From`] implementation for
//! convenience.
//!
//! Note, however, that you must deserialize the iterator as a vector—see the
//! example in the [crate-level documentation](crate).

use core::{cell::RefCell, ops::DerefMut};

use crate::prelude::*;
use ser::*;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SerIter<'a, T: 'a, I: ExactSizeIterator<Item = &'a T>>(RefCell<I>);

impl<'a, T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = &'a T>> SerIter<'a, T, I> {
    pub fn new(iter: I) -> Self {
        SerIter(RefCell::new(iter))
    }
}

impl<'a, T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = &'a T>> From<I> for SerIter<'a, T, I> {
    fn from(iter: I) -> Self {
        SerIter::new(iter)
    }
}

impl<'a, T: ZeroCopy + SerInner + TypeHash + AlignHash, I: ExactSizeIterator<Item = &'a T>> SerInner
    for SerIter<'a, T, I>
where
    SerIter<'a, T, I>: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T]>;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { SerHelper::_ser_inner(self, backend) }
    }
}

impl<'a, T: ZeroCopy + SerInner + TypeHash + AlignHash, I: ExactSizeIterator<Item = &'a T>>
    SerHelper<Zero> for SerIter<'a, T, I>
{
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        check_zero_copy::<T>();
        // This code must be kept aligned with that of Box<[T]> for zero-copy
        // types
        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        backend.write("len", &len)?;
        backend.align::<T>()?;

        let mut c = 0;
        for item in iter.deref_mut() {
            ser_zero_unchecked(backend, item)?;
            c += 1;
        }

        if c != len {
            Err(ser::Error::IteratorLengthMismatch {
                actual: c,
                expected: len,
            })
        } else {
            Ok(())
        }
    }
}

impl<'a, T: DeepCopy + SerInner + TypeHash + AlignHash, I: ExactSizeIterator<Item = &'a T>>
    SerHelper<Deep> for SerIter<'a, T, I>
{
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        check_mismatch::<T>();
        // This code must be kept aligned with that of Vec<T> for deep-copy
        // types
        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        backend.write("len", &len)?;

        let mut c = 0;
        for item in iter.deref_mut() {
            unsafe { item._ser_inner(backend) }?;
            c += 1;
        }

        if c != len {
            Err(ser::Error::IteratorLengthMismatch {
                actual: c,
                expected: len,
            })
        } else {
            Ok(())
        }
    }
}
