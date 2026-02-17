/*
 * SPDX-FileCopyrightText: 2025 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Blanket implementations for references and smart pointers.
//!
//! This module provides blanket implementations of serialization traits for
//! (mutable) references. Moreover, it provides (de)serialization support for
//! [`Box`], [`Rc`] and [`Arc`] if the `std` or `alloc` feature is enabled.
//!
//! While references have the obvious semantics (we serialize the referred
//! value), smart pointers are supported by erasure: if a type parameter has
//! value `Box<T>`, `Rc<T>`, or `Arc<T>`, we serialize `T` in its place (with
//! the exception of boxed slices, which [have their own
//! treatment](crate::impls::boxed_slice)).
//!
//! Upon deserialization, if the type parameter is `T` we deserialize `T`, but
//! if it is `Box<T>`, `Rc<T>`, or `Arc<T>` we deserialize `T` and then wrap
//! it in the appropriate smart pointer.
//!
//! In particular, this means that it is always possible to wrap in a smart pointer
//! type parameters, even if the serialized data did not come from a smart pointer.
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
use crate::prelude::*;
use ser::*;

macro_rules! impl_ser {
    ($type:ty) => {
        impl<T: SerInner> SerInner for $type {
            type SerType = T::SerType;
            const IS_ZERO_COPY: bool = <T as SerInner>::IS_ZERO_COPY;

            #[inline(always)]
            unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                unsafe { <T as SerInner>::_ser_inner(self, backend) }
            }
        }
    };
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
            fn __check_covariance<'__long: '__short, '__short>(
                p: deser::CovariantProof<Self::DeserType<'__long>>,
            ) -> deser::CovariantProof<Self::DeserType<'__short>> {
                // SAFETY: Box/Rc/Arc are covariant in T, and T::DeserType is
                // covariant (enforced by T's own __check_covariance).
                unsafe { core::mem::transmute(p) }
            }

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
