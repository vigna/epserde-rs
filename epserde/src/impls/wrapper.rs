/*
 * SPDX-FileCopyrightText: 2025 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for references and wrappers.
//!
//! This module provides implementations of serialization traits for (mutable)
//! references. Moreover, it provides (de)serialization support for [`Box`],
//! [`Rc`] and [`Arc`] if the `std` or `alloc` feature is enabled.
//!
//! While references have the obvious semantics (we serialize the referred
//! value), wrappers are supported by erasure: if a type parameter has value
//! `Box<T>`, `Rc<T>`, or `Arc<T>`, we serialize `T` in its place (with the
//! exception of boxed slices and `Box<str>`, which have their own treatment
//! in [`boxed_slice`] and [`string`]).
//!
//! Upon deserialization, if the type parameter is `T` we deserialize `T`, but
//! if it is `Box<T>`, `Rc<T>`, or `Arc<T>` we deserialize `T` and then wrap
//! it in the appropriate smart pointer.
//!
//! In particular, this means that it is always possible to wrap in a smart
//! pointer type parameters, even if the serialized data did not come from a
//! smart pointer.
//!
//! We also provide an implementation of [`TypeHash`] for `*const T`, which is
//! useful to write tuples in [`PhantomData`] with unsized type parameters, such
//! as `PhantomData<(*const T, U)>` when `T` is unsized.
//!
//! # Examples
//!
//! In this example we serialize a vector wrapped in an [`Rc`], but then we
//! deserialize it as a plain vector, or even wrapped with an [`Arc`]:
//!
//! ```
//! # use epserde::prelude::*;
//! # use std::rc::Rc;
//! # use std::sync::Arc;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let v = vec![1, 2, 3, 4, 5];
//! let mut cursor = <AlignedCursor<Aligned16>>::new();
//! unsafe { Rc::new(v).serialize(&mut cursor)?; }
//! // Rc is erased
//! cursor.set_position(0);
//! let _no_rc: Vec<i32> = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
//!
//! // In fact, we can deserialize wrapping in any smart pointer
//! cursor.set_position(0);
//! let _no_rc_but_arc: Arc<Vec<i32>> =
//!     unsafe { <Arc<Vec<i32>>>::deserialize_full(&mut cursor)? };
//! # Ok(())
//! # }
//! ```
//!
//! The same is true of fields, provided that their type is a type parameter:
//! ```
//! # use epserde::prelude::*;
//! # use std::rc::Rc;
//! # use std::sync::Arc;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #[derive(Epserde)]
//! struct Data<A>(A);
//! let data = Data(Rc::new(vec![1, 2, 3, 4, 5]));
//! let mut cursor = <AlignedCursor<Aligned16>>::new();
//! unsafe { data.serialize(&mut cursor)?; }
//! // Rc is erased
//! cursor.set_position(0);
//! let _no_rc: Data<Vec<i32>> = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
//!
//! // In fact, we can deserialize wrapping in any smart pointer
//! cursor.set_position(0);
//! let _no_rc_but_arc: Data<Arc<Vec<i32>>> =
//!     unsafe { <Data<Arc<Vec<i32>>>>::deserialize_full(&mut cursor)? };
//! # Ok(())
//! # }
//! ```
//!
//! [`boxed_slice`]: crate::impls::boxed_slice
//! [`string`]: crate::impls::string
//! [`PhantomData`]: core::marker::PhantomData

use crate::prelude::*;
use core::hash::Hash;
use ser::*;

macro_rules! impl_ser {
    ($type:ty) => {
        impl<T: SerInner> SerInner for $type {
            type SerType = T::SerType;
            // Wrappers and references are never zero-copy, regardless of T:
            // they carry a pointer and do not implement CopyType.
            const IS_ZERO_COPY: bool = false;

            #[inline(always)]
            unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                unsafe { <T as SerInner>::_ser_inner(self, backend) }
            }
        }
    };
}

// For use with PhantomData<(*const T, ...)>, with T unsized

impl<T: ?Sized + TypeHash> TypeHash for *const T {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "*const".hash(hasher);
        T::type_hash(hasher);
    }
}

impl_ser!(&T);
impl_ser!(&mut T);

#[cfg(not(feature = "std"))]
mod imports {
    pub use alloc::boxed::Box;
    pub use alloc::rc::Rc;
    pub use alloc::sync::Arc;
}
#[cfg(feature = "std")]
mod imports {
    pub use std::rc::Rc;
    pub use std::sync::Arc;
}
use imports::*;

macro_rules! impl_all {
    ($type:ident) => {
        impl_ser!($type<T>);

        impl<T: DeserInner> DeserInner for $type<T> {
            type DeserType<'a> = $type<DeserType<'a, T>>;
            // SAFETY: Box/Rc/Arc are covariant in T, and T::DeserType is
            // covariant (enforced by T's own __check_covariance).
            crate::unsafe_assume_covariance!(T);

            #[inline(always)]
            unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
                unsafe { <T as DeserInner>::_deser_full_inner(backend).map($type::new) }
            }
            #[inline(always)]
            unsafe fn _deser_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                unsafe { <T as DeserInner>::_deser_eps_inner(backend).map($type::new) }
            }
        }
    };
}

impl_all!(Box);
impl_all!(Arc);
impl_all!(Rc);
