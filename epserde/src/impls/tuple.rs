/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for tuples.

For the time being, we only support tuples of up to 10 elements all of which
are [`ZeroCopy`]. For tuples of more than 10 elements, or tuples with elements
that are not [`ZeroCopy`], you must use [`epserde_derive::Epserde`] on a newtype.

*/
use crate::deser::DeserializeInner;
use crate::prelude::*;
use crate::traits::TypeHash;
use core::hash::Hash;
use deser::*;
use ser::*;

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: ZeroCopy,)*> CopyType for ($($t,)*)  {
            type Copy = Zero;
		}

		impl<$($t: TypeHash,)*> TypeHash for ($($t,)*)
        {
            #[inline(always)]
            fn type_hash(
                hasher: &mut impl core::hash::Hasher,
            ) {
                "()".hash(hasher);
                core::mem::align_of::<Self>().hash(hasher);
                $(
                    <$t>::type_hash(hasher);
                )*
            }
        }

		impl<$($t: ReprHash,)*> ReprHash for ($($t,)*)
        {
            #[inline(always)]
            fn repr_hash(
                hasher: &mut impl core::hash::Hasher,
                offset_of: &mut usize,
            ) {
                $(
                    <$t>::repr_hash(hasher, offset_of);
                )*
            }
        }

        impl<$($t: MaxSizeOf,)*> MaxSizeOf for ($($t,)*)
        {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                $(if max_size_of < std::cmp::max(max_size_of, <$t>::max_size_of()) {
                    max_size_of = <$t>::max_size_of();
                })*
                max_size_of
            }
        }

		impl<$($t: ZeroCopy + TypeHash + ReprHash,)*> SerializeInner for ($($t,)*) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> ser::Result<()> {
                backend.write_field_zero("tuple", self)
            }
        }

		impl<$($t: ZeroCopy + TypeHash + ReprHash + 'static,)*> DeserializeInner for ($($t,)*) {
            type DeserType<'a> = &'a ($($t,)*);
            fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
                deserialize_full_zero::<($($t,)*)>(backend)
            }

            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
                ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<($($t,)*)>(backend)
            }
        }
    };
}

macro_rules! impl_tuples_muncher {
    ($ty:ident, $($t:ident),*) => {
        impl_tuples!($ty, $($t),*);
        impl_tuples_muncher!($($t),*);
    };
    ($ty:ident) => {
        impl_tuples!($ty);
    };
    () => {};
}

impl_tuples_muncher!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
