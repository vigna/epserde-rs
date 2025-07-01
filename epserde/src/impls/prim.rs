/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Implementations for primitive types, `()`, [`PhantomData`] and [`Option`].

*/

use crate::prelude::*;
use common_traits::NonZero;
use core::hash::Hash;
use core::marker::PhantomData;
use core::mem::size_of;
use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};
use deser::*;
use ser::*;

macro_rules! impl_prim_type_hash {
    ($($ty:ty),*) => {$(
        impl CopyType for $ty {
            type Copy = Zero;
        }

        impl TypeHash for $ty {
            fn type_hash(
                hasher: &mut impl core::hash::Hasher,
            ) {
                stringify!($ty).hash(hasher);
            }
        }

        impl AlignHash for $ty {
            fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_align_hash::<Self>(hasher, offset_of)
            }
        }

        impl MaxSizeOf for $ty {
            fn max_size_of() -> usize {
                size_of::<$ty>()
            }
        }
    )*};
}

macro_rules! impl_prim_ser_des {
    ($($ty:ty),*) => {$(
		impl SerializeInner for $ty {
            type SerType = Self;
            // Note that primitive types are declared zero-copy to be able to
            // be part of zero-copy types, but we actually deserialize
            // them in isolation as values.
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }

		unsafe impl DeserializeInner for $ty {
            #[inline(always)]
            fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<$ty> {
                let mut buf = [0; size_of::<$ty>()];
                backend.read_exact(&mut buf)?;
                Ok(<$ty>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <$ty>::from_ne_bytes(
                        backend.data.get(..size_of::<$ty>()).ok_or(deser::Error::ReadError)?
                            .try_into().unwrap(),
                    );

                backend.skip(size_of::<$ty>());
                Ok(res)
            }
        }
    )*};
}

impl_prim_type_hash!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);
impl_prim_ser_des!(isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128, f32, f64);

macro_rules! impl_nonzero_ser_des {
    ($($ty:ty),*) => {$(
		impl SerializeInner for $ty {
            type SerType = Self;                // Note that primitive types are declared zero-copy to be able to
            // be part of zero-copy types, but we actually deserialize
            // them in isolation as values.
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;

            #[inline(always)]
            fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }

		unsafe impl DeserializeInner for $ty {
            #[inline(always)]
            fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<$ty> {
                let mut buf = [0; size_of::<$ty>()];
                backend.read_exact(&mut buf)?;
                Ok(<$ty as NonZero>::BaseType::from_ne_bytes(buf).try_into().unwrap())
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <$ty as NonZero>::BaseType::from_ne_bytes(
                        backend.data.get(..size_of::<$ty>()).ok_or(deser::Error::ReadError)?
                            .try_into()
                            .unwrap()).try_into().unwrap();

                backend.skip(size_of::<$ty>());
                Ok(res)
            }
        }
    )*};
}

impl_prim_type_hash!(
    NonZeroIsize,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroUsize,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128
);

impl_nonzero_ser_des!(
    NonZeroIsize,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroUsize,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128
);

impl_prim_type_hash!(bool, char, ());

// Booleans are zero-copy serialized as u8.

impl SerializeInner for bool {
    type SerType = Self;
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        let val = if *self { 1 } else { 0 };
        backend.write_all(&[val])
    }
}

unsafe impl DeserializeInner for bool {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<bool> {
        Ok(u8::_deserialize_full_inner(backend)? != 0)
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let res = backend.data[0] != 0;
        backend.skip(1);
        Ok(res)
    }
}

// Chars are zero-copy serialized as u32.

impl SerializeInner for char {
    type SerType = Self;
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        (*self as u32)._serialize_inner(backend)
    }
}

unsafe impl DeserializeInner for char {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(char::from_u32(u32::_deserialize_full_inner(backend)?).unwrap())
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(char::from_u32(u32::_deserialize_eps_inner(backend)?).unwrap())
    }
}

// () is zero-copy. No reading or writing is performed when (de)serializing it.

impl SerializeInner for () {
    type SerType = ();
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

unsafe impl DeserializeInner for () {
    #[inline(always)]
    fn _deserialize_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(())
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(())
    }
}

// PhantomData is zero-copy. No reading or writing is performed when
// (de)serializing it. The type parameter does not have to be sized,
// but it does have to implement TypeHash, as we must be able to tell
// apart structures with different type parameters stored in a PhantomData.

impl<T: ?Sized> CopyType for PhantomData<T> {
    type Copy = Zero;
}

impl<T: ?Sized> MaxSizeOf for PhantomData<T> {
    fn max_size_of() -> usize {
        0
    }
}

impl<T: ?Sized + TypeHash> TypeHash for PhantomData<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "PhantomData".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: ?Sized> AlignHash for PhantomData<T> {
    #[inline(always)]
    fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
}

impl<T: ?Sized> SerializeInner for PhantomData<T> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = true;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, _backend: &mut impl WriteWithNames) -> ser::Result<()> {
        Ok(())
    }
}

unsafe impl<T: ?Sized> DeserializeInner for PhantomData<T> {
    #[inline(always)]
    fn _deserialize_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        Ok(PhantomData::<T>)
    }
    type DeserType<'a> = Self;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        _backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        Ok(PhantomData)
    }
}

// Options are deep-copy types serialized as a one-byte tag (0 for None, 1 for Some) followed, in case, by the value.

impl<T> CopyType for Option<T> {
    type Copy = Deep;
}

impl<T: TypeHash> TypeHash for Option<T> {
    #[inline(always)]
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "Option".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<T: AlignHash> AlignHash for Option<T> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        T::align_hash(hasher, &mut 0);
    }
}

impl<T: SerializeInner + TypeHash + AlignHash> SerializeInner for Option<T> {
    type SerType = Self;
    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    #[inline(always)]
    fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        match self {
            None => backend.write("Tag", &0_u8),
            Some(val) => {
                backend.write("Tag", &1_u8)?;
                backend.write("Some", val)
            }
        }
    }
}

unsafe impl<T: DeserializeInner> DeserializeInner for Option<T> {
    #[inline(always)]
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        let tag = u8::_deserialize_full_inner(backend)?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(T::_deserialize_full_inner(backend)?)),
            _ => Err(deser::Error::InvalidTag(tag as usize)),
        }
    }
    type DeserType<'a> = Option<<T as DeserializeInner>::DeserType<'a>>;
    #[inline(always)]
    fn _deserialize_eps_inner<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> deser::Result<Self::DeserType<'a>> {
        let tag = u8::_deserialize_full_inner(backend)?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(T::_deserialize_eps_inner(backend)?)),
            _ => Err(deser::Error::InvalidTag(backend.data[0] as usize)),
        }
    }
}
