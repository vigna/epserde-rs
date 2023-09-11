/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::marker::PhantomData;

use crate::{CopySelector, CopyType, Eps, EpsCopy, TypeHash, Zero, ZeroCopy};

use super::ser::{FieldWrite, Result, Serialize, SerializeInner};

macro_rules! impl_stuff{
    ($($ty:ty),*) => {$(
        impl SerializeInner for $ty {
            // here we are lying :)
            // primitive types are zero copy, but we won't return
            // a reference to them
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
                backend.write(&self.to_ne_bytes())?;
                Ok(backend)
            }
        }
    )*};
}

impl_stuff!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
fn serialize_zero_copy<T: Serialize, F: FieldWrite>(data: &T, mut backend: F) -> Result<F> {
    let buffer = unsafe {
        #[allow(clippy::manual_slice_size_calculation)]
        core::slice::from_raw_parts(data as *const T as *const u8, core::mem::size_of::<T>())
    };
    backend = backend.add_field_bytes(
        "data",
        core::any::type_name::<T>().to_string(),
        buffer,
        core::mem::align_of::<T>(),
    )?;

    Ok(backend)
}

impl SerializeInner for () {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        Ok(backend)
    }
}

impl SerializeInner for bool {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
        let val = if *self { 1 } else { 0 };
        backend.write(&[val])?;
        Ok(backend)
    }
}

impl SerializeInner for char {
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        (*self as u32)._serialize_inner(backend)
    }
}

impl<T> CopyType for Option<T> {
    type Copy = Eps;
}

impl<T: SerializeInner> SerializeInner for Option<T> {
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
        match self {
            None => {
                backend = backend.add_field("Tag", &0_u8)?;
            }
            Some(val) => {
                backend = backend.add_field("Tag", &1_u8)?;
                backend = backend.add_field("Some", val)?;
            }
        };
        Ok(backend)
    }
}

impl<T> CopyType for PhantomData<T> {
    type Copy = Zero;
}

impl<T: SerializeInner> SerializeInner for PhantomData<T> {
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        Ok(backend)
    }
}

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
fn serialize_slice<T: Serialize, F: FieldWrite>(
    data: &[T],
    mut backend: F,
    zero_copy: bool,
) -> Result<F> {
    // TODO: check for IS_ZERO_COPY
    let len = data.len();
    backend = backend.add_field("len", &len)?;
    if zero_copy {
        if !T::IS_ZERO_COPY {
            panic!(
                "Cannot serialize non zero-copy type {} declared as zero copy",
                core::any::type_name::<T>()
            );
        }
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
        if T::ZERO_COPY_MISMATCH {
            eprintln!("Type {} is zero copy, but it has not declared as such; use the #full_copy attribute to silence this warning", core::any::type_name::<T>());
        }
        for item in data.iter() {
            backend = backend.add_field("data", item)?;
        }
    }

    Ok(backend)
}

// Since impls with distinct parameters are considered disjoint
// we can write multiple blanket impls for SerializeHelper given different paremeters
trait SerializeHelper<T: CopySelector> {
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F>;
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + SerializeInner + TypeHash, const N: usize> SerializeInner for [T; N]
where
    [T; N]: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = T::IS_ZERO_COPY;
    const ZERO_COPY_MISMATCH: bool = T::ZERO_COPY_MISMATCH;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner, const N: usize> SerializeHelper<Zero> for [T; N] {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_zero_copy(self, backend)
    }
}

impl<T: EpsCopy + SerializeInner, const N: usize> SerializeHelper<Eps> for [T; N] {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
        for item in self.iter() {
            backend = backend.add_field("data", item)?;
        }
        Ok(backend)
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Vec<T>
where
    Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Vec<T> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_slice(), backend, true)
    }
}

impl<T: EpsCopy + SerializeInner> SerializeHelper<Eps> for Vec<T> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_slice(), backend, false)
    }
}

// This delegates to a private helper trait which we can specialize on in stable rust
impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Box<[T]>
where
    Box<[T]>: SerializeHelper<<T as CopyType>::Copy>,
{
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        SerializeHelper::_serialize_inner(self, backend)
    }
}

impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self, backend, true)
    }
}

impl<T: EpsCopy + SerializeInner> SerializeHelper<Eps> for Box<[T]> {
    #[inline(always)]
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self, backend, false)
    }
}

impl SerializeInner for Box<str> {
    // Box<[$ty]> can, but Vec<Box<[$ty]>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_bytes(), backend, true)
    }
}

impl SerializeInner for String {
    // Vec<$ty> can, but Vec<Vec<$ty>> cannot!
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F> {
        serialize_slice(self.as_bytes(), backend, true)
    }
}
