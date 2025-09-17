/*
 * SPDX-FileCopyrightText: 2025 Inria
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;
use std::{rc::Rc, sync::Arc};
use anyhow::Result;

fn test_generic<T>(s: T) -> Result<()>
where
    T: Serialize + Deserialize + PartialEq + core::fmt::Debug,
    for<'a> <T as DeserializeInner>::DeserType<'a>: PartialEq<T> + core::fmt::Debug,
{
    test_generic_split::<T, T, T>(s, |value| value)
}

fn test_generic_split<Ser, Deser, OwnedSer>(
    s: Ser,
    deref: impl Fn(&Ser) -> &OwnedSer,
) -> Result<()>
where
    Ser: Serialize,
    Deser: Deserialize + PartialEq<OwnedSer> + core::fmt::Debug,
    OwnedSer: core::fmt::Debug,
    for<'a> <Deser as DeserializeInner>::DeserType<'a>: PartialEq<OwnedSer> + core::fmt::Debug,
{
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);

        let mut schema = unsafe { s.serialize_with_schema(&mut cursor)? };
        schema.0.sort_by_key(|a| a.offset);

        cursor.set_position(0);
        let full_copy = unsafe { <Deser>::deserialize_full(&mut cursor)? };
        assert_eq!(&full_copy, deref(&s));

        let full_copy = unsafe { <Deser>::deserialize_eps(&v)? };
        assert_eq!(&full_copy, deref(&s));

        let _ = schema.to_csv();
        let _ = schema.debug(&v);
    }
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);
        unsafe { s.serialize(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <Deser>::deserialize_full(&mut cursor)? };
        assert_eq!(&full_copy, deref(&s));

        let full_copy = unsafe { <Deser>::deserialize_eps(&v)? };
        assert_eq!(&full_copy, deref(&s));
    }

    Ok(())
}

#[test]
fn test_containers() -> Result<()> {
    test_generic::<Box<i32>>(Box::new(10))?;
    test_generic::<Arc<i32>>(Arc::new(10))?;
    test_generic::<Rc<i32>>(Rc::new(10))?;
    Ok(())
}

#[test]
fn test_references() -> Result<()> {
    test_generic_split::<&i32, i32, i32>(&10, |n| *n)?;
    test_generic_split::<&mut i32, i32, i32>(&mut 10, |n| *n)?;
    Ok(())
}

#[test]
fn test_erasure_vec() -> Result<()> {
    let data = vec![1, 2, 3];
    let mut cursor = <AlignedCursor<A16>>::new();
    unsafe { data.serialize(&mut cursor)? };

    cursor.set_position(0);
    let boxed = unsafe { <Box<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(data, *boxed);
    cursor.set_position(0);
    let rc = unsafe { <Rc<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(data, *rc);
    cursor.set_position(0);
    let arc = unsafe { <Arc<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(data, *arc);

    let boxed: Box<&[i32]> = unsafe { <Box<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(data, *boxed);
    let rc: Rc<&[i32]> = unsafe { <Rc<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(data, *rc);
    let arc: Arc<&[i32]> = unsafe { <Arc<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(data, *arc);

    let data = Box::new(vec![1, 2, 3]);
    cursor.set_position(0);
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let unbox = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(*data, unbox);
    let unbox: &[i32] = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*data, unbox);

    let data = Rc::new(vec![1, 2, 3]);
    cursor.set_position(0);
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let unrc = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(*data, unrc);
    let unrc: &[i32] = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*data, unrc);

    let data = Arc::new(vec![1, 2, 3]);
    cursor.set_position(0);
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let unarc = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(*data, unarc);
    let unarc: &[i32] = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*data, unarc);

    Ok(())
}

#[test]
fn test_erasure_struct() -> Result<()> {
    #[derive(Epserde, PartialEq, Eq, Debug)]
    struct Data<A>(A);
    let data = Data(vec![1, 2, 3]);
    let mut cursor = <AlignedCursor<A16>>::new();
    unsafe { data.serialize(&mut cursor)? };

    cursor.set_position(0);
    let boxed = unsafe { <Data<Box<Vec<i32>>>>::deserialize_full(&mut cursor)? };
    assert_eq!(data.0, *boxed.0);
    cursor.set_position(0);
    let rc = unsafe { <Data<Rc<Vec<i32>>>>::deserialize_full(&mut cursor)? };
    assert_eq!(data.0, *rc.0);
    cursor.set_position(0);
    let arc = unsafe { <Data<Arc<Vec<i32>>>>::deserialize_full(&mut cursor)? };
    assert_eq!(data.0, *arc.0);

    let boxed: Data<Box<&[i32]>> =
        unsafe { <Data<Box<Vec<i32>>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(data.0, *boxed.0);
    let rc: Data<Rc<&[i32]>> = unsafe { <Data<Rc<Vec<i32>>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(data.0, *rc.0);
    let arc: Data<Arc<&[i32]>> =
        unsafe { <Data<Arc<Vec<i32>>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(data.0, *arc.0);

    let data = Data(Box::new(vec![1, 2, 3]));
    cursor.set_position(0);
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let unbox = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(*data.0, unbox.0);
    let unbox: Data<&[i32]> = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*data.0, unbox.0);

    let data = Data(Rc::new(vec![1, 2, 3]));
    cursor.set_position(0);
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let unrc = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(*data.0, unrc.0);
    let unrc: Data<&[i32]> = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*data.0, unrc.0);

    let data = Data(Arc::new(vec![1, 2, 3]));
    cursor.set_position(0);
    unsafe { data.serialize(&mut cursor)? };
    cursor.set_position(0);
    let unarc = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(*data.0, unarc.0);
    let unarc: Data<&[i32]> = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*data.0, unarc.0);

    Ok(())
}
