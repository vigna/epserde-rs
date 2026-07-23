/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Implementations for exact-size iterators.
//!
//! In theory all types serialized by ε-serde must be immutable. However, we
//! provide a convenience implementation that serializes [exact-size iterators]
//! returning elements that are [`Borrow<T>`] for some type `T` as boxed
//! slices of `T`.
//!
//! More precisely, we provide a [`SerIter`] type that [wraps] an iterator into
//! a type that can be serialized. We provide a [`From`] implementation for
//! convenience.
//!
//! Note, however, that you must deserialize the iterator as a vector or a
//! boxed slice; see the
//! example in the [crate-level documentation].
//!
//! [exact-size iterators]: core::iter::ExactSizeIterator
//! [`Borrow<T>`]: core::borrow::Borrow
//! [wraps]: SerIter::new
//! [crate-level documentation]: crate
use core::{borrow::Borrow, cell::RefCell, ops::DerefMut};

use crate::prelude::*;
use ser::*;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

/// A wrapper serializing an exact-size iterator as a boxed slice.
///
/// Note that serialization consumes the wrapped iterator: serializing the
/// same [`SerIter`] a second time serializes an empty sequence, since the
/// exhausted iterator reports a length of zero.
///
/// For more information, see the [module-level documentation].
///
/// [module-level documentation]: crate::impls::iter
pub struct SerIter<T, I: ExactSizeIterator>(RefCell<I>, core::marker::PhantomData<T>);

impl<T, I: ExactSizeIterator + core::fmt::Debug> core::fmt::Debug for SerIter<T, I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Print the wrapped iterator, not the internal RefCell.
        match self.0.try_borrow() {
            Ok(iter) => f.debug_tuple("SerIter").field(&&*iter).finish(),
            Err(_) => f
                .debug_tuple("SerIter")
                .field(&format_args!("<borrowed>"))
                .finish(),
        }
    }
}

impl<T, I: ExactSizeIterator> SerIter<T, I> {
    pub const fn new(iter: I) -> Self {
        SerIter(RefCell::new(iter), core::marker::PhantomData)
    }
}

impl<T, I: ExactSizeIterator> From<I> for SerIter<T, I> {
    fn from(iter: I) -> Self {
        SerIter::new(iter)
    }
}

impl<T, I> SerInner for SerIter<T, I>
where
    I: ExactSizeIterator,
    I::Item: Borrow<T>,
    T: CopyType + SerInner,
    Self: SerHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T::SerType]>;
    const IS_ZERO_COPY: bool = false;
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { <Self as SerHelper<<T as CopyType>::Copy>>::_ser_inner(self, backend) }
    }
}

impl<T, I> SerHelper<Zero> for SerIter<T, I>
where
    I: ExactSizeIterator,
    I::Item: Borrow<T>,
    T: ZeroCopy,
{
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        check_zero_copy::<T>();
        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        unsafe { backend.write("len", &len) }?;
        backend.align::<T>()?;

        let mut c = 0;
        for item in iter.deref_mut() {
            // Stop before writing anything beyond the declared length, so
            // that an iterator whose len underestimates the actual number of
            // items cannot write unboundedly (the reported actual count is
            // then a lower bound).
            if c == len {
                c += 1;
                break;
            }
            unsafe { ser_zero_unchecked(backend, item.borrow()) }?;
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

impl<T, I> SerHelper<Deep> for SerIter<T, I>
where
    I: ExactSizeIterator,
    I::Item: Borrow<T>,
    T: DeepCopy,
{
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        let mut iter = self.0.borrow_mut();
        let len = iter.len();
        unsafe { backend.write("len", &len) }?;

        let mut c = 0;
        for item in iter.deref_mut() {
            // Stop before writing anything beyond the declared length, so
            // that an iterator whose len underestimates the actual number of
            // items cannot write unboundedly (the reported actual count is
            // then a lower bound).
            if c == len {
                c += 1;
                break;
            }
            // We go through write so that schema-recording backends see the
            // same per-item structure as the serialization of a vector.
            unsafe { backend.write("item", item.borrow())? };
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
