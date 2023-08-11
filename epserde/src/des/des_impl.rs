/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::des::*;
use crate::Align;
use crate::CopySelector;
use crate::CopyType;
use crate::Eps;
use crate::EpsCopy;
use crate::Zero;
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

fn deserialize_slice<T: ZeroCopy>(backend: Cursor) -> Result<(&'_ [T], Cursor), DeserializeError> {
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

mod private {
    use super::*;

    fn deserialize_vec_eps<T: DeserializeInner>(
        backend: Cursor,
    ) -> Result<(Vec<<T as DeserializeInner>::DeserType<'_>>, Cursor), DeserializeError> {
        let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, new_backend) = T::_deserialize_eps_copy_inner(backend)?;
            res.push(elem);
            backend = new_backend;
        }
        Ok((res, backend))
    }

    fn deserialize_vec_full<T: DeserializeInner>(
        backend: Cursor,
    ) -> Result<(Vec<T>, Cursor), DeserializeError> {
        let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, new_backend) = T::_deserialize_full_copy_inner(backend)?;
            res.push(elem);
            backend = new_backend;
        }
        Ok((res, backend))
    }
    // Since impls with distinct parameters are considered disjoint
    // we can write multiple blanket impls for DeserializeHelper given different paremeters
    pub trait DeserializeHelper<T: CopySelector> {
        // TODO: do we really need this?
        type FullType: TypeHash;
        type DeserType<'a>: TypeHash;
        fn _deserialize_full_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(Self::FullType, Cursor), DeserializeError>;
        fn _deserialize_eps_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(Self::DeserType<'_>, Cursor), DeserializeError>;
    }

    // This delegates to a private helper trait which we can specialize on in stable rust
    impl<T: CopyType + DeserializeInner + 'static> DeserializeInner for Vec<T>
    where
        Vec<T>: DeserializeHelper<<T as CopyType>::Type, FullType = Vec<T>>,
    {
        type DeserType<'a> = <Vec<T> as DeserializeHelper<<T as CopyType>::Type>>::DeserType<'a>;
        fn _deserialize_full_copy_inner(
            backend: Cursor,
        ) -> Result<(Vec<T>, Cursor), DeserializeError> {
            <Vec<T> as DeserializeHelper<<T as CopyType>::Type>>::_deserialize_full_copy_inner_impl(
                backend,
            )
        }

        fn _deserialize_eps_copy_inner(
            backend: Cursor,
        ) -> Result<
            (
                <Vec<T> as DeserializeHelper<<T as CopyType>::Type>>::DeserType<'_>,
                Cursor,
            ),
            DeserializeError,
        > {
            <Vec<T> as DeserializeHelper<<T as CopyType>::Type>>::_deserialize_eps_copy_inner_impl(
                backend,
            )
        }
    }

    impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero> for Vec<T> {
        type FullType = Self;
        type DeserType<'a> = &'a [T];
        fn _deserialize_full_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(Vec<T>, Cursor), DeserializeError> {
            deserialize_vec_full(backend)
        }
        #[inline(always)]
        fn _deserialize_eps_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(<Self as DeserializeInner>::DeserType<'_>, Cursor), DeserializeError> {
            deserialize_slice(backend)
        }
    }

    impl<T: EpsCopy + DeserializeInner + 'static> DeserializeHelper<Eps> for Vec<T> {
        type FullType = Self;
        type DeserType<'a> = Vec<<T as DeserializeInner>::DeserType<'a>>;
        fn _deserialize_full_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(Self, Cursor), DeserializeError> {
            deserialize_vec_full(backend)
        }
        #[inline(always)]
        fn _deserialize_eps_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(<Self as DeserializeInner>::DeserType<'_>, Cursor), DeserializeError> {
            deserialize_vec_eps::<T>(backend)
        }
    }

    // This delegates to a private helper trait which we can specialize on in stable rust
    impl<T: CopyType + DeserializeInner + 'static> DeserializeInner for Box<[T]>
    where
        Box<[T]>: DeserializeHelper<<T as CopyType>::Type, FullType = Box<[T]>>,
    {
        type DeserType<'a> = <Box<[T]> as DeserializeHelper<<T as CopyType>::Type>>::DeserType<'a>;
        fn _deserialize_full_copy_inner(
            backend: Cursor,
        ) -> Result<(Box<[T]>, Cursor), DeserializeError> {
            <Box<[T]> as DeserializeHelper<<T as CopyType>::Type>>::_deserialize_full_copy_inner_impl(
            backend,
        )
        }

        fn _deserialize_eps_copy_inner(
            backend: Cursor,
        ) -> Result<
            (
                <Box<[T]> as DeserializeHelper<<T as CopyType>::Type>>::DeserType<'_>,
                Cursor,
            ),
            DeserializeError,
        > {
            <Box<[T]> as DeserializeHelper<<T as CopyType>::Type>>::_deserialize_eps_copy_inner_impl(
                backend,
            )
        }
    }

    impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero> for Box<[T]> {
        type FullType = Self;
        type DeserType<'a> = &'a [T];
        fn _deserialize_full_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(Box<[T]>, Cursor), DeserializeError> {
            deserialize_vec_full(backend).map(|(v, b)| (v.into_boxed_slice(), b))
        }
        #[inline(always)]
        fn _deserialize_eps_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(<Self as DeserializeInner>::DeserType<'_>, Cursor), DeserializeError> {
            deserialize_slice(backend)
        }
    }

    impl<T: EpsCopy + DeserializeInner + 'static> DeserializeHelper<Eps> for Box<[T]> {
        type FullType = Self;
        type DeserType<'a> = Box<[<T as DeserializeInner>::DeserType<'a>]>;
        fn _deserialize_full_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(Self, Cursor), DeserializeError> {
            deserialize_vec_full(backend).map(|(v, b)| (v.into_boxed_slice(), b))
        }
        #[inline(always)]
        fn _deserialize_eps_copy_inner_impl(
            backend: Cursor,
        ) -> Result<(<Self as DeserializeInner>::DeserType<'_>, Cursor), DeserializeError> {
            deserialize_vec_eps::<T>(backend).map(|(v, b)| (v.into_boxed_slice(), b))
        }
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
