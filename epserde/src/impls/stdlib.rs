/*
 * SPDX-FileCopyrightText: 2023 Tommaso Fontana
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementation of traits for struts from the std library
use crate::prelude::*;
use core::hash::Hash;

impl TypeHash for std::hash::DefaultHasher {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "std::hash::DefaultHasher".hash(hasher);
    }
}

impl ReprHash for std::hash::DefaultHasher {
    fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        crate::traits::std_repr_hash::<Self>(hasher, offset_of)
    }
}

impl MaxSizeOf for std::hash::DefaultHasher {
    fn max_size_of() -> usize {
        core::mem::size_of::<Self>()
    }
}
