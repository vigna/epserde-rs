use crate::des::*;
use crate::CheckAlignement;

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl DeserializeInner for $ty {
            #[inline(always)]
            fn deserialize_inner<'a>(mut backend:Cursor<'a>) -> Result<(Self,Cursor<'a>), DeserializeError> {
                backend = <$ty>::check_alignement(backend)?;
                Ok((
                    <$ty>::from_ne_bytes(backend.data[..core::mem::size_of::<$ty>()].try_into().unwrap()),
                    backend.skip(core::mem::size_of::<$ty>()),
                ))
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl DeserializeInner for () {
    #[inline(always)]
    fn deserialize_inner<'a>(backend: Cursor<'a>) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Ok(((), backend))
    }
}

impl DeserializeInner for bool {
    #[inline(always)]
    fn deserialize_inner<'a>(backend: Cursor<'a>) -> Result<(Self, Cursor<'a>), DeserializeError> {
        Ok((backend.data[0] != 0, backend.skip(1)))
    }
}

impl DeserializeInner for char {
    #[inline(always)]
    fn deserialize_inner<'a>(backend: Cursor<'a>) -> Result<(Self, Cursor<'a>), DeserializeError> {
        u32::deserialize_inner(backend).map(|(x, y)| (char::from_u32(x).unwrap(), y))
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<T: DeserializeInner> DeserializeInner for Vec<T> {
    fn deserialize_inner<'a>(backend: Cursor<'a>) -> Result<(Self, Cursor<'a>), DeserializeError> {
        let (len, mut backend) = usize::deserialize_inner(backend)?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            let (elem, new_backend) = T::deserialize_inner(backend)?;
            res.push(elem);
            backend = new_backend;
        }
        Ok((res, backend))
    }
}
