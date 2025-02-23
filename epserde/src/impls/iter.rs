/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for (references to) slices.

In theory all types serialized by ε-serde must not contain references. However,
we provide a convenience implementation that serializes references to
slices as vectors. Moreover, we implement [`TypeHash`] and
[`AlignHash`] for slices, so that they can be used with
[`PhantomData`](std::marker::PhantomData).

Note, however, that you must deserialize the slice as a vector,
even when it appears a type parameter—see the example
in the [crate-level documentation](crate).

*/

use core::{cell::RefCell, ops::DerefMut};

use crate::prelude::*;
use ser::*;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ZeroCopyIter<T, I: ExactSizeIterator<Item = T>>(RefCell<I>);

impl<T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = T>> ZeroCopyIter<T, I> {
    pub fn new(iter: I) -> Self {
        ZeroCopyIter(RefCell::new(iter))
    }
}

impl<T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = T>> From<I> for ZeroCopyIter<T, I> {
    fn from(iter: I) -> Self {
        ZeroCopyIter::new(iter)
    }
}

impl<T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = T>> CopyType for ZeroCopyIter<T, I> {
    type Copy = Zero;
}

impl<T: ZeroCopy + TypeHash, I: ExactSizeIterator<Item = T>> TypeHash for ZeroCopyIter<T, I> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        Vec::<T>::type_hash(hasher);
    }
}

impl<T: ZeroCopy + AlignHash, I: ExactSizeIterator<Item = T>> AlignHash for ZeroCopyIter<T, I> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        Vec::<T>::align_hash(hasher, offset_of);
    }
}

impl<T: ZeroCopy + SerializeInner + TypeHash + AlignHash, I: ExactSizeIterator<Item = T>>
    SerializeInner for ZeroCopyIter<T, I>
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    type SerType = Vec<T>;
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> Result<()> {
        check_zero_copy::<T>();

        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        backend.write("len", &len)?;
        backend.align::<T>()?;
        let mut c = 0;
        for item in iter.deref_mut() {
            serialize_zero_unchecked(backend, &item)?;
            c += 1;
        }
        if c != len {
            Err(ser::Error::WriteError)
        } else {
            Ok(())
        }
    }
}
