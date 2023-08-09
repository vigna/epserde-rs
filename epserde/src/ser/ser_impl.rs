/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::{CopySelector, CopyType, Eps, EpsCopy, TypeHash, Zero, ZeroCopy};

use super::ser::{FieldWrite, Result, Serialize, SerializeInner};

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl SerializeInner for $ty {
            const IS_ZERO_COPY: bool = true;

            #[inline(always)]
            fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
                backend.add_padding_to_align(core::mem::align_of::<Self>())?;
                backend.write(&self.to_ne_bytes())?;
                Ok(backend)
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

impl SerializeInner for () {
    const IS_ZERO_COPY: bool = true;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        Ok(backend)
    }
}

impl SerializeInner for bool {
    const IS_ZERO_COPY: bool = true;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
        let val = if *self { 1 } else { 0 };
        backend.write(&[val])?;
        Ok(backend)
    }
}

impl SerializeInner for char {
    const IS_ZERO_COPY: bool = true;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        (*self as u32)._serialize_inner(backend)
    }
}

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
fn serialize_slice<T: Serialize, F: FieldWrite>(data: &[T], mut backend: F) -> Result<F> {
    let len = data.len();
    backend = backend.add_field("len", &len)?;
    if <T>::IS_ZERO_COPY {
        // ensure alignment
        backend.add_padding_to_align(core::mem::align_of::<T>())?;
        let buffer = unsafe {
            #[allow(clippy::manual_slice_size_calculation)]
            core::slice::from_raw_parts(data.as_ptr() as *const u8, len * core::mem::size_of::<T>())
        };
        backend = backend.add_field_bytes(
            "data",
            core::any::type_name::<T>().to_string(),
            buffer,
            core::mem::align_of::<T>(),
        )?;
    } else {
        for item in data.iter() {
            backend = backend.add_field("data", item)?;
        }
    }

    Ok(backend)
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Vec<T>
where
    Vec<T>: SerializeHelper<<T as CopyType>::Type>,
{
    const IS_ZERO_COPY: bool = false;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

// Since impls with distinct parameters are considered disjoint
// we can write multiple blanket impls for DeserializeHelper given different paremeters
trait SerializeHelper<T: CopySelector> {
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F>;
}

// blanket impl 1
impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Vec<T> {
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_slice(), backend)
    }
}

// blanket impl 2
impl<T: EpsCopy + SerializeInner> SerializeHelper<Eps> for Vec<T> {
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_slice(), backend)
    }
}

impl SerializeInner for Box<str> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_bytes(), backend)
    }
}

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const IS_ZERO_COPY: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_bytes(), backend)
    }
}

macro_rules! impl_ser_vec {
    ($ty:ty) => {
        impl<T: SerializeInner + ZeroCopy + TypeHash> SerializeInner for Vec<$ty> {
            /// This type cannot be serialized just by writing its bytes
            const IS_ZERO_COPY: bool = false;
            /// We will read back this as a vec of slices

            fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
                // write the number of sub-fields
                backend = backend.add_field("len", &self.len())?;
                for (i, sub_vec) in self.iter().enumerate() {
                    // serialize each sub-vector
                    backend = backend.add_field(&format!("sub_vec_{}", i), sub_vec)?;
                }

                Ok(backend)
            }
        }
    };
}

pub trait SerializeInnerRef<T: SerializeInner>: AsRef<[T]> {
    /// This type cannot be serialized just by writing its bytes
    const IS_ZERO_COPY: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
        // write the number of sub-fields
        backend = backend.add_field("len", &self.as_ref().len())?;
        for (i, sub_vec) in self.as_ref().iter().enumerate() {
            // serialize each sub-vector
            backend = backend.add_field(&format!("sub_vec_{}", i), sub_vec)?;
        }

        Ok(backend)
    }
}
