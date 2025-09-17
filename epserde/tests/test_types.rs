/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;
use std::iter;
use anyhow::Result;

macro_rules! impl_test {
    ($ty:ty, $val:expr) => {{
        let a = $val;
        let mut cursor = <AlignedCursor<A16>>::new();

        let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
        schema.0.sort_by_key(|a| a.offset);
        println!("{}", schema.to_csv());

        cursor.set_position(0);
        let a1 = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(a, a1);

        let a2 = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(a, a2);
    }};
}

#[test]
fn test_array_usize() -> Result<()> {
    let a = [1, 2, 3, 4, 5];

    let mut cursor = <AlignedCursor<A16>>::new();
    let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    cursor.set_position(0);
    let a1 = unsafe { <[usize; 5]>::deserialize_full(&mut cursor)? };
    assert_eq!(a, a1);

    let a2 = unsafe { <[usize; 5]>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *a2);
    Ok(())
}

#[test]
fn test_vec_usize() -> Result<()> {
    impl_test!(Vec<usize>, vec![1, 2, 3, 4, 5]);
    Ok(())
}

#[test]
fn test_box_slice_usize() -> Result<()> {
    let a = vec![1, 2, 3, 4, 5].into_boxed_slice();

    let mut cursor = <AlignedCursor<A16>>::new();
    let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    cursor.set_position(0);
    let a1 = unsafe { <Box<[usize]>>::deserialize_full(&mut cursor)? };
    assert_eq!(a, a1);

    let a2 = unsafe { <Box<[usize]>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, a2.into());
    Ok(())
}

#[test]
fn test_box_slice_string() -> Result<()> {
    let a = vec!["A".to_string(), "V".to_string()].into_boxed_slice();

    let mut cursor = <AlignedCursor<A16>>::new();
    let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    cursor.set_position(0);
    let a1 = unsafe { <Box<[String]>>::deserialize_full(&mut cursor)? };
    assert_eq!(a, a1);

    let a2 = unsafe { <Box<[String]>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a.len(), a2.len());
    a.iter().zip(a2.iter()).for_each(|(a, a2)| {
        assert_eq!(a, a2);
    });
    Ok(())
}

#[test]
fn test_vec_vec_usize() -> Result<()> {
    impl_test!(Vec<Vec<usize>>, vec![vec![1, 2, 3], vec![4, 5]]);
    Ok(())
}

#[test]
fn test_vec_array_string() -> Result<()> {
    impl_test!(
        Vec<[String; 2]>,
        vec![
            ["a".to_string(), "b".to_string()],
            ["c".to_string(), "aasfihjasomk".to_string()]
        ]
    );
    Ok(())
}

#[test]
fn test_vec_vec_string() -> Result<()> {
    impl_test!(
        Vec<Vec<String>>,
        vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "aasfihjasomk".to_string()]
        ]
    );
    Ok(())
}

#[test]
fn test_vec_vec_array_array_string() -> Result<()> {
    impl_test!(
        Vec<Vec<[[String; 2]; 2]>>,
        vec![
            vec![[
                ["a".to_string(), "b".to_string()],
                ["c".to_string(), "d".to_string()],
            ]],
            vec![[
                ["a".to_string(), "b".to_string()],
                ["c".to_string(), "d".to_string()],
            ]],
        ]
    );
    Ok(())
}

#[test]
fn test_vec_vec_array_array_usize() -> Result<()> {
    impl_test!(
        Vec<Vec<[[usize; 2]; 2]>>,
        vec![vec![[[1, 2], [3, 4],]], vec![[[5, 6], [7, 8],]],]
    );
    Ok(())
}

#[test]
fn test_struct_deep() -> Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    struct Struct {
        a: usize,
        b: usize,
        c: i32,
    }
    let a = Struct { a: 0, b: 1, c: 2 };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Struct>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);

    cursor.set_position(0);
    let eps = unsafe { <Struct>::deserialize_full(&mut cursor)? };
    assert_eq!(a, eps);
    Ok(())
}

#[test]
fn test_struct_zero() -> Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    #[repr(C)]
    #[zero_copy]
    struct Struct {
        a: usize,
        b: usize,
        c: i32,
    }
    let a = Struct { a: 0, b: 1, c: 2 };
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Struct>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);

    cursor.set_position(0);
    let eps = unsafe { <Struct>::deserialize_full(&mut cursor)? };
    assert_eq!(a, eps);
    Ok(())
}

#[test]
fn test_tuple_struct_deep() -> Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    struct Tuple(usize, usize, i32);
    let a = Tuple(0, 1, 2);
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Tuple>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);

    let eps = unsafe { <Tuple>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, eps);
    Ok(())
}

#[test]
fn test_tuple_struct_zero() -> Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    #[repr(C)]
    #[zero_copy]
    struct Tuple(usize, usize, i32);
    let a = Tuple(0, 1, 2);
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Tuple>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);

    let eps = unsafe { <Tuple>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *eps);
    Ok(())
}

#[test]
fn test_enum_deep() -> Result<()> {
    #[derive(Epserde, Clone, Debug, PartialEq)]
    enum Data<V = Vec<usize>> {
        A,
        B(u64),
        C(u64, Vec<usize>),
        D { a: i32, b: V },
        E,
    }

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::A;
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::A));

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::B(3);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::B(3)));

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::C(4, vec![1, 2, 3]);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::C(4, _)));

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::D {
        a: 1,
        b: vec![1, 2],
    };
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert!(matches!(full, Data::D { a: 1, b: _ }));
    let eps = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::D { a: 1, b: [1, 2] }));

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::E;
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::E));

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Vec::from_iter(iter::repeat(Data::A).take(10));
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<Data>>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Vec<Data>>::deserialize_eps(cursor.as_bytes())? };
    for e in eps {
        assert!(matches!(e, Data::A));
    }
    Ok(())
}

#[test]
fn test_enum_zero() -> Result<()> {
    #[derive(Epserde, Clone, Copy, Debug, PartialEq)]
    #[repr(C)]
    #[zero_copy]
    enum Data {
        A,
        B(u64),
        C(u64),
        D { a: i32, b: i32 },
    }

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::A;
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *eps);

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::B(3);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *eps);

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::C(4);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *eps);

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Data::D { a: 1, b: 2 };
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *eps);

    let mut cursor = <AlignedCursor<A16>>::new();
    let a = Vec::from_iter(iter::repeat(Data::A).take(10));
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<Data>>::deserialize_full(&mut cursor)? };
    assert_eq!(a, full);
    let eps = unsafe { <Vec<Data>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, *eps);
    Ok(())
}
