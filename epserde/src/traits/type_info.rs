/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Traits computing structural information about a type.

*/

use crate::pad_align_to;
use core::hash::Hash;

/// Recursively compute a type hash for a type.
///
/// [`TypeHash::type_hash`] is a recursive function that computes information
/// about a type. It is used to
/// check that the type of the data being deserialized matches
/// syntactically the type of the data that was written.
///
/// The type hasher should store information about the name and the type
/// of the fields of a type, and the name of the type itself.
pub trait TypeHash {
    fn type_hash(hasher: &mut impl core::hash::Hasher);

    /// Call [`TypeHash::type_hash`] on a value.
    fn type_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_hash(hasher);
    }
}

/// Recursively compute a representational hash for a type.
///
/// [`ReprHash::repr_hash`] is a recursive function that computes
/// representation information about a zero-copy type. It is used to
/// check that the the alignment and the representation data
/// of the data being deserialized.
///
/// More precisely, at each call a zero-copy type look at `offset_of`,
/// assuming that the type is stored at that offset in the structure,
/// hashes in the padding necessary to make `offset_of` a multiple of
/// [`core::mem::align_of`] the type, hashes in the type size, and
/// finally increases `offset_of` by [`core::mem::size_of`] the type.
///
/// For practical reasons, [`ReprHash`] must be implemented also for deep-copy
/// types as a no-op.
pub trait ReprHash {
    fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}

    /// Call [`ReprHash::repr_hash`] on a value.
    fn repr_hash_val(&self, hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        Self::repr_hash(hasher, offset_of);
    }
}

/// A function providing a reasonable default
/// implementation of [`ReprHash::repr_hash`].
pub(crate) fn std_repr_hash<T>(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
    let padding = pad_align_to(*offset_of, core::mem::align_of::<T>());
    padding.hash(hasher);
    core::mem::size_of::<T>().hash(hasher);
    *offset_of += padding;
    *offset_of += core::mem::size_of::<T>();
}

/// A trait giving the maximum size of a primitive field in a type.
///
/// We use the value returned by [`MaxSizeOf::max_size_of`]
/// to generate padding before storing a zero-copy type. Note that this
/// is different from the padding used to align the same type inside
/// a struct, which is not under our control and is
/// given by [`core::mem::align_of`].
///
/// In this way we increase interoperability between architectures
/// with different alignment requirements for the same types (e.g.,
/// 4 or 8 bytes for `u64`).
pub trait MaxSizeOf: Sized {
    fn max_size_of() -> usize;
}
