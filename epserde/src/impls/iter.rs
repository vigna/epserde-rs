
use core::{cell::RefCell, ops::DerefMut, borrow::Borrow};

use crate::prelude::*;
use ser::*;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SerIter<T, I: ExactSizeIterator>(RefCell<I>, core::marker::PhantomData<T>);

impl<T, I: ExactSizeIterator> SerIter<T, I> {
    pub fn new(iter: I) -> Self {
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
        backend.write("len", &len)?;
        backend.align::<T>()?;

        let mut c = 0;
        for item in iter.deref_mut() {
            ser_zero_unchecked(backend, item.borrow())?;
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
        backend.write("len", &len)?;

        let mut c = 0;
        for item in iter.deref_mut() {
            unsafe { item.borrow()._ser_inner(backend)? };
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
