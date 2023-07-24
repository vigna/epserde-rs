use super::des::*;

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl DeserializeNature for $ty {
            const ZC_TYPE: bool = true;
            const ZC_SUB_TYPE: bool = true;
        }

        impl<const ZC_SUB_TYPE: bool> DeserializeZeroCopy<ZC_SUB_TYPE> for $ty {
            type DesType<'b> = Self;

            #[inline(always)]
            fn deserialize_zero_copy_inner<'a>(backend: &'a [u8]) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
                Self::deserialize_full_copy_inner(backend)
            }
        }

        impl DeserializeFullCopy for $ty {
            #[inline(always)]
            fn deserialize_full_copy_inner<'a>(backend: &'a [u8]) -> Result<(Self, &'a [u8]), DeserializeError> {
                <$ty>::check_alignement(backend)?;
                Ok((
                    <$ty>::from_ne_bytes(backend[..core::mem::size_of::<$ty>()].try_into().unwrap()),
                    &backend[core::mem::size_of::<$ty>()..],
                ))
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl DeserializeNature for () {
    const ZC_TYPE: bool = true;
    const ZC_SUB_TYPE: bool = true;
}

impl<const ZC_SUB_TYPE: bool> DeserializeZeroCopy<ZC_SUB_TYPE> for () {
    type DesType<'b> = Self;

    #[inline(always)]
    fn deserialize_zero_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        Self::deserialize_full_copy_inner(backend)
    }
}

impl DeserializeFullCopy for () {
    #[inline(always)]
    fn deserialize_full_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self, &'a [u8]), DeserializeError> {
        Ok(((), backend))
    }
}

impl DeserializeNature for bool {
    const ZC_TYPE: bool = true;
    const ZC_SUB_TYPE: bool = true;
}

impl<const ZC_SUB_TYPE: bool> DeserializeZeroCopy<ZC_SUB_TYPE> for bool {
    type DesType<'b> = Self;

    #[inline(always)]
    fn deserialize_zero_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        Self::deserialize_full_copy_inner(backend)
    }
}

impl DeserializeFullCopy for bool {
    #[inline(always)]
    fn deserialize_full_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self, &'a [u8]), DeserializeError> {
        Ok((backend[0] != 0, &backend[1..]))
    }
}

impl DeserializeNature for char {
    const ZC_TYPE: bool = true;
    const ZC_SUB_TYPE: bool = true;
}

impl<const ZC_SUB_TYPE: bool> DeserializeZeroCopy<ZC_SUB_TYPE> for char {
    type DesType<'b> = Self;

    #[inline(always)]
    fn deserialize_zero_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        Self::deserialize_full_copy_inner(backend)
    }
}

impl DeserializeFullCopy for char {
    #[inline(always)]
    fn deserialize_full_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self, &'a [u8]), DeserializeError> {
        u32::deserialize_full_copy_inner(backend).map(|(x, y)| (char::from_u32(x).unwrap(), y))
    }
}

impl<'a, T> DeserializeNature for &'a [T] {
    const ZC_TYPE: bool = true;
    const ZC_SUB_TYPE: bool = true;
}

impl<T: DeserializeNature> DeserializeNature for Vec<T> {
    const ZC_TYPE: bool = false;
    const ZC_SUB_TYPE: bool = T::ZC_TYPE;
}

impl<'c, T: 'static> DeserializeZeroCopy<true> for &'c [T] {
    type DesType<'b> = &'b [T];

    #[inline(always)]
    fn deserialize_zero_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        let (len, backend) = usize::deserialize_full_copy_inner(backend)?;
        let bytes = len * core::mem::size_of::<T>();
        // a slice can only be deserialized with zero copy
        // outerwise you need a vec, TODO!: how do we enforce this at compile time?
        //<T>::check_alignement(backend)?;
        let (pre, data, after) = unsafe { backend[..bytes].align_to::<T>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        Ok((data, &backend[bytes..]))
    }
}

impl<T: DeserializeNature + 'static> DeserializeZeroCopy<false> for Vec<T> {
    type DesType<'b> = &'b [T];
    #[inline(always)]
    fn deserialize_zero_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        let (len, backend) = usize::deserialize_full_copy_inner(backend)?;
        let bytes = len * core::mem::size_of::<T>();
        // a slice can only be deserialized with zero copy
        // outerwise you need a vec, TODO!: how do we enforce this at compile time?
        <T>::check_alignement(backend)?;
        let (pre, data, after) = unsafe { backend[..bytes].align_to::<T>() };
        debug_assert!(pre.is_empty());
        debug_assert!(after.is_empty());
        Ok((data, &backend[bytes..]))
    }
}

impl<T: DeserializeFullCopy> DeserializeFullCopy for Vec<T> {
    #[inline(always)]
    fn deserialize_full_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self, &'a [u8]), DeserializeError> {
        // we have to iter and deserialize each element :(
        let (len, backend) = usize::deserialize_full_copy_inner(backend)?;
        let mut data = Vec::with_capacity(len);
        let mut backend = backend;
        for _ in 0..len {
            let (elem, new_backend) = T::deserialize_full_copy_inner(backend)?;
            data.push(elem);
            backend = new_backend;
        }
        Ok((data, backend))
    }
}
