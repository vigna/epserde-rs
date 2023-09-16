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

use crate::prelude::{CopyType, Deep, TypeHash};

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

impl<T> CopyType for Box<T> {
    type Copy = Deep;
}

#[cfg(feature = "alloc")]
impl<T: TypeHash + ?Sized> TypeHash for Box<T> {
    fn type_hash(
        type_hasher: &mut impl core::hash::Hasher,
        repr_hasher: &mut impl core::hash::Hasher,
        offset_of: &mut usize,
    ) {
        "Box".hash(type_hasher);
        T::type_hash(type_hasher, repr_hasher, offset_of);
    }
}
