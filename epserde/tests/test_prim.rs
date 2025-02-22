/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};
use epserde::prelude::*;

macro_rules! impl_test {
    ($data:expr, $ty:ty) => {{
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);

        let _ = $data.serialize_with_schema(&mut cursor).unwrap();

        let full_copy = <$ty>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
        assert_eq!($data, full_copy);

        let eps_copy = <$ty>::deserialize_eps(&v).unwrap();
        assert_eq!($data, eps_copy);
    }
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);
        $data.serialize(&mut cursor).unwrap();

        let full_copy = <$ty>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
        assert_eq!($data, full_copy);

        let eps_copy = <$ty>::deserialize_eps(&v).unwrap();
        assert_eq!($data, eps_copy);
    }};
}

macro_rules! test_prim {
    ($ty:ty, $test_name:ident) => {
        #[test]
        fn $test_name() {
            impl_test!(<$ty>::MAX, $ty);
            impl_test!(<$ty>::MIN, $ty);
            impl_test!(0 as $ty, $ty);
            impl_test!(7 as $ty, $ty);
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

macro_rules! test_nonzero {
    ($ty:ty, $test_name:ident) => {
        #[test]
        fn $test_name() {
            impl_test!(<$ty>::MAX, $ty);
            impl_test!(<$ty>::MIN, $ty);
            impl_test!(<$ty>::try_from(1).unwrap(), $ty);
            impl_test!(<$ty>::try_from(7).unwrap(), $ty);
        }
    };
}

test_nonzero!(NonZeroU8, test_nonzero_u8);
test_nonzero!(NonZeroU16, test_nonzero_u16);
test_nonzero!(NonZeroU32, test_nonzero_u32);
test_nonzero!(NonZeroU64, test_nonzero_u64);
test_nonzero!(NonZeroU128, test_nonzero_u128);
test_nonzero!(NonZeroUsize, test_nonzero_usize);
test_nonzero!(NonZeroI8, testnonzero_i8);
test_nonzero!(NonZeroI16, test_nonzero_i16);
test_nonzero!(NonZeroI32, test_nonzero_i32);
test_nonzero!(NonZeroI64, test_nonzero_i64);
test_nonzero!(NonZeroI128, test_nonzero_i128);
test_nonzero!(NonZeroIsize, test_nonzero_isize);

#[test]
fn test_unit() {
    impl_test!((), ());
}

#[test]
fn test_bool() {
    impl_test!(true, bool);
    impl_test!(false, bool);
}

const TEST_STRS: &[&str] = &["abc\0\x0a🔥\u{0d2bdf}", ""];

#[test]
fn test_char() {
    for test_str in TEST_STRS {
        for c in test_str.chars() {
            impl_test!(c, char);
        }
    }
}

#[test]
fn test_string() {
    for test_str in TEST_STRS {
        let s = test_str.to_string();
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);

            let mut schema = s.serialize_with_schema(&mut cursor).unwrap();
            schema.0.sort_by_key(|a| a.offset);

            cursor.set_position(0);
            let full_copy = <String>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <String>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_str(), full_copy);

            let _ = schema.to_csv();
            let _ = schema.debug(&v);
        }
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);
            s.serialize(&mut cursor).unwrap();

            cursor.set_position(0);
            let full_copy = <String>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <String>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_str(), full_copy);
        }
    }
}

#[test]
fn test_box_str() {
    for test_str in TEST_STRS {
        let s = test_str.to_string().into_boxed_str();
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);

            let mut schema = s.serialize_with_schema(&mut cursor).unwrap();
            schema.0.sort_by_key(|a| a.offset);

            cursor.set_position(0);
            let full_copy = <Box<str>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <Box<str>>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_ref(), full_copy);
        }
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);
            s.serialize(&mut cursor).unwrap();

            cursor.set_position(0);
            let full_copy = <Box<str>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <Box<str>>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_ref(), full_copy);
        }
    }
}
