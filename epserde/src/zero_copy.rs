/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/**

A marker trait for data that can be zero-copy deserialized.

For a vector or boxed slice of elements of type `T` to be ε-copy serializable and
deserializable, `T` must implement `ZeroCopy`. The conditions for this marker trait are that
`T` is a copy type, that it has a fixed
memory layout, and that it does not contain any reference.

Here we implement `ZeroCopy` for all the primitive types, arrays of zero-copy types, and tuples
(up to length 10) of zero-copy types.

You can implement `ZeroCopy` for your copy types, but you must ensure that the type does not
contain references and has a fixed memory layout; for structures, this requires
`repr(C)`. ε-serde will checks for these conditions at runtime, and in case of failure
serialization/deserialization will panic.

*/
pub trait ZeroCopy: 'static {}

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl ZeroCopy for $ty {
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

impl<T: ZeroCopy, const N: usize> ZeroCopy for [T; N] {}

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: ZeroCopy,)*> ZeroCopy for ($($t,)*) {}
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
