/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use std::{io::Cursor, ops::ControlFlow};

use epserde::prelude::*;

fn test_generic<T>(s: T)
where
    T: Serialize + Deserialize + PartialEq + core::fmt::Debug,
    for<'a> <T as DeserializeInner>::DeserType<'a>: PartialEq<T> + core::fmt::Debug,
{
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);

        let mut schema = unsafe { s.serialize_with_schema(&mut cursor).unwrap() };
        schema.0.sort_by_key(|a| a.offset);

        cursor.set_position(0);
        let full_copy = unsafe { <T>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
        assert_eq!(s, full_copy);

        let full_copy = unsafe { <T>::deserialize_eps(&v).unwrap() };
        assert_eq!(full_copy, s);

        let _ = schema.to_csv();
        let _ = schema.debug(&v);
    }
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);
        unsafe { s.serialize(&mut cursor).unwrap() };

        cursor.set_position(0);
        let full_copy = unsafe { <T>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
        assert_eq!(s, full_copy);

        let full_copy = unsafe { <T>::deserialize_eps(&v).unwrap() };
        assert_eq!(full_copy, s);
    }
}
/*
#[test]
fn test_range() {
    test_generic::<std::ops::Range<i32>>(0..10);

    #[derive(Epserde, PartialEq, Debug)]
    struct Data(std::ops::Range<i32>);
    test_generic(Data(0..10));
}
*/
#[test]
fn test_range_covariant_downcast() {
    let mut buffer = Vec::new();
    let range = std::ops::Range { start: 0, end: 10 };
    unsafe { range.serialize(&mut buffer).unwrap() };

    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { <std::ops::Range<i32>>::read_mem(cursor, buffer.len()).unwrap() };
    assert_eq!(range, *mem_case.get());

    let mut buffer = Vec::new();
    let range = std::ops::RangeFrom { start: 0 };
    unsafe { range.serialize(&mut buffer).unwrap() };

    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { <std::ops::RangeFrom<i32>>::read_mem(cursor, buffer.len()).unwrap() };
    assert_eq!(range, *mem_case.get());

    let mut buffer = Vec::new();
    let range = std::ops::RangeTo { end: 0 };
    unsafe { range.serialize(&mut buffer).unwrap() };

    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { <std::ops::RangeTo<i32>>::read_mem(cursor, buffer.len()).unwrap() };
    assert_eq!(range, *mem_case.get());
    let mut buffer = Vec::new();
    let range = std::ops::RangeToInclusive { end: 0 };
    unsafe { range.serialize(&mut buffer).unwrap() };

    let cursor = Cursor::new(&buffer);
    let mem_case =
        unsafe { <std::ops::RangeToInclusive<i32>>::read_mem(cursor, buffer.len()).unwrap() };
    assert_eq!(range, *mem_case.get());

    let mut buffer = Vec::new();
    let range = std::ops::RangeFull {};
    unsafe { range.serialize(&mut buffer).unwrap() };

    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { std::ops::RangeFull::read_mem(cursor, buffer.len()).unwrap() };
    assert_eq!(range, *mem_case.get());
}

#[test]
fn test_control_flow_covariant_downcast() {
    let mut buffer = Vec::new();
    let control_flow = ControlFlow::<(), ()>::Continue(());
    unsafe { control_flow.serialize(&mut buffer).unwrap() };

    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { <ControlFlow<(), ()>>::read_mem(cursor, buffer.len()).unwrap() };
    assert_eq!(control_flow, *mem_case.get());
}
