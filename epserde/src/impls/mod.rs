/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations of [`SerializeInner`](crate::ser::SerializeInner)
and [`DeserializeInner`](crate::des::DeserializeInner) for standard Rust types.

*/

pub mod array;
pub mod boxed_slice;
pub mod prim;
pub mod slice;
pub mod string;
pub mod tuple;
pub mod vec;

use core::hash::Hash;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;

use crate::prelude::TypeHash;

#[cfg(feature = "alloc")]
impl<T: TypeHash + ?Sized> TypeHash for Box<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box".hash(hasher);
        T::type_hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        T::type_repr_hash(hasher);
    }
}
