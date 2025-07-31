/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::marker::PhantomData;
use epserde::prelude::*;
use epserde::PhantomDeserData;
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
    let _bytes_written = unsafe { obj.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <PhantomData<usize>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <PhantomData<usize>>::deserialize_eps(cursor.as_bytes()).unwrap() };
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
    let _bytes_written = unsafe { obj.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <DataFull<NotSerializableType>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps =
        unsafe { <DataFull<NotSerializableType>>::deserialize_eps(cursor.as_bytes()).unwrap() };
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

/// Test that we can serialize a PhantomData in a zero-copy type if the argument
/// of the PhantomData is zero-copy. This should be a no-op.
#[test]
fn test_phantom_zero_copy() {
    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let obj = <DataZero<ZeroCopyType>>::default();

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = unsafe { <DataZero<ZeroCopyType>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(obj, zero);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <DataZero<ZeroCopyType>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(obj.a, eps.a);
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[zero_copy]
struct OnlyPhantom<A: Default + ZeroCopy> {
    a: PhantomData<A>,
    b: PhantomData<(A, A)>,
}

/// Test that we can serialize a zero-copy type containing a single PhantomData.
/// This should be a no-op.
#[test]
fn test_only_phantom() {
    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let obj = <OnlyPhantom<ZeroCopyType>>::default();

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = unsafe { <OnlyPhantom<ZeroCopyType>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(obj, zero);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <OnlyPhantom<ZeroCopyType>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(obj.a, eps.a);

    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let vec = vec![<OnlyPhantom<ZeroCopyType>>::default(); 10];

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { vec.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = unsafe { <Vec<OnlyPhantom<ZeroCopyType>>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(vec, zero);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps =
        unsafe { <Vec<OnlyPhantom<ZeroCopyType>>>::deserialize_eps(cursor.as_bytes()).unwrap() };
    assert_eq!(vec, eps);
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct DataWithPhantomDeserData<T> {
    data: T,
    phantom: PhantomDeserData<T>,
}

/// Test that PhantomDeserData works correctly with generic types that are
/// transformed during deserialization in a deep-copy type.
#[test]
fn test_deser_phantom_deep_copy() {
    let obj = DataWithPhantomDeserData {
        data: vec![1, 2, 3, 4],
        phantom: PhantomDeserData(PhantomData),
    };

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full =
        unsafe { <DataWithPhantomDeserData<Vec<i32>>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(obj, full);

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe {
        <DataWithPhantomDeserData<Vec<i32>>>::deserialize_eps(cursor.as_bytes()).unwrap()
    };

    // The data field should be transformed from Vec<i32> to &[i32]
    assert_eq!(obj.data.as_slice(), eps.data);

    // The phantom field should be PhantomData<&[i32]> (the DeserType of Vec<i32>)
    // We can't directly compare PhantomData types, but we can verify the deserialization worked
    let _phantom_check: PhantomDeserData<&[i32]> = eps.phantom;
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[zero_copy]
struct DataZeroWithPhantomDeserData<T: ZeroCopy> {
    data: T,
    phantom: PhantomDeserData<T>,
}

/// Test that PhantomDeserData works correctly with generic types that are
/// transformed during deserialization in a zero-copy type.
#[test]
fn test_deser_phantom_zero_copy() {
    let obj = DataZeroWithPhantomDeserData {
        data: [1, 2, 3, 4],
        phantom: PhantomDeserData(PhantomData),
    };

    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor).unwrap() };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full =
        unsafe { <DataZeroWithPhantomDeserData<[i32; 4]>>::deserialize_full(&mut cursor).unwrap() };
    assert_eq!(obj, full);

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe {
        <DataZeroWithPhantomDeserData<[i32; 4]>>::deserialize_eps(cursor.as_bytes()).unwrap()
    };

    // The data field should be transformed from Vec<i32> to &[i32]
    assert_eq!(obj.data.as_slice(), eps.data);

    // The phantom field should be PhantomDeserData<&[i32]> (the DeserType of Vec<i32>)
    // We can't directly compare PhantomData types, but we can verify the deserialization worked
    let _phantom_check: PhantomDeserData<[i32; 4]> = eps.phantom;
}
