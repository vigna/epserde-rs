/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementations for tuples.
//!
//! We only support tuples of up to 12 elements of the same [`ZeroCopy`] type.
//! There is no `repr(C)` for tuples, so we [cannot guarantee that the storage
//! order of the fields is well-defined], albeit we assume that it is
//! for homogeneous tuples.
//!
//! To circumvent this problem (or to be 100% sure that the order is
//! well-defined even in the homogeneous case) you can define a tuple newtype
//! with a `repr(C)` attribute.
//!
//! We also provide a [`TypeHash`] implementation for tuples of up to 12
//! elements to help with the idiom `PhantomData<(T1, T2, …)>`.
//!
//! [cannot guarantee that the storage order of the fields is well-defined]: https://doc.rust-lang.org/reference/type-layout.html#the-rust-representation

use crate::check_covariance;
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

        impl<T: PadTo> PadTo for ($($t,)*)
        {
            fn pad_to() -> usize {
                let mut pad_to = 0;
                $(if pad_to < <$t>::pad_to() {
                    pad_to = <$t>::pad_to();
                })*
                pad_to
            }
        }

        impl<T: ZeroCopy> SerInner for ($($t,)*) {
            type SerType = Self;
            // Forwarded, not hardcoded, so that a hand-written incoherent
            // element impl still trips the check_zero_copy runtime net.
            const IS_ZERO_COPY: bool = T::IS_ZERO_COPY;

            #[inline(always)]
            unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                const {
                    assert!(
                        ::core::mem::size_of::<($($t,)*)>()
                            == (0_usize $( + { let _ = ::core::stringify!($t); 1_usize } )*)
                                * ::core::mem::size_of::<T>(),
                        "epserde: homogeneous tuple layout assumption violated by this compiler"
                    );
                }
                unsafe { ser_zero(backend, self) }
            }
        }

        impl<T: ZeroCopy> DeserInner for ($($t,)*) {
            check_covariance!();
            type DeserType<'a> = &'a ($($t,)*);
            unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
                const {
                    assert!(
                        ::core::mem::size_of::<($($t,)*)>()
                            == (0_usize $( + { let _ = ::core::stringify!($t); 1_usize } )*)
                                * ::core::mem::size_of::<T>(),
                        "epserde: homogeneous tuple layout assumption violated by this compiler"
                    );
                }
                unsafe { deser_full_zero::<($($t,)*)>(backend) }
            }

            unsafe fn _deser_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
                ) -> deser::Result<Self::DeserType<'a>> {
                const {
                    assert!(
                        ::core::mem::size_of::<($($t,)*)>()
                            == (0_usize $( + { let _ = ::core::stringify!($t); 1_usize } )*)
                                * ::core::mem::size_of::<T>(),
                        "epserde: homogeneous tuple layout assumption violated by this compiler"
                    );
                }
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
