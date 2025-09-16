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

use core::hash::Hash;

use crate::prelude::*;
use ser::*;

impl<T: TypeHash> TypeHash for [T] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T> CopyType for &[T] {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for &[T] {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        Vec::<T>::type_hash(hasher);
    }
}

impl<T: AlignHash> AlignHash for &[T] {
    fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        Vec::<T>::align_hash(hasher, offset_of);
    }
}

impl<T: CopyType + SerializeInner + TypeHash + AlignHash> SerializeInner for &[T]
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    type SerType = Vec<T>;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> Result<()> {
        // SAFETY: the fake vector we create is never used, and we forget it immediately
        // after writing it to the backend.
        let fake = unsafe { Vec::from_raw_parts(self.as_ptr() as *mut T, self.len(), self.len()) };
        unsafe { ser::SerializeInner::_serialize_inner(&fake, backend) }?;
        core::mem::forget(fake);
        Ok(())
    }
}
