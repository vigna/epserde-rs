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
        unsafe impl<T: ZeroCopy> CopyType for ($($t,)*)  {
            type Copy = Zero;
		}

		impl<T: AlignHash> AlignHash for ($($t,)*)
        {
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
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                $(if max_size_of < core::cmp::max(max_size_of, <$t>::max_size_of()) {
                    max_size_of = <$t>::max_size_of();
                })*
                max_size_of
            }
        }

		impl<T: ZeroCopy + TypeHash + AlignHash> SerInner for ($($t,)*) {
            type SerType = Self;
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                ser_zero(backend, self)
            }
        }

		impl<T: ZeroCopy + TypeHash + AlignHash> DeserInner for ($($t,)*) {
            type DeserType<'a> = &'a ($($t,)*);
            unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
                unsafe { deser_full_zero::<($($t,)*)>(backend) }
            }

            unsafe fn _deser_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
                ) -> deser::Result<Self::DeserType<'a>> {
                unsafe { deser_eps_zero::<($($t,)*)>(backend) }
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
