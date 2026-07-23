/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Tuple-specific tests. Round trips for small homogeneous tuples of
//! zero-copy types are in test_zero.rs.

use epserde::prelude::*;

macro_rules! impl_test {
    ($ty:ty, $data:expr) => {{
        let mut cursor = <AlignedCursor<Aligned16>>::new();

        let _ = unsafe { $data.serialize_with_schema(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(full_copy, $data);

        let eps_copy = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(*eps_copy, $data);
    }
    {
        let mut cursor = <AlignedCursor<Aligned16>>::new();
        unsafe { $data.serialize(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(full_copy, $data);

        let eps_copy = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(*eps_copy, $data);
    }};
}

macro_rules! test_tuple {
    ($test_name:ident, $ty:ty, $data: expr) => {
        #[test]
        fn $test_name() -> anyhow::Result<()> {
            impl_test!($ty, $data);
            Ok(())
        }
    };
}

// Tuples are supported up to arity 12
test_tuple!(
    test_tuple_arity_12,
    (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32),
    (1_i32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)
);

/// IS_ZERO_COPY must be forwarded from the element type, not hardcoded: a
/// hand-written element whose constant is false must propagate to the tuple,
/// so that the check_zero_copy runtime net still trips.
#[test]
fn test_tuple_is_zero_copy_forwarded() {
    use core::hash::Hash;
    use epserde::deser::{DeserInner, ReadWithPos, SliceWithPos};
    use epserde::ser::{SerInner, WriteWithNames};
    use epserde::traits::{AlignHash, CopyType, PadTo, TypeHash, Zero};

    #[derive(Copy, Clone)]
    struct Fake(#[allow(dead_code)] u8);

    unsafe impl CopyType for Fake {
        type Copy = Zero;
    }
    impl TypeHash for Fake {
        fn type_hash(hasher: &mut impl core::hash::Hasher) {
            "Fake".hash(hasher);
        }
    }
    impl AlignHash for Fake {
        fn align_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
    }
    impl PadTo for Fake {
        fn pad_to() -> usize {
            1
        }
    }
    impl SerInner for Fake {
        type SerType = Self;
        const IS_ZERO_COPY: bool = false;
        unsafe fn _ser_inner(
            &self,
            _backend: &mut impl WriteWithNames,
        ) -> epserde::ser::Result<()> {
            Ok(())
        }
    }
    impl DeserInner for Fake {
        epserde::check_covariance!();
        type DeserType<'a> = Self;
        unsafe fn _deser_full_inner(
            _backend: &mut impl ReadWithPos,
        ) -> epserde::deser::Result<Self> {
            Ok(Fake(0))
        }
        unsafe fn _deser_eps_inner<'a>(
            _backend: &mut SliceWithPos<'a>,
        ) -> epserde::deser::Result<Self::DeserType<'a>> {
            Ok(Fake(0))
        }
    }

    const { assert!(!<(Fake, Fake) as SerInner>::IS_ZERO_COPY) };
    const { assert!(<(u32, u32) as SerInner>::IS_ZERO_COPY) };
}
