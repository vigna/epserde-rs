/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

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

    /// Call [`TypeHash::type_hash`] on a value.
    #[inline(always)]
    fn type_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_hash(hasher)
    }

    /// Call [`TypeHash::type_repr_hash`] on a value.
    #[inline(always)]
    fn type_repr_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_repr_hash(hasher)
    }
}
