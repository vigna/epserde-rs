/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::ser::{FieldWrite, Result, Serialize};

/// this is a private function so we have a consistent implementation
/// and slice can't be generally serialized
pub fn serialize_slice<T: Serialize, F: FieldWrite>(
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
