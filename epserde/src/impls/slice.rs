/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for (references to) slices.
//!
//! In theory all types serialized by ε-serde must not contain references.
//! However, we provide a convenience implementation that serializes references
//! to slices as vectors. Moreover, we implement [`TypeHash`] and [`AlignHash`]
//! for slices, so that they can be used with
//! [`PhantomData`](std::marker::PhantomData).
//!
//! Note, however, that you must deserialize the slice as a vector, even when it
//! appears a type parameter—see the example in the [crate-level
//! documentation](crate).

use crate::prelude::*;
use ser::*;

impl<T: CopyType + SerInner + TypeHash + AlignHash> SerInner for &[T]
where
    Box<[T]>: SerializeHelper<<T as CopyType>::Copy>,
{
    type SerType = Box<[T]>;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    unsafe fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> Result<()> {
        // SAFETY: the fake boxed slice we create is never used, and we forget
        // it immediately after writing it to the backend.
        let fake = unsafe { Vec::from_raw_parts(self.as_ptr() as *mut T, self.len(), self.len()) }
            .into_boxed_slice();
        unsafe { ser::SerInner::_serialize_inner(&fake, backend) }?;
        core::mem::forget(fake);
        Ok(())
    }
}
