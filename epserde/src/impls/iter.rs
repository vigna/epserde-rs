/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for exact-size iterators.

In theory all types serialized by ε-serde must be immutable. However,
we provide a convenience implementation that serializes
[exact-size iterators](core::iter::ExactSizeIterator) returning
references to `T` as vectors of `T`.

More precisely, we provide a [`SerIter`] type that [wraps](SerIter::new)
an iterator into a serializable type. We provide a [`From`] implementation for convenience.

Note, however, that you must deserialize the iterator as a vector—see
the example in the [crate-level documentation](crate).

*/

use core::{cell::RefCell, ops::DerefMut};

use crate::prelude::*;
use ser::*;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SerIter<'a, T: 'a, I: ExactSizeIterator<Item = &'a T>>(RefCell<I>);

impl<'a, T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = &'a T>> CopyType
    for SerIter<'a, T, I>
{
    type Copy = Deep;
}

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

impl<'a, T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = &'a T>> TypeHash
    for SerIter<'a, T, I>
{
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        Vec::<T>::type_hash(hasher);
    }
}

impl<'a, T: ZeroCopy + AlignHash, I: ExactSizeIterator<Item = &'a T>> AlignHash
    for SerIter<'a, T, I>
{
    fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        Vec::<T>::align_hash(hasher, offset_of);
    }
}

impl<
        'a,
        T: ZeroCopy + SerializeInner + TypeHash + AlignHash,
        I: ExactSizeIterator<Item = &'a T>,
    > SerializeInner for SerIter<'a, T, I>
where
    SerIter<'a, T, I>: SerializeHelper<<T as CopyType>::Copy>,
{
    type SerType = Vec<T>;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<
        'a,
        T: ZeroCopy + SerializeInner + TypeHash + AlignHash,
        I: ExactSizeIterator<Item = &'a T>,
    > SerializeHelper<Zero> for SerIter<'a, T, I>
{
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        check_zero_copy::<T>();
        // This code must be kept aligned with that of Vec<T> for zero-copy
        // types
        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        backend.write("len", &len)?;
        backend.align::<T>()?;

        let mut c = 0;
        for item in iter.deref_mut() {
            serialize_zero_unchecked(backend, item)?;
            c += 1;
        }

        if c != len {
            Err(ser::Error::IteratorLengthMismatch { actual: c, expected: len })
        } else {
            Ok(())
        }
    }
}

impl<
        'a,
        T: DeepCopy + SerializeInner + TypeHash + AlignHash,
        I: ExactSizeIterator<Item = &'a T>,
    > SerializeHelper<Deep> for SerIter<'a, T, I>
{
    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        check_mismatch::<T>();
        // This code must be kept aligned with that of Vec<T> for deep-copy
        // types
        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        backend.write("len", &len)?;

        let mut c = 0;
        for item in iter.deref_mut() {
            item._serialize_inner(backend)?;
            c += 1;
        }

        if c != len {
            Err(ser::Error::IteratorLengthMismatch { actual: c, expected: len })
        } else {
            Ok(())
        }
    }
}

