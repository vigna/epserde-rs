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
//! We also provide a [`TypeHash`] implementation for tuples of up to 12
//! elements to help with the idiom `PhantomData<(T1, T2, …)>`.
//!
//! Note that up to ε-serde 0.7.0 we provided an erroneous implementation for
//! mixed zero-copy types. If you serialized a structure using such a tuple,
//! it will be no longer deserializable.

use crate::prelude::*;
use core::hash::Hash;
use deser::*;
use ser::*;

macro_rules! impl_type_hash {
    ($($t:ident),*) => {
		impl<$($t: TypeHash,)*> TypeHash for ($($t,)*)
        {
            #[inline(always)]
            fn type_hash(
                hasher: &mut impl core::hash::Hasher,
            ) {
                "(".hash(hasher);
                $(
                    <$t>::type_hash(hasher);
                )*
                ")".hash(hasher);
            }
        }
    }
}

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<T: ZeroCopy> CopyType for ($($t,)*)  {
            type Copy = Zero;
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

		unsafe impl<T: ZeroCopy + TypeHash + AlignHash> DeserializeInner for ($($t,)*) {
            type DeserType<'a> = &'a ($($t,)*);
            fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
                unsafe { deserialize_full_zero::<($($t,)*)>(backend) }
            }

            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
                ) -> deser::Result<Self::DeserType<'a>> {
                unsafe { deserialize_eps_zero::<($($t,)*)>(backend) }
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

macro_rules! impl_type_hash_muncher {
    ($ty:ident, $($t:ident),*) => {
        impl_type_hash!($ty, $($t),*);
        impl_type_hash_muncher!($($t),*);
    };
    ($ty:ident) => {
        impl_type_hash!($ty);
    };
    () => {};
}

impl_type_hash_muncher!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
