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
use crate::des::DeserializeInner;
use crate::prelude::*;
use crate::traits::TypeHash;
use core::hash::Hash;
use des::*;
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
                type_hasher: &mut impl core::hash::Hasher,
                repr_hasher: &mut impl core::hash::Hasher,
                offset_of: &mut usize,
            ) {
                "()".hash(type_hasher);
                core::mem::align_of::<Self>().hash(type_hasher);
                $(
                    <$t>::type_hash(type_hasher, repr_hasher, offset_of);
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

		impl<$($t: ZeroCopy + TypeHash,)*> SerializeInner for ($($t,)*) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> ser::Result<F> {
                backend.write_field_zero("tuple", self)
            }
        }

		impl<$($t: ZeroCopy + TypeHash + 'static,)*> DeserializeInner for ($($t,)*) {
            type DeserType<'a> = &'a ($($t,)*);
            fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> des::Result<(Self, R)> {
                backend.deserialize_full_zero::<($($t,)*)>()
            }

            fn _deserialize_eps_copy_inner(
                backend: SliceWithPos,
                ) -> des::Result<(Self::DeserType<'_>, SliceWithPos)> {
                backend.deserialize_eps_zero::<($($t,)*)>()
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
