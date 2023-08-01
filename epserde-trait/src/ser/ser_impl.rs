use crate::{CheckAlignement, TypeName};

use super::ser::{Result, Serialize, SerializeInner, WriteWithPosNoStd};

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl SerializeInner for $ty {
            const WRITE_ALL_OPTIMIZATION: bool = true;

            #[inline(always)]
            fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
                backend = <$ty>::pad_align_to::<F>(backend)?;
                backend.write(&self.to_ne_bytes())?;
                Ok(backend)
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl SerializeInner for () {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    #[inline(always)]
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        Ok(backend)
    }
}

impl SerializeInner for bool {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    #[inline(always)]
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        let val = if *self { 1 } else { 0 };
        backend.write(&[val])?;
        Ok(backend)
    }
}

impl SerializeInner for char {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    #[inline(always)]
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        (*self as u32)._serialize_inner(backend)
    }
}

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
fn serialize_slice<T: Serialize, V: AsRef<[T]>, F: WriteWithPosNoStd>(
    data: V,
    mut backend: F,
) -> Result<F> {
    let data = data.as_ref();
    let len = data.len();
    backend = backend.add_field("len", &len)?;
    if <T>::WRITE_ALL_OPTIMIZATION {
        // ensure alignement
        backend = <T>::pad_align_to(backend)?;
        let buffer = unsafe {
            core::slice::from_raw_parts(data.as_ptr() as *const u8, len * core::mem::size_of::<T>())
        };
        backend = backend.add_field_bytes("data", data.type_name_val(), &buffer)?;
    } else {
        for item in data.iter() {
            backend = backend.add_field("data", item)?;
        }
    }

    Ok(backend)
}

impl<T: Serialize> SerializeInner for Vec<T> {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        serialize_slice(self, backend)
    }
}

/*
impl<T: Serialize> SerializeInner for Box<[T]> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        backend.add_type(&self.type_name_val())?;
        serialize_slice(self, backend)
    }
}
*/

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const WRITE_ALL_OPTIMIZATION: bool = false;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F> {
        serialize_slice(self, backend)
    }
}

impl<const N: usize, T: Serialize> SerializeInner for [T; N] {
    const WRITE_ALL_OPTIMIZATION: bool = true;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        if <T>::WRITE_ALL_OPTIMIZATION {
            backend = <T>::pad_align_to(backend)?;
            let buffer = unsafe {
                core::slice::from_raw_parts(
                    self.as_ptr() as *const u8,
                    N * core::mem::size_of::<T>(),
                )
            };
            backend = backend.add_field_bytes("data", Self::type_name(), &buffer)?;
        } else {
            for item in self.iter() {
                backend = backend.add_field("data", item)?;
            }
        }
        Ok(backend)
    }
}
