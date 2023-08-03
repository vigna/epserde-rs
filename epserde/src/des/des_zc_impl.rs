use crate::des::*;
use crate::{CheckAlignment, IsZeroCopy};

macro_rules! impl_zc_stuff{
    ($($ty:ty),*) => {$(
        impl DeserializeEpsCopyInner for $ty {
            type DeserType<'b> = $ty;
            #[inline(always)]
            fn deserialize_zc_inner<'a>(
                backend: Cursor<'a>,
            ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
                <$ty as DeserializeInner>::deserialize_inner(backend)
            }
        }
    )*
};
}

impl_zc_stuff!(
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

#[inline]
fn deserialize_slice<'a, T>(
    backend: Cursor<'a>,
) -> Result<(&'a [T], Cursor<'a>), DeserializeError> {
    let (len, mut backend) = usize::deserialize_inner(backend)?;
    let bytes = len * core::mem::size_of::<T>();
    // a slice can only be deserialized with zero copy
    // outerwise you need a vec, TODO!: how do we enforce this at compile time?
    backend = <T>::check_alignment(backend)?;
    let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
    debug_assert!(pre.is_empty());
    debug_assert!(after.is_empty());
    Ok((data, backend.skip(bytes)))
}

impl<T: 'static + IsZeroCopy + TypeName> DeserializeEpsCopyInner for Box<[T]> {
    type DeserType<'c> = &'c [T];
    #[inline(always)]
    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        deserialize_slice(backend)
    }
}

impl<T: 'static + IsZeroCopy + TypeName> DeserializeEpsCopyInner for Vec<T> {
    type DeserType<'c> = &'c [T];
    #[inline(always)]
    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        deserialize_slice(backend)
    }
}

impl DeserializeEpsCopyInner for String {
    type DeserType<'c> = &'c str;
    #[inline(always)]
    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        let (slice, backend) = deserialize_slice(backend)?;
        Ok((
            unsafe { core::mem::transmute::<&'a [u8], &'a str>(slice) },
            backend,
        ))
    }
}

impl DeserializeEpsCopyInner for Box<str> {
    type DeserType<'c> = &'c str;
    #[inline(always)]
    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError> {
        let (slice, backend) = deserialize_slice(backend)?;
        Ok((
            unsafe { core::mem::transmute::<&'a [u8], &'a str>(slice) },
            backend,
        ))
    }
}
