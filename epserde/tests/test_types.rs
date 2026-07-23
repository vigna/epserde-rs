/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;
use std::iter;

macro_rules! impl_test {
    ($ty:ty, $val:expr) => {{
        let a = $val;
        let mut cursor = <AlignedCursor<Aligned16>>::new();

        let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
        schema.0.sort_by_key(|a| a.offset);
        println!("{}", schema.to_csv());

        cursor.set_position(0);
        let a1 = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(a1, a);

        let a2 = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(a2, a);
    }};
}

#[test]
fn test_array_usize() -> anyhow::Result<()> {
    let a = [1, 2, 3, 4, 5];

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    cursor.set_position(0);
    let a1 = unsafe { <[usize; 5]>::deserialize_full(&mut cursor)? };
    assert_eq!(a1, a);

    let a2 = unsafe { <[usize; 5]>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*a2, a);
    Ok(())
}

#[test]
fn test_vec_usize() -> anyhow::Result<()> {
    impl_test!(Vec<usize>, vec![1, 2, 3, 4, 5]);
    Ok(())
}

#[test]
fn test_box_slice_usize() -> anyhow::Result<()> {
    let a = vec![1, 2, 3, 4, 5].into_boxed_slice();

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    cursor.set_position(0);
    let a1 = unsafe { <Box<[usize]>>::deserialize_full(&mut cursor)? };
    assert_eq!(a1, a);

    let a2 = unsafe { <Box<[usize]>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a2, &*a);
    Ok(())
}

#[test]
fn test_box_slice_string() -> anyhow::Result<()> {
    let a = vec!["A".to_string(), "V".to_string()].into_boxed_slice();

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let mut schema = unsafe { a.serialize_with_schema(&mut cursor)? };
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    cursor.set_position(0);
    let a1 = unsafe { <Box<[String]>>::deserialize_full(&mut cursor)? };
    assert_eq!(a1, a);

    let a2 = unsafe { <Box<[String]>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a2.len(), a.len());
    a.iter().zip(a2.iter()).for_each(|(a, a2)| {
        assert_eq!(a2, a);
    });
    Ok(())
}

#[test]
fn test_vec_vec_usize() -> anyhow::Result<()> {
    impl_test!(Vec<Vec<usize>>, vec![vec![1, 2, 3], vec![4, 5]]);
    Ok(())
}

#[test]
fn test_vec_array_string() -> anyhow::Result<()> {
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
fn test_vec_vec_string() -> anyhow::Result<()> {
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
fn test_vec_vec_array_array_string() -> anyhow::Result<()> {
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
fn test_vec_vec_array_array_usize() -> anyhow::Result<()> {
    impl_test!(
        Vec<Vec<[[usize; 2]; 2]>>,
        vec![vec![[[1, 2], [3, 4],]], vec![[[5, 6], [7, 8],]],]
    );
    Ok(())
}

#[test]
fn test_struct_deep() -> anyhow::Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    #[epserde(deep_copy)]
    struct Struct {
        a: usize,
        b: usize,
        c: i32,
    }
    let a = Struct { a: 0, b: 1, c: 2 };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Struct>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);

    let eps = unsafe { <Struct>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, a);
    Ok(())
}

#[test]
fn test_struct_zero() -> anyhow::Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    #[repr(C)]
    #[epserde(zero_copy)]
    struct Struct {
        a: usize,
        b: usize,
        c: i32,
    }
    let a = Struct { a: 0, b: 1, c: 2 };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Struct>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);

    let eps = unsafe { <Struct>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);
    Ok(())
}

#[test]
fn test_tuple_struct_deep() -> anyhow::Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    #[epserde(deep_copy)]
    struct Tuple(usize, usize, i32);
    let a = Tuple(0, 1, 2);
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Tuple>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);

    let eps = unsafe { <Tuple>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, a);
    Ok(())
}

#[test]
fn test_tuple_struct_zero() -> anyhow::Result<()> {
    #[derive(Epserde, Copy, Clone, Debug, PartialEq)]
    #[repr(C)]
    #[epserde(zero_copy)]
    struct Tuple(usize, usize, i32);
    let a = Tuple(0, 1, 2);
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { a.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Tuple>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);

    let eps = unsafe { <Tuple>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);
    Ok(())
}

#[test]
fn test_enum_deep() -> anyhow::Result<()> {
    #[derive(Epserde, Clone, Debug, PartialEq)]
    enum Data<V = Vec<usize>> {
        A,
        B(u64),
        C(u64, Vec<usize>),
        D { a: i32, b: V },
        E,
    }

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::A;
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::A));

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::B(3);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::B(3)));

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::C(4, vec![1, 2, 3]);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::C(4, _)));

    let mut cursor = <AlignedCursor<Aligned16>>::new();
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

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::E;
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert!(matches!(eps, Data::E));

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Vec::from_iter(iter::repeat_n(Data::A, 10));
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<Data>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Vec<Data>>::deserialize_eps(cursor.as_bytes())? };
    for e in eps {
        assert!(matches!(e, Data::A));
    }
    Ok(())
}

#[test]
fn test_enum_zero() -> anyhow::Result<()> {
    #[derive(Epserde, Clone, Copy, Debug, PartialEq)]
    #[repr(C)]
    #[epserde(zero_copy)]
    enum Data {
        A,
        B(u64),
        C(u64),
        D { a: i32, b: i32 },
    }

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::A;
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::B(3);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::C(4);
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Data::D { a: 1, b: 2 };
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Data>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let a = Vec::from_iter(iter::repeat_n(Data::A, 10));
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<Data>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    let eps = unsafe { <Vec<Data>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(*eps, a);
    Ok(())
}

// A generic struct whose only field is a smart-pointer wrapping a primitive
// must not trip the "could be zero-copy" lint. The field's serialized form is
// zero-copy-shaped via erasure, but the Rust layout is not: wrappers report
// IS_ZERO_COPY = false, so the lint stays silent.
#[derive(Epserde, Debug, PartialEq, Eq)]
struct BoxBox<T> {
    #[allow(clippy::redundant_allocation)]
    data: Box<Box<T>>,
}

#[test]
fn test_box_box_generic_compiles() -> anyhow::Result<()> {
    let a = BoxBox {
        data: Box::new(Box::new(42_i32)),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { a.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <BoxBox<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, a);
    Ok(())
}

/// Field names that are raw identifiers must not break the generated
/// serialization code (the derive rebinds named-variant fields).
#[test]
fn test_enum_raw_ident_fields() -> anyhow::Result<()> {
    #[derive(Epserde, Debug, PartialEq, Clone)]
    enum RawIdent {
        V { r#type: u32, r#loop: Vec<u8> },
    }

    let v = RawIdent::V {
        r#type: 7,
        r#loop: vec![1, 2],
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { v.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <RawIdent>::deserialize_full(&mut cursor)? };
    assert_eq!(full, v);
    let eps = unsafe { <RawIdent>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, v);
    Ok(())
}

/// Field names that collide with the derive's internal identifiers (like
/// backend) must not shadow the generated writer parameter.
#[test]
fn test_enum_field_named_backend() -> anyhow::Result<()> {
    #[derive(Epserde, Debug, PartialEq, Clone)]
    enum Internals {
        V { backend: usize, hasher: Vec<u32> },
    }

    let v = Internals::V {
        backend: 7,
        hasher: vec![1, 2, 3],
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { v.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Internals>::deserialize_full(&mut cursor)? };
    assert_eq!(full, v);
    let eps = unsafe { <Internals>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, v);
    Ok(())
}
