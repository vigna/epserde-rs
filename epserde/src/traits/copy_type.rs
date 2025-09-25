/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Traits to mark types as zero-copy or deep-copy.

use crate::{
    prelude::MaxSizeOf,
    ser::SerInner,
    traits::{AlignHash, TypeHash},
};
use sealed::sealed;

/// Internal trait used to select whether a type is zero-copy
/// or deep-copy.
///
/// It has only two implementations, [`Zero`] and [`Deep`].
///
/// In the first case, the type can be serialized from memory and deserialized
/// to memory as a sequence of bytes; in the second case, one has to
/// (de)serialize the type field by field.
#[sealed]
pub trait CopySelector {
    const IS_ZERO_COPY: bool;
}
/// An implementation of a [`CopySelector`] specifying that a type is zero-copy.
pub struct Zero {}

#[sealed]
impl CopySelector for Zero {
    const IS_ZERO_COPY: bool = true;
}

/// An implementation of a [`CopySelector`] specifying that a type is deep-copy.
pub struct Deep {}

#[sealed]
impl CopySelector for Deep {
    const IS_ZERO_COPY: bool = false;
}

/// Marker trait for data specifying whether it is zero-copy or deep-copy.
///
/// The trait comes in two flavors: `CopySelector<Type=Zero>` and
/// `CopySelector<Type=Deep>`. To each of these flavors corresponds two
/// dependent traits, [`ZeroCopy`] (which requires implementing [`Copy`],
/// [`MaxSizeOf`], and be `'static`) and [`DeepCopy`], which are automatically
/// implemented.
///
/// You should not implement this trait manually, but rather use
/// the provided [derive macro](epserde_derive::Epserde).
///
/// We use this trait to implement a different behavior for [`ZeroCopy`] and
/// [`DeepCopy`] types, in particular on arrays, vectors, and boxed slices,
/// [working around the bug that prevents the compiler from understanding that
/// implementations for the two flavors of `CopySelector` are mutually
/// exclusive](https://github.com/rust-lang/rfcs/pull/1672#issuecomment-1405377983).
///
/// For an array of elements of type `T` to be zero-copy serializable and
/// deserializable, `T` must implement `CopySelector<Type=Zero>`. The conditions
/// for this marker trait are that `T` is a [copy type](Copy), that it has a
/// fixed memory layout, and that it does not contain any reference (in
/// particular, that it has `'static` lifetime). If this happen vectors of `T`
/// or boxed slices of `T` can be ε-copy deserialized using a reference to a
/// slice of `T`.
///
/// You can make zero-copy your own types, but you must ensure that they do not
/// contain references and that they have a fixed memory layout; for structures,
/// this requires `repr(C)`. ε-serde will track these conditions at compile time
/// and check them at runtime: in case of failure, serialization will panic.
///
/// Since we cannot use negative trait bounds, every type that is used as a
/// parameter of an array, vector or boxed slice must implement either
/// `CopySelector<Type=Zero>` or `CopySelector<Type=Deep>`. In the latter case,
/// slices will be deserialized element by element, and the result will be a
/// fully deserialized vector or boxed slice. If you do not implement either of
/// these traits, the type will not be serializable inside vectors or boxed
/// slices but error messages will be very unhelpful due to the contrived way we
/// have to implement mutually exclusive types.
///
/// If you use the provided derive macros all this logic will be hidden from
/// you. You'll just have to add `#[zero_copy]` to your structures (if you want
/// them to be zero-copy) and ε-serde will do the rest.
///
/// # Safety
///
/// The trait is unsafe because the user must guarantee that zero-copy types do
/// not contain references, and this cannot be checked by the compiler.
pub unsafe trait CopyType: Sized {
    type Copy: CopySelector;
}

/// Marker trait for zero-copy types. You should never implement
/// this trait directly, but rather implement [`CopyType`] with `Copy=Zero`.
pub trait ZeroCopy:
    CopyType<Copy = Zero> + Copy + TypeHash + AlignHash + MaxSizeOf + SerInner<SerType = Self> + 'static
{
}
impl<
    T: CopyType<Copy = Zero>
        + Copy
        + TypeHash
        + AlignHash
        + MaxSizeOf
        + SerInner<SerType = Self>
        + 'static,
> ZeroCopy for T
{
}

/// Marker trait for deep-copy types. You should never implement
/// this trait directly, but rather implement [`CopyType`] with `Copy=Deep`.
pub trait DeepCopy: CopyType<Copy = Deep> + SerInner<SerType: TypeHash + AlignHash> {}
impl<T: CopyType<Copy = Deep> + SerInner<SerType: TypeHash + AlignHash>> DeepCopy for T {}
