/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::marker::PhantomData;
use epserde::prelude::*;
use epserde::TypeInfo;
use maligned::A16;

#[test]
/// Test that we can serialize and deserialize a PhantomData
/// This should be a NOOP
fn test_phantom() {
    // Create a new value to serialize
    let obj = PhantomData::<usize>;
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = obj.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <PhantomData<usize>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <PhantomData<usize>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(obj, eps);
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct DataFull<D> {
    a: usize,
    b: PhantomData<D>,
}
#[derive(Debug, PartialEq, Eq, Clone, Default, TypeInfo)]
struct NotSerializableType;

/// Test that we can serialize a PhantomData of a non-serializable type
/// in a full-copy type.
/// This should be a no-op.
#[test]
fn test_not_serializable_in_phantom() {
    // Full copy with a non-serializable type
    let obj = <DataFull<NotSerializableType>>::default();

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = obj.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <DataFull<NotSerializableType>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = <DataFull<NotSerializableType>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(obj.a, eps.a);
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[zero_copy]
struct DataZero<A: Default + ZeroCopy> {
    a: usize,
    b: PhantomData<A>,
}
#[derive(Epserde, Debug, Copy, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[zero_copy]
struct ZeroCopyType;

/// Test that we can serialize a PhantomData in a zero-copy
/// type if the argument of the PhantomData is zero-copy.
/// This should be a no-op.
#[test]
fn test_phantom_zero_copy() {
    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let obj = <DataZero<ZeroCopyType>>::default();

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = obj.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = <DataZero<ZeroCopyType>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(obj, zero);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = <DataZero<ZeroCopyType>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(obj.a, eps.a);
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[zero_copy]
struct OnlyPhantom<A: Default + ZeroCopy> {
    a: PhantomData<A>,
    b: PhantomData<(A, A)>,
}

/// Test that we can serialize a zero-copy type containing a single
/// PhantomData.
/// This should be a no-op.
#[test]
fn test_only_phantom() {
    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let obj = <OnlyPhantom<ZeroCopyType>>::default();

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = obj.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = <OnlyPhantom<ZeroCopyType>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(obj, zero);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = <OnlyPhantom<ZeroCopyType>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(obj.a, eps.a);

    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let vec = vec![<OnlyPhantom<ZeroCopyType>>::default(); 10];

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = vec.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = <Vec<OnlyPhantom<ZeroCopyType>>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(vec, zero);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = <Vec<OnlyPhantom<ZeroCopyType>>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(vec, eps);
}
