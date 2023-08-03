/**

A marker trait for data that can be zero-copy deserialized.

For a slice of elements of type `T` to be ε-copy deserializable, `T` must implement `IsZeroCopy`.
The conditions for the marker trait is that `T` is a copy type, that it has a fixed
memory layout, and that it does not contain any reference.

Here we implement `IsZeroCopy` for all the primitive types, arrays of zero-copy types and tuples
(up to length 10) of zero-copy types.

You can implement `IsZeroCopy` for your own copy types, but you must ensure that the type is does not
contain references and has a fixed memory layout; for structures, this requires
`repr(C)`.

*/
pub trait IsZeroCopy: 'static {}

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl IsZeroCopy for $ty {
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

impl<T: IsZeroCopy, const N: usize> IsZeroCopy for [T; N] {}

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: IsZeroCopy,)*> IsZeroCopy for ($($t,)*) {}
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
