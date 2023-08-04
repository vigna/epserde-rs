use crate::des::*;
use crate::CheckAlignment;

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl DeserializeInner for $ty {
            #[inline(always)]
            fn _deserialize_full_copy_inner<'a>(mut backend:Cursor<'a>) -> Result<(Self,Cursor<'a>), DeserializeError> {
                backend = <$ty>::check_alignment(backend)?;
                Ok((
                    <$ty>::from_ne_bytes(backend.data[..core::mem::size_of::<$ty>()].try_into().unwrap()),
                    backend.skip(core::mem::size_of::<$ty>()),
                ))
            }
            type DeserType<'b> = $ty;
            #[inline(always)]
            fn _deserialize_eps_copy_inner<'a>(
                backend: Cursor<'a>,
            ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
                <$ty as DeserializeInner>::_deserialize_full_copy_inner(backend)
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl DeserializeInner for () {
    #[inline(always)]
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Ok(((), backend))
    }
    type DeserType<'a> = Self;
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Self::_deserialize_full_copy_inner(backend)
    }
}

impl DeserializeInner for bool {
    #[inline(always)]
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Ok((backend.data[0] != 0, backend.skip(1)))
    }
    type DeserType<'a> = Self;
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Self::_deserialize_full_copy_inner(backend)
    }
}

impl DeserializeInner for char {
    #[inline(always)]
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        u32::_deserialize_full_copy_inner(backend).map(|(x, y)| (char::from_u32(x).unwrap(), y))
    }
    type DeserType<'a> = Self;
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Self::_deserialize_full_copy_inner(backend)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[inline]
fn deserialize_slice<'a, T>(
    backend: Cursor<'a>,
) -> Result<(&'a [T], Cursor<'a>), DeserializeError> {
    let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
    let bytes = len * core::mem::size_of::<T>();
    // a slice can only be deserialized with zero copy
    // outerwise you need a vec, TODO!: how do we enforce this at compile time?
    backend = <T>::check_alignment(backend)?;
    let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
    debug_assert!(pre.is_empty());
    debug_assert!(after.is_empty());
    Ok((data, backend.skip(bytes)))
}

impl<T: DeserializeInner + 'static> DeserializeInner for Vec<T> {
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, new_backend) = T::_deserialize_full_copy_inner(backend)?;
            res.push(elem);
            backend = new_backend;
        }
        Ok((res, backend))
    }
    type DeserType<'c> = &'c [T];
    #[inline(always)]
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        deserialize_slice(backend)
    }
}

impl<T: DeserializeInner + 'static> DeserializeInner for Box<[T]> {
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        <Vec<T>>::_deserialize_full_copy_inner(backend).map(|(d, a)| (d.into_boxed_slice(), a))
    }
    type DeserType<'c> = &'c [T];
    #[inline(always)]
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        deserialize_slice(backend)
    }
}

impl DeserializeInner for String {
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        let (len, mut backend) = usize::_deserialize_full_copy_inner(backend)?;
        let data = &backend.data[..len];
        backend.data = &backend.data[len..];
        let res = String::from_utf8(data.to_vec()).unwrap();
        Ok((res, backend))
    }
    type DeserType<'c> = &'c str;
    #[inline(always)]
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        let (slice, backend) = deserialize_slice(backend)?;
        Ok((
            unsafe { core::mem::transmute::<&'a [u8], &'a str>(slice) },
            backend,
        ))
    }
}

impl DeserializeInner for Box<str> {
    fn _deserialize_full_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self, Cursor<'a>), DeserializeError> {
        String::_deserialize_full_copy_inner(backend).map(|(d, a)| (d.into_boxed_str(), a))
    }
    type DeserType<'c> = &'c str;
    #[inline(always)]
    fn _deserialize_eps_copy_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        let (slice, backend) = deserialize_slice(backend)?;
        Ok((
            unsafe { core::mem::transmute::<&'a [u8], &'a str>(slice) },
            backend,
        ))
    }
}
