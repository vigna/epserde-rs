/*
 * SPDX-FileCopyrightText: 2025 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Blanket implementations for references and single-item containers

*/

use crate::prelude::*;
use ser::*;

macro_rules! impl_ser {
    ($type:ty) => {
        impl<T: CopyType> CopyType for $type {
            type Copy = <T as CopyType>::Copy;
        }

        impl<T: TypeHash> TypeHash for $type {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                <T as TypeHash>::type_hash(hasher)
            }
        }

        impl<T: AlignHash> AlignHash for $type {
            #[inline(always)]
            fn align_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                <T as AlignHash>::align_hash(hasher, offset_of)
            }
        }

        impl<T: SerializeInner> SerializeInner for $type {
            type SerType = T;
            const IS_ZERO_COPY: bool = <T as SerializeInner>::IS_ZERO_COPY;
            const ZERO_COPY_MISMATCH: bool = <T as SerializeInner>::ZERO_COPY_MISMATCH;

            #[inline(always)]
            unsafe fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                <T as SerializeInner>::_serialize_inner(self, backend)
            }
        }
    };
}

macro_rules! impl_all {
    ($type:ident) => {
        impl_ser!($type<T>);

        impl<T: DeserializeInner> DeserializeInner for $type<T> {
            type DeserType<'a> = $type<<T as DeserializeInner>::DeserType<'a>>;

            #[inline(always)]
            unsafe fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                <T as DeserializeInner>::_deserialize_full_inner(backend).map($type::new)
            }
            #[inline(always)]
            unsafe fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                <T as DeserializeInner>::_deserialize_eps_inner(backend).map($type::new)
            }
        }
    };
}

impl_ser!(&T);
impl_ser!(&mut T);

#[cfg(any(feature = "std", feature = "alloc"))]
mod std_impl {
    use super::*;

    #[cfg(not(feature = "std"))]
    mod imports {
        pub use alloc::boxed::Box;
        pub use alloc::rc::Rc;
        pub use alloc::sync::Arc;
    }
    #[cfg(feature = "std")]
    mod imports {
        pub use std::rc::Rc;
        pub use std::sync::Arc;
    }
    use imports::*;

    impl_all!(Box);
    impl_all!(Arc);
    impl_all!(Rc);
}
