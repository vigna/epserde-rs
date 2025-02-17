/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for slices.

Slices cannot be serialized in isolation, but they must implement [`TypeHash`] and
[`ReprHash`] so that they can be used with [`PhantomData`](std::marker::PhantomData).

We also provide a serialize-only (slightly cheaty) implementation
for slices that deserializes to vectors.

It is slightly cheaty in that it serializes a vector using the
slice as a backing array, so it must be deserialized using a vector as type.

Note that if you ε-copy deserialize the vector, you will
get back the same slice.
```rust
use epserde::prelude::*;
use maligned::A16;
let a = vec![1, 2, 3, 4];
let s = a.as_slice();
let mut cursor = <AlignedCursor<A16>>::new();
s.serialize(&mut cursor).unwrap();
cursor.set_position(0);
let b: Vec<i32> = <Vec<i32>>::deserialize_full(&mut cursor).unwrap();
assert_eq!(a, b);
let b: &[i32] = <Vec<i32>>::deserialize_eps(cursor.as_bytes()).unwrap();
assert_eq!(a, *b);
```

*/

use crate::prelude::*;
use ser::*;
use std::hash::Hash;

impl<T> CopyType for &[T] {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for &[T] {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T> ReprHash for &[T] {
    #[inline(always)]
    fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl<T: SerializeInner + CopyType + TypeHash + ReprHash> SerializeInner for &[T]
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    type SerType = Vec<T>;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> Result<()> {
        // SAFETY: the fake vector we create is never used, and we forget it immediately
        // after writing it to the backend.
        let fake = unsafe { Vec::from_raw_parts(self.as_ptr() as *mut T, self.len(), self.len()) };
        ser::SerializeInner::_serialize_inner(&fake, backend)?;
        core::mem::forget(fake);
        Ok(())
    }
}
