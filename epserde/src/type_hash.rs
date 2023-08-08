/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::hash::Hash;

/// Compute a stable hash for a type. This is used during deserialization to
/// check that the type of the data matches the type of the value being
/// deserialized into.
pub trait TypeHash {
    /// Hash the type, this considers the name, order, and type of the fields
    /// and the type of the struct.  
    fn type_hash(hasher: &mut impl core::hash::Hasher);
    /// Call type_hash on a value
    #[inline(always)]
    fn type_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
        Self::type_hash(hasher)
    }
}

// Blanket impls

impl<T: TypeHash + ?Sized> TypeHash for &'_ T {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        '&'.hash(hasher);
        T::type_hash(hasher);
    }
}

// Core types

impl<T: TypeHash> TypeHash for Option<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Option".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<S: TypeHash, E: TypeHash> TypeHash for Result<S, E> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Result".hash(hasher);
        S::type_hash(hasher);
        E::type_hash(hasher);
    }
}

// Primitive types

impl<T: TypeHash, const N: usize> TypeHash for [T; N] {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[;]".hash(hasher);
        T::type_hash(hasher);
        N.hash(hasher);
    }
}

impl<T: TypeHash> TypeHash for [T] {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "[]".hash(hasher);
        T::type_hash(hasher);
    }
}

macro_rules! impl_primitives {
    ($($ty:ty),*) => {$(
impl TypeHash for $ty {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        stringify!($ty).hash(hasher);
    }
}
    )*};
}

impl_primitives! {
    char, bool, str, f32, f64, (),
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize
}

// Alloc related types

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

#[cfg(feature = "alloc")]
impl TypeHash for String {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "String".hash(hasher);
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
impl<T: TypeHash> TypeHash for Vec<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Vec".hash(hasher);
        T::type_hash(hasher);
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
impl<T: TypeHash + ?Sized> TypeHash for Box<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Box".hash(hasher);
        T::type_hash(hasher);
    }
}

// foreign types

#[cfg(feature = "mmap_rs")]
impl TypeHash for mmap_rs::Mmap {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Mmap".hash(hasher);
    }
}

#[cfg(feature = "mmap_rs")]
impl TypeHash for mmap_rs::MmapMut {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "MmapMut".hash(hasher);
    }
}

// tuples

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: TypeHash,)*> TypeHash for ($($t,)*)
        {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                let mut len = 0;
                $(
                    <$t>::type_hash(hasher);
                    len += 1;
                )*
                len.hash(hasher);
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
