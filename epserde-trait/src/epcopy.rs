pub trait IsEpCopy {}

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl IsEpCopy for $ty {
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

impl<T: IsEpCopy> IsEpCopy for Option<T> {}
impl<R: IsEpCopy, E: IsEpCopy> IsEpCopy for Result<R, E> {}
impl<T: IsEpCopy, const N: usize> IsEpCopy for [T; N] {}

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: IsEpCopy,)*> IsEpCopy for ($($t,)*) {}
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
