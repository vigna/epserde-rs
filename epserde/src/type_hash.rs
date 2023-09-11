/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::hash::Hash;

/// Compute a stable hash for a type. This is used during deserialization to
/// check that the type of the data matches the type of the value being
/// deserialized into.
pub trait TypeHash {
    /// Hash the type, this considers the name, order, and type of the fields
    /// and the type of the struct.  
    fn type_hash(hasher: &mut impl core::hash::Hasher);

    /// Hash the align and size of the type, this is used to check that the
    /// type of the data matches the type of the value being deserialized into.
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher);

    /// Call type_hash on a value
    #[inline(always)]
    fn type_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_hash(hasher)
    }

    /// Call type_repr_hash on a value
    #[inline(always)]
    fn type_repr_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_repr_hash(hasher)
    }
}

// Blanket impls

impl<T: TypeHash + ?Sized> TypeHash for &'_ T {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        '&'.hash(hasher);
        T::type_hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        T::type_repr_hash(hasher)
    }
}

/*
// Core types

impl<S: TypeHash, E: TypeHash> TypeHash for Result<S, E> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Result".hash(hasher);
        S::type_hash(hasher);
        E::type_hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        core::mem::align_of::<Self>().hash(hasher);
        core::mem::size_of::<Self>().hash(hasher);
        S::type_repr_hash(hasher);
        E::type_repr_hash(hasher);
    }
}

// Primitive types


#[cfg(feature = "mmap-rs")]
impl TypeHash for mmap_rs::Mmap {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Mmap".hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        core::mem::align_of::<Self>().hash(hasher);
        core::mem::size_of::<Self>().hash(hasher);
    }
}

#[cfg(feature = "mmap-rs")]
impl TypeHash for mmap_rs::MmapMut {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "MmapMut".hash(hasher);
    }
    #[inline(always)]
    fn type_repr_hash(hasher: &mut impl core::hash::Hasher) {
        core::mem::align_of::<Self>().hash(hasher);
        core::mem::size_of::<Self>().hash(hasher);
    }
}
*/
