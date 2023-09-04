/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Internal trait used to select whether a type is zero copy or not.
/// It has only two implementations, [`Eps`] and [`Zero`].
pub trait CopySelector {
    const IS_ZERO_COPY: bool;
}
/// An implementation of a [`CopySelector`] specifying that a type is zero copy.
pub struct Zero {}

impl CopySelector for Zero {
    const IS_ZERO_COPY: bool = true;
}

/// An implementation of a [`CopySelector`] specifying that a type is not zero copy.
pub struct Eps {}

impl CopySelector for Eps {
    const IS_ZERO_COPY: bool = false;
}

/**

Marker trait for data specifying whether it can be zero-copy deserialized.

The trait comes in two flavors: `CopySelector<Type=Zero>` and
`CopySelector<Type=Eps>`. To each of these flavors corresponds two
dependent traits, [`ZeroCopy`] and [`EpsCopy`], which are automatically
implemented:
```rust
use epserde::*;

struct MyType {}

impl CopyType for MyType {
    type Copy = Zero;
}
// Now MyType implements ZeroCopy
```

We use this trait to implement a different behavior for [`ZeroCopy`] and [`EpsCopy`] types
on arrays, vectors, and boxed slices,
[working around the bug that prevents the compiler from understanding that implementations
for the two flavors of `CopySelector` are mutually
exclusive](https://github.com/rust-lang/rfcs/pull/1672#issuecomment-1405377983).

For a slice of elements of type `T` to be zero-copy serializable and
deserializable, `T` must implement `CopySelector<Type=Zero>`. The conditions for this marker trait are that
`T` is a copy type, that it has a fixed memory layout, and that it does not contain any reference.
If this happen, a value of type `T` can be zero-copy deserialized just by taking a reference, and
consequently vectors of `T` or boxed slices of `T` can be ε-copy deserialized
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
pub trait EpsCopy: CopyType<Copy = Eps> {}
impl<T: CopyType<Copy = Eps>> EpsCopy for T {}

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl CopyType for $ty {
            type Copy = Zero;
        }
    )*};
}

impl_stuff!(
    (),
    bool,
    char,
    isize,
    i8,
    i16,
    i32,
    i64,
    i128,
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    f32,
    f64
);

impl<T: CopyType, const N: usize> CopyType for [T; N] {
    type Copy = T::Copy;
}

/// TODO
macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: ZeroCopy,)*> CopyType for ($($t,)*)  {
            /// TODO
            type Copy = Zero;
        }
    };
}

macro_rules! impl_tuples_muncher {
    ($ty:ident, $($t:ident),*) => {
        impl_tuples!($ty, $($t),*);
        impl_tuples_muncher!($($t),*);
    };
    ($ty:ident) => {
        impl_tuples!($ty);
    };
    () => {};
}

impl_tuples_muncher!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);

impl<T> CopyType for Vec<T> {
    type Copy = Eps;
}

impl<T> CopyType for Box<[T]> {
    type Copy = Eps;
}

impl<T> CopyType for Option<T> {
    type Copy = Eps;
}

impl<R, E> CopyType for Result<R, E> {
    type Copy = Eps;
}

impl CopyType for String {
    type Copy = Eps;
}

impl CopyType for Box<str> {
    type Copy = Eps;
}
