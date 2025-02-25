/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for tuples.
//!
//! We only support tuples of up to 12 elements of the same [`ZeroCopy`] type.
//! The is no `repr(C)` for tuples, so we [cannot guarantee that the storage
//! order of the fields is
//! well-defined](https://doc.rust-lang.org/reference/type-layout.html#the-rust-representation).
//!
//! To circumvent this problem, you can define a tuple newtype with a `repr(C)`
//! attribute.
//! 
//! Note that because of this limitation the standard technique of aggregating
//! types in tuples to reduce the number of
//! [`PhantomData`](core::marker::PhantomData) marker should be avoided, unless
//! they are all [`ZeroCopy`].

use crate::prelude::*;
use core::hash::Hash;
use deser::*;
use ser::*;

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<T: ZeroCopy> CopyType for ($($t,)*)  {
            type Copy = Zero;
		}

		impl<T: TypeHash> TypeHash for ($($t,)*)
        {
            #[inline(always)]
            fn type_hash(
                hasher: &mut impl core::hash::Hasher,
            ) {
                "()".hash(hasher);
                $(
                    <$t>::type_hash(hasher);
                )*
            }
        }

		impl<T: AlignHash> AlignHash for ($($t,)*)
        {
            #[inline(always)]
            fn align_hash(
                hasher: &mut impl core::hash::Hasher,
                offset_of: &mut usize,
            ) {
                $(
                    <$t>::align_hash(hasher, offset_of);
                )*
            }
        }

        impl<T: MaxSizeOf> MaxSizeOf for ($($t,)*)
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

		impl<T: ZeroCopy + TypeHash + AlignHash> SerializeInner for ($($t,)*) {
            type SerType = Self;
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }

		impl<T: ZeroCopy + TypeHash + AlignHash> DeserializeInner for ($($t,)*) {
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

impl_tuples_muncher!(T, T, T, T, T, T, T, T, T, T, T, T);
