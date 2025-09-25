/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;

const TEST_STRS: &[&str] = &["abc\0\x0a🔥\u{0d2bdf}", ""];

#[test]
fn test_box_str() {
    for &test_str in TEST_STRS {
        let s = test_str;
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);

            let mut schema = unsafe { s.serialize_with_schema(&mut cursor).unwrap() };
            schema.0.sort_by_key(|a| a.offset);

            cursor.set_position(0);
            let full_copy =
                unsafe { <Box<str>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
            assert_eq!(s, &*full_copy);

            let eps_copy = unsafe { <Box<str>>::deserialize_eps(&v).unwrap() };
            assert_eq!(s, eps_copy);
            let eps_copy = unsafe { String::deserialize_eps(&v).unwrap() };
            assert_eq!(s, eps_copy);
        }
        let s = test_str.to_string();
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);

            let mut schema = unsafe { s.serialize_with_schema(&mut cursor).unwrap() };
            schema.0.sort_by_key(|a| a.offset);

            cursor.set_position(0);
            let full_copy =
                unsafe { String::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
            assert_eq!(s, full_copy);

            let eps_copy = unsafe { <Box<str>>::deserialize_eps(&v).unwrap() };
            assert_eq!(s, eps_copy);
            let eps_copy = unsafe { String::deserialize_eps(&v).unwrap() };
            assert_eq!(s, eps_copy);
        }
        let s = test_str.to_string().into_boxed_str();
        {
            let mut v = vec![];
            let mut cursor = std::io::Cursor::new(&mut v);
            unsafe { s.serialize(&mut cursor).unwrap() };

            cursor.set_position(0);
            let full_copy =
                unsafe { <Box<str>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
            assert_eq!(s, full_copy);

            let eps_copy = unsafe { <Box<str>>::deserialize_eps(&v).unwrap() };
            assert_eq!(s.as_ref(), eps_copy);
            let eps_copy = unsafe { String::deserialize_eps(&v).unwrap() };
            assert_eq!(s.as_ref(), eps_copy);
        }
    }
}

fn test_generic<T>(s: T)
where
    T: Serialize + Deserialize + PartialEq + core::fmt::Debug,
    for<'a> DeserType<'a, T>: PartialEq<T> + core::fmt::Debug,
{
    {
        let mut cursor = <AlignedCursor<A16>>::new();

        let mut schema = unsafe { s.serialize_with_schema(&mut cursor).unwrap() };
        schema.0.sort_by_key(|a| a.offset);

        cursor.set_position(0);
        let full_copy = unsafe {
            <T>::deserialize_full(&mut std::io::Cursor::new(&cursor.as_bytes())).unwrap()
        };
        assert_eq!(s, full_copy);

        let bytes = cursor.as_bytes();
        let full_copy = unsafe { <T>::deserialize_eps(bytes).unwrap() };
        assert_eq!(full_copy, s);

        let _ = schema.to_csv();
        let _ = schema.debug(bytes);
    }
    {
        let mut cursor = <AlignedCursor<A16>>::new();
        unsafe { s.serialize(&mut cursor).unwrap() };

        cursor.set_position(0);
        let full_copy =
            unsafe { <T>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())).unwrap() };
        assert_eq!(s, full_copy);

        let full_copy = unsafe { <T>::deserialize_eps(cursor.as_bytes()).unwrap() };
        assert_eq!(full_copy, s);
    }
}

#[test]
fn test_range() {
    test_generic::<std::ops::Range<i32>>(0..10);

    #[derive(Epserde, PartialEq, Debug)]
    struct Data(std::ops::Range<i32>);
    test_generic(Data(0..10));
}

#[test]
fn test_ser_rc_ref() {
    use std::rc::Rc;
    let v = vec![0, 1, 2, 3];
    let mut cursor = <AlignedCursor<A16>>::new();
    unsafe { Serialize::serialize(&Rc::new(v.as_slice()), &mut cursor).unwrap() };
    cursor.set_position(0);
    let s = unsafe { <Rc<Box<[i32]>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    dbg!(s);
}

#[test]
fn test_ref_field() {
    use std::rc::Rc;
    let v = vec![0, 1, 2, 3];
    let mut cursor = <AlignedCursor<A16>>::new();
    #[derive(Epserde, Debug)]
    struct Data<A>(A);
    unsafe { Serialize::serialize(&Rc::new(Data(v.as_slice())), &mut cursor).unwrap() };
    cursor.set_position(0);
    let s = unsafe { <Rc<Data<Box<[i32]>>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    dbg!(s);
}
