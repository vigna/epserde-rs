/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use core::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use epserde::prelude::*;

macro_rules! impl_test {
    ($data:expr, $ty:ty) => {{
        let mut cursor = <AlignedCursor>::new();

        let _ = unsafe { $data.serialize_with_schema(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(full_copy, $data);

        let eps_copy = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(eps_copy, $data);
    }
    {
        let mut cursor = <AlignedCursor>::new();
        unsafe { $data.serialize(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(full_copy, $data);

        let eps_copy = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(eps_copy, $data);
    }};
}

macro_rules! test_prim {
    ($ty:ty, $test_name:ident) => {
        #[test]
        fn $test_name() -> anyhow::Result<()> {
            impl_test!(<$ty>::MAX, $ty);
            impl_test!(<$ty>::MIN, $ty);
            impl_test!(0 as $ty, $ty);
            impl_test!(7 as $ty, $ty);
            Ok(())
        }
    };
}

test_prim!(u8, test_u8);
test_prim!(u16, test_u16);
test_prim!(u32, test_u32);
test_prim!(u64, test_u64);
test_prim!(u128, test_u128);
test_prim!(usize, test_usize);
test_prim!(i8, test_i8);
test_prim!(i16, test_i16);
test_prim!(i32, test_i32);
test_prim!(i64, test_i64);
test_prim!(i128, test_i128);
test_prim!(isize, test_isize);
test_prim!(f32, test_f32);
test_prim!(f64, test_f64);

macro_rules! test_nonzero {
    ($ty:ty, $test_name:ident) => {
        #[test]
        fn $test_name() -> anyhow::Result<()> {
            impl_test!(<$ty>::MAX, $ty);
            impl_test!(<$ty>::MIN, $ty);
            impl_test!(<$ty>::try_from(1)?, $ty);
            impl_test!(<$ty>::try_from(7)?, $ty);
            Ok(())
        }
    };
}

test_nonzero!(NonZeroU8, test_nonzero_u8);
test_nonzero!(NonZeroU16, test_nonzero_u16);
test_nonzero!(NonZeroU32, test_nonzero_u32);
test_nonzero!(NonZeroU64, test_nonzero_u64);
test_nonzero!(NonZeroU128, test_nonzero_u128);
test_nonzero!(NonZeroUsize, test_nonzero_usize);
test_nonzero!(NonZeroI8, test_nonzero_i8);
test_nonzero!(NonZeroI16, test_nonzero_i16);
test_nonzero!(NonZeroI32, test_nonzero_i32);
test_nonzero!(NonZeroI64, test_nonzero_i64);
test_nonzero!(NonZeroI128, test_nonzero_i128);
test_nonzero!(NonZeroIsize, test_nonzero_isize);

#[test]
fn test_unit() -> anyhow::Result<()> {
    impl_test!((), ());
    Ok(())
}

#[test]
fn test_bool() -> anyhow::Result<()> {
    impl_test!(true, bool);
    impl_test!(false, bool);
    Ok(())
}

const TEST_STRS: &[&str] = &["abc\0\x0a🔥\u{0d2bdf}", ""];

#[test]
fn test_char() -> anyhow::Result<()> {
    for test_str in TEST_STRS {
        for c in test_str.chars() {
            impl_test!(c, char);
        }
    }
    Ok(())
}
