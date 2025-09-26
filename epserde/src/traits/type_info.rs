/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Traits computing information about a type.

use crate::pad_align_to;
use core::hash::Hash;

use super::ZeroCopy;

/// Recursively compute a type hash for a type.
///
/// [`type_hash`](TypeHash::type_hash) is a recursive function that computes
/// information about a type. It is used to check that the type of the data
/// being deserialized matches syntactically the type of the data that was
/// written.
///
/// The type hasher should store information about the name and the type of the
/// fields of a type, and the name of the type itself.
///
/// When serializing an instance of type `T`,
/// [`SerType<T>`](crate::ser::SerInner::SerType) must implement this trait.
///
/// Additionally, it is recommended that commonly used types implement this
/// trait, even if their serialized type is different, because it makes it
/// possible to use [`PhantomData`](core::marker::PhantomData) and
/// [`PhantomDeserData`](crate::PhantomDeserData).
pub trait TypeHash {
    /// Accumulates type information in `hasher`.
    fn type_hash(hasher: &mut impl core::hash::Hasher);

    /// Calls [`TypeHash::type_hash`] on a value.
    fn type_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_hash(hasher);
    }
}

/// Recursively compute an alignment hash for a type.
///
/// [`align_hash`](AlignHash::align_hash) is a recursive function that computes
/// alignment information about zero-copy types. It is used to check that the
/// alignment (and thus padding) of data that is zero-copied matches the
/// alignment at serialization time.
///
/// More precisely, at each call a zero-copy type looks at `offset_of`, assuming
/// that the type is stored at that offset in the structure, hashes in the
/// padding necessary to make `offset_of` a multiple of [`core::mem::align_of`]
/// the type, hashes in the type size, and finally increases `offset_of` by
/// [`core::mem::size_of`] the type.
///
/// All deep-copy types must implement [`AlignHash`] by calling the [`AlignHash`]
/// implementations of their fields with offset argument `&mut 0` (or a mutable
/// reference to a variable initialized to 0).
///
/// If a type has inherently no alignment requirements (e.g., all types of
/// strings), the implementation can be a no-op.
///
/// When serializing an instance of type `T`,
/// [`SerType<T>`](crate::ser::SerInner::SerType) must implement this trait.
/// Thus, if `T` different from its serialized type it is not necessary to
/// implement this trait for `T`.
pub trait AlignHash {
    /// Accumulates alignment information in `hasher` assuming to be positioned
    /// at `offset_of`.
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize);

    /// Calls [`AlignHash::align_hash`] on a value.
    fn align_hash_val(&self, hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
        Self::align_hash(hasher, offset_of);
    }
}

/// A function providing a reasonable default
/// implementation of [`AlignHash::align_hash`] for basic sized types.
pub(crate) fn std_align_hash<T: ZeroCopy>(
    hasher: &mut impl core::hash::Hasher,
    offset_of: &mut usize,
) {
    let padding = pad_align_to(*offset_of, core::mem::align_of::<T>());
    padding.hash(hasher);
    core::mem::size_of::<T>().hash(hasher);
    *offset_of += padding;
    *offset_of += core::mem::size_of::<T>();
}

/// A trait providing the desired alignment of zero-copy types in serialized
/// data.
///
/// We use the value returned by [`AlignTo::align_to`] to generate padding
/// before storing a zero-copy type. Note that this is different from the
/// padding used to align the same type inside a struct, which is not under our
/// control and is given by [`core::mem::align_of`].
///
/// The alignment returned by this function is computed by maximizing the
/// alignment required by the type itself (i.e., [`core::mem::align_of`]) and
/// the [`AlignTo::align_to`] of its fields; moreover, [`AlignTo::align_to`] of
/// primitive types is defined using the size, rather than the value returned by
/// [`core::mem::align_of`]. In this way we increase interoperability between
/// architectures with different alignment requirements for the same types
/// (e.g., 4 or 8 bytes for `u64`).
///
/// By maximizing with [`core::mem::align_of`] we ensure that we provide
/// sufficient alignment in case the attribute `repr(align(N))` was specified.
///
/// Deep-copy types do not need to implement [`AlignTo`].
pub trait AlignTo: Sized {
    fn align_to() -> usize;
}
