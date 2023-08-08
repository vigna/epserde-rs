/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::des::*;
use crate::Align;
use crate::ZeroCopy;

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl DeserializeInner for $ty {
            #[inline(always)]
            fn _deserialize_full_copy_inner(mut backend:Cursor) -> Result<(Self,Cursor), DeserializeError> {
                backend = <$ty>::pad_align_and_check(backend)?;
                Ok((
                    <$ty>::from_ne_bytes(backend.data[..core::mem::size_of::<$ty>()].try_into().unwrap()),
                    backend.skip(core::mem::size_of::<$ty>()),
                ))
            }
            type DeserType<'a> = $ty;
            #[inline(always)]
            fn _deserialize_eps_copy_inner(
                backend: Cursor,
            ) -> Result<(Self::DeserType<'_>, Cursor), DeserializeError> {
                <$ty as DeserializeInner>::_deserialize_full_copy_inner(backend)
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl DeserializeInner for () {
    #[inline(always)]
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        Ok(((), backend))
    }
    type DeserType<'a> = Self;
    fn _deserialize_eps_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        Self::_deserialize_full_copy_inner(backend)
    }
}

impl DeserializeInner for bool {
    #[inline(always)]
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        Ok((backend.data[0] != 0, backend.skip(1)))
    }
    type DeserType<'a> = Self;
    fn _deserialize_eps_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        Self::_deserialize_full_copy_inner(backend)
    }
}

impl DeserializeInner for char {
    #[inline(always)]
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        u32::_deserialize_full_copy_inner(backend).map(|(x, y)| (char::from_u32(x).unwrap(), y))
    }
    type DeserType<'a> = Self;
    fn _deserialize_eps_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        Self::_deserialize_full_copy_inner(backend)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[inline]
fn deserialize_slice<T>(backend: Cursor) -> Result<(&'_ [T], Cursor), DeserializeError> {
    let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
    let bytes = len * core::mem::size_of::<T>();
    // a slice can only be deserialized with zero copy
    // outerwise you need a vec, TODO!: how do we enforce this at compile time?
    backend = <T>::pad_align_and_check(backend)?;
    let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
    debug_assert!(pre.is_empty());
    debug_assert!(after.is_empty());
    Ok((data, backend.skip(bytes)))
}

impl<T: DeserializeInner + ZeroCopy + 'static> DeserializeInner for Vec<T> {
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, new_backend) = T::_deserialize_full_copy_inner(backend)?;
            res.push(elem);
            backend = new_backend;
        }
        Ok((res, backend))
    }
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: Cursor,
    ) -> Result<(Self::DeserType<'_>, Cursor), DeserializeError> {
        deserialize_slice(backend)
    }
}

impl<T: DeserializeInner + ZeroCopy + 'static> DeserializeInner for Box<[T]> {
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        <Vec<T>>::_deserialize_full_copy_inner(backend).map(|(d, a)| (d.into_boxed_slice(), a))
    }
    type DeserType<'a> = &'a [T];
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: Cursor,
    ) -> Result<(Self::DeserType<'_>, Cursor), DeserializeError> {
        deserialize_slice(backend)
    }
}

impl DeserializeInner for String {
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
        let data = &backend.data[..len];
        backend.data = &backend.data[len..];
        let res = String::from_utf8(data.to_vec()).unwrap();
        Ok((res, backend))
    }
    type DeserType<'a> = &'a str;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: Cursor,
    ) -> Result<(Self::DeserType<'_>, Cursor), DeserializeError> {
        let (slice, backend) = deserialize_slice(backend)?;
        Ok((
            unsafe {
                #[allow(clippy::transmute_bytes_to_str)]
                core::mem::transmute::<&'_ [u8], &'_ str>(slice)
            },
            backend,
        ))
    }
}

impl DeserializeInner for Box<str> {
    fn _deserialize_full_copy_inner(backend: Cursor) -> Result<(Self, Cursor), DeserializeError> {
        String::_deserialize_full_copy_inner(backend).map(|(d, a)| (d.into_boxed_str(), a))
    }
    type DeserType<'a> = &'a str;
    #[inline(always)]
    fn _deserialize_eps_copy_inner(
        backend: Cursor,
    ) -> Result<(Self::DeserType<'_>, Cursor), DeserializeError> {
        let (slice, backend) = deserialize_slice(backend)?;
        Ok((
            #[allow(clippy::transmute_bytes_to_str)]
            unsafe {
                core::mem::transmute::<&'_ [u8], &'_ str>(slice)
            },
            backend,
        ))
    }
}

macro_rules! impl_deser_vec {
    ($ty:ty) => {
        impl<T: DeserializeInner + ZeroCopy + 'static> DeserializeInner for Vec<$ty> {
            #[inline(always)]
            fn _deserialize_full_copy_inner(
                backend: Cursor,
            ) -> core::result::Result<(Self, Cursor), DeserializeError> {
                // read the len
                let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
                let mut vec = Vec::with_capacity(len);
                // deserialize every subvector
                for _ in 0..len {
                    let (sub_vec, tmp) = <$ty>::_deserialize_full_copy_inner(backend)?;
                    backend = tmp;
                    vec.push(sub_vec);
                }

                Ok((vec, backend))
            }
            /// This is the return type of the ε-copy deserialization.
            type DeserType<'a> = Vec<<$ty as DeserializeInner>::DeserType<'a>>;

            fn _deserialize_eps_copy_inner(
                backend: Cursor,
            ) -> std::result::Result<(Self::DeserType<'_>, Cursor), DeserializeError> {
                // read the len
                let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
                let mut vec = Vec::with_capacity(len);
                // deserialize every subvector but using ε-copy!
                for _ in 0..len {
                    let (sub_vec, tmp) = <$ty>::_deserialize_eps_copy_inner(backend)?;
                    backend = tmp;
                    vec.push(sub_vec);
                }

                Ok((vec, backend))
            }
        }
    };
}

impl_deser_vec!(Vec<T>);
impl_deser_vec!(Vec<Vec<T>>);
