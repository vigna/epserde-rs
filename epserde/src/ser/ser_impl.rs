use crate::{IsZeroCopy, TypeName};

use super::ser::{Result, Serialize, SerializeInner, WriteWithPosNoStd};

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl SerializeInner for $ty {
            const WRITE_ALL_OPTIMIZATION: bool = true;
            type SerType<'a> = Self;

            #[inline(always)]
            fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
                backend.add_padding_to_align(core::mem::align_of::<Self>())?;
                backend.write(&self.to_ne_bytes())?;
                Ok(backend)
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl SerializeInner for () {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    type SerType<'a> = Self;

    #[inline(always)]
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        Ok(backend)
    }
}

impl SerializeInner for bool {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    type SerType<'a> = Self;
    #[inline(always)]
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        let val = if *self { 1 } else { 0 };
        backend.write(&[val])?;
        Ok(backend)
    }
}

impl SerializeInner for char {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    type SerType<'a> = Self;
    #[inline(always)]
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        (*self as u32)._serialize_inner(backend)
    }
}

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
fn serialize_slice<T: Serialize, F: WriteWithPosNoStd>(data: &[T], mut backend: F) -> Result<F> {
    let data = data.as_ref();
    let len = data.len();
    backend = backend.add_field("len", &len)?;
    if <T>::WRITE_ALL_OPTIMIZATION {
        // ensure alignment
        backend.add_padding_to_align(core::mem::align_of::<T>())?;
        let buffer = unsafe {
            core::slice::from_raw_parts(data.as_ptr() as *const u8, len * core::mem::size_of::<T>())
        };
        backend = backend.add_field_bytes(
            "data",
            T::SerType::type_name(),
            &buffer,
            core::mem::align_of::<T>(),
        )?;
    } else {
        for item in data.iter() {
            backend = backend.add_field("data", item)?;
        }
    }

    Ok(backend)
}

impl<T: Serialize + IsZeroCopy + TypeName> SerializeInner for Vec<T> {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;
    type SerType<'a> = &'a [T];

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_slice(), backend)
    }
}

impl<T: Serialize + IsZeroCopy + TypeName + ?Sized> SerializeInner for Box<[T]> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;
    type SerType<'a> = &'a [T];

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_ref(), backend)
    }
}

impl SerializeInner for Box<str> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;
    type SerType<'a> = &'a str;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_bytes(), backend)
    }
}

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;
    type SerType<'a> = &'a str;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_bytes(), backend)
    }
}

/*
impl<const N: usize, T: Serialize> SerializeInner for [T; N] {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        if <T>::WRITE_ALL_OPTIMIZATION {
            backend.add_padding_to_align(core::mem::align_of::<T>())?;
            let buffer = unsafe {
                core::slice::from_raw_parts(
                    self.as_ptr() as *const u8,
                    N * core::mem::size_of::<T>(),
                )
            };
            backend = backend.add_field_bytes(
                "data",
                Self::DeserType::type_name(),
                &buffer,
                core::mem::align_of::<T>(),
            )?;
        } else {
            for item in self.iter() {
                backend = backend.add_field("data", item)?;
            }
        }
        Ok(backend)
    }
}
 */
