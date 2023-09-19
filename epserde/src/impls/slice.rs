/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Serialize-only (slightly cheaty) implementation for slices that deserializes to vectors.

This implementation of [`Serialize`] for slices is slightly cheaty, as it actually
serializes a vector using the slice as a backing array, so it must be deserialized
using a vector as type.

Note that if you ε-copy deserialize the vector, you will
get back the same slice.
```rust
use epserde::prelude::*;
let a = vec![1, 2, 3, 4];
let mut cursor = epserde::new_aligned_cursor();
a.serialize(&mut cursor).unwrap();
cursor.set_position(0);
let b: Vec<i32> = <Vec<i32>>::deserialize_full(&mut cursor).unwrap();
assert_eq!(a, b);
let buf = cursor.into_inner();
let b: &[i32] = <Vec<i32>>::deserialize_eps(&buf).unwrap();
assert_eq!(a, *b);
```
*/
use crate::prelude::*;
use ser::*;

impl<T: SerializeInner + CopyType> Serialize for &[T]
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    fn serialize_on_field_write(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        write_header::<Vec<T>>(backend)?;
        // SAFETY: the fake vector we create is never used, and we forget it immediately
        // after writing it to the backend.
        let fake = unsafe { Vec::from_raw_parts(self.as_ptr() as *mut T, self.len(), self.len()) };
        backend.serialize("ROOT", &fake)?;
        core::mem::forget(fake);
        backend.flush()
    }
}
