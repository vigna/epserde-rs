/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

fn test_generic<T>(s: T)
where
    T: Serialize + Deserialize + PartialEq + core::fmt::Debug,
    for<'a> <T as DeserializeInner>::DeserType<'a>: PartialEq<T> + core::fmt::Debug,
{
    test_generic_split::<T, T, T>(s, |value| value)
}
fn test_generic_split<Ser, Deser, OwnedSer>(s: Ser, deref: impl Fn(&Ser) -> &OwnedSer)
where
    Ser: Serialize,
    Deser: Deserialize + PartialEq<OwnedSer> + core::fmt::Debug,
    OwnedSer: core::fmt::Debug,
    for<'a> <Deser as DeserializeInner>::DeserType<'a>: PartialEq<OwnedSer> + core::fmt::Debug,
{
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);

        let mut schema = unsafe { s.serialize_with_schema(&mut cursor).unwrap() };
        schema.0.sort_by_key(|a| a.offset);

        cursor.set_position(0);
        let full_copy =
            unsafe { <Deser>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
        assert_eq!(&full_copy, deref(&s));

        let full_copy = unsafe { <Deser>::deserialize_eps(&v).unwrap() };
        assert_eq!(&full_copy, deref(&s));

        let _ = schema.to_csv();
        let _ = schema.debug(&v);
    }
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);
        unsafe { s.serialize(&mut cursor).unwrap() };

        cursor.set_position(0);
        let full_copy =
            unsafe { <Deser>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap() };
        assert_eq!(&full_copy, deref(&s));

        let full_copy = unsafe { <Deser>::deserialize_eps(&v).unwrap() };
        assert_eq!(&full_copy, deref(&s));
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
fn test_containers() {
    test_generic::<Box<i32>>(Box::new(10));
    test_generic::<Arc<i32>>(Arc::new(10));
    test_generic::<Rc<i32>>(Rc::new(10));
}

#[test]
fn test_references() {
    test_generic_split::<&i32, i32, i32>(&10, |n| *n);
    test_generic_split::<&mut i32, i32, i32>(&mut 10, |n| *n);
}
