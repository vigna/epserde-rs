/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use epserde::prelude::*;

macro_rules! impl_test {
    ($data:expr, $ty:ty) => {{
        let mut v = vec![];
        let mut buf = std::io::Cursor::new(&mut v);

        let _ = $data.serialize_with_schema(&mut buf).unwrap();

        let full_copy = <$ty>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
        assert_eq!($data, full_copy);

        let full_copy = <$ty>::deserialize_eps(&v).unwrap();
        assert_eq!($data, full_copy);
    }
    {
        let mut v = vec![];
        let mut buf = std::io::Cursor::new(&mut v);
        $data.serialize(&mut buf).unwrap();

        let full_copy = <$ty>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
        assert_eq!($data, full_copy);

        let full_copy = <$ty>::deserialize_eps(&v).unwrap();
        assert_eq!($data, full_copy);
    }};
}

macro_rules! test_primitive {
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

test_primitive!(u8, test_u8);
test_primitive!(u16, test_u16);
test_primitive!(u32, test_u32);
test_primitive!(u64, test_u64);
test_primitive!(u128, test_u128);
test_primitive!(usize, test_usize);
test_primitive!(i8, test_i8);
test_primitive!(i16, test_i16);
test_primitive!(i32, test_i32);
test_primitive!(i64, test_i64);
test_primitive!(i128, test_i128);
test_primitive!(isize, test_isize);

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
            let mut buf = std::io::Cursor::new(&mut v);

            let mut schema = s.serialize_with_schema(&mut buf).unwrap();
            schema.0.sort_by_key(|a| a.offset);

            buf.set_position(0);
            let full_copy = <String>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <String>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_str(), full_copy);

            let _ = schema.to_csv();
            let _ = schema.debug(&v);
        }
        {
            let mut v = vec![];
            let mut buf = std::io::Cursor::new(&mut v);
            s.serialize(&mut buf).unwrap();

            buf.set_position(0);
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
            let mut buf = std::io::Cursor::new(&mut v);

            let mut schema = s.serialize_with_schema(&mut buf).unwrap();
            schema.0.sort_by_key(|a| a.offset);

            buf.set_position(0);
            let full_copy = <Box<str>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <Box<str>>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_ref(), full_copy);
        }
        {
            let mut v = vec![];
            let mut buf = std::io::Cursor::new(&mut v);
            s.serialize(&mut buf).unwrap();

            buf.set_position(0);
            let full_copy = <Box<str>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
            assert_eq!(s, full_copy);

            let full_copy = <Box<str>>::deserialize_eps(&v).unwrap();
            assert_eq!(s.as_ref(), full_copy);
        }
    }
}
