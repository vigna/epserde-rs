/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::{CopySelector, CopyType, Eps, EpsCopy, TypeHash, Zero, ZeroCopy};

use super::ser::{FieldWrite, Result, Serialize, SerializeInner};

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
fn serialize_zero_copy<T: Serialize, F: FieldWrite>(data: &T, backend: F) -> Result<F> {
    let buffer = unsafe {
        #[allow(clippy::manual_slice_size_calculation)]
        core::slice::from_raw_parts(data as *const T as *const u8, core::mem::size_of::<T>())
    };
    backend.add_field_bytes::<T>("data", buffer)
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
    backend = backend.add_field_align("len", &len)?;
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
        backend.add_field_bytes::<T>("data", buffer)
    } else {
        if T::ZERO_COPY_MISMATCH {
            eprintln!("Type {} is zero copy, but it has not declared as such; use the #full_copy attribute to silence this warning", core::any::type_name::<T>());
        }
        for item in data.iter() {
            backend = backend.add_field_align("data", item)?;
        }
        Ok(backend)
    }
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
            backend = backend.add_field_align("data", item)?;
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
