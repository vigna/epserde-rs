/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::traits::TypeHash;
use core::hash::Hash;

/// Internal trait used to select whether a type is zero copy or not.
/// It has only two implementations, [`Eps`] and [`Zero`].
pub trait CopySelector: TypeHash {
    const IS_ZERO_COPY: bool;
}
/// An implementation of a [`CopySelector`] specifying that a type is zero copy.
pub struct Zero {}

impl TypeHash for Zero {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        0xdeadbeefdeadf00d_u64.hash(hasher);
    }
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        0xabadcafefadc0c0a_u64.hash(hasher);
    }
}

impl CopySelector for Zero {
    const IS_ZERO_COPY: bool = true;
}

/// An implementation of a [`CopySelector`] specifying that a type is not zero copy.
#[derive(Hash)]
pub struct Full {}

impl TypeHash for Full {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        0xc0ffec0ffe_u64.hash(hasher);
    }
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        0xc0ffec0ffe_u64.hash(hasher);
    }
}

impl CopySelector for Full {
    const IS_ZERO_COPY: bool = false;
}

/**

Marker trait for data specifying whether it can be zero-copy deserialized.

The trait comes in two flavors: `CopySelector<Type=Zero>` and
`CopySelector<Type=Eps>`. To each of these flavors corresponds two
dependent traits, [`ZeroCopy`] and [`FullCopy`], which are automatically
implemented:
```rust
use epserde::traits::*;

struct MyType {}

impl CopyType for MyType {
    type Copy = Zero;
}
// Now MyType implements ZeroCopy
```

We use this trait to implement a different behavior for [`ZeroCopy`] and [`FullCopy`] types
on arrays, vectors, and boxed slices,
[working around the bug that prevents the compiler from understanding that implementations
for the two flavors of `CopySelector` are mutually
exclusive](https://github.com/rust-lang/rfcs/pull/1672#issuecomment-1405377983).

For an array of elements of type `T` to be zero-copy serializable and
deserializable, `T` must implement `CopySelector<Type=Zero>`. The conditions for this marker trait are that
`T` is a copy type, that it has a fixed memory layout, and that it does not contain any reference.
If this happen vectors of `T` or boxed slices of `T` can be ε-copy deserialized
using a reference to a slice of `T`.

You can implement `CopySelector<Type=Zero>` for your copy types, but you must ensure that the type does not
contain references and has a fixed memory layout; for structures, this requires
`repr(C)`. ε-serde will track these conditions at compile time and check them at
runtime: in case of failure, serialization will panic.

Since we cannot use negative trait bounds, every type that is used as a parameter of
an array, vector or boxed slice must implement either `CopySelector<Type=Zero>` or `CopySelector<Type=Eps>`. In the latter
case, slices will be deserialized element by element, and the result will be a fully
deserialized vector or boxed
slice. If you do not implement either of these traits, the type will not be serializable inside
vectors or boxed slices but error messages will be very unhelpful due to the
contrived way we implement mutually exclusive types.

If you use the provided derive macros all this logic will be hidden from you. You'll
just have to add `#[zero_copy]` to your structures (if you want them to be zero-copy)
and ε-serde will do the rest.

*/

pub trait CopyType {
    type Copy: CopySelector;
}

/// Marker trait for zero-copy types. You should never implement
/// this trait manually, but rather implement [`CopyType`] with `Copy=Zero`.
pub trait ZeroCopy: CopyType<Copy = Zero> {}
impl<T: CopyType<Copy = Zero>> ZeroCopy for T {}

/// Marker trait for non zero-copy types. You should never implement
/// this trait manually, but rather implement [`CopyType`] with `Copy=Eps`.
pub trait FullCopy: CopyType<Copy = Full> {}
impl<T: CopyType<Copy = Full>> FullCopy for T {}
