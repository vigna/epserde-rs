/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

// Tests still exercise the deprecated PhantomDeserData type for
// backward-compatibility coverage; suppress the warnings file-wide.
#![allow(deprecated)]

use core::marker::PhantomData;
use epserde::PhantomDeserData;
use epserde::TypeInfo;
use epserde::prelude::*;

#[test]
/// Test that we can serialize and deserialize a PhantomData
/// This should be a NOOP
fn test_phantom() -> anyhow::Result<()> {
    // Create a new value to serialize
    let obj = PhantomData::<usize>;
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <PhantomData<usize>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    println!();

    // Do an ε-copy deserialization
    let eps = unsafe { <PhantomData<usize>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, obj);
    Ok(())
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
#[epserde(deep_copy)]
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
fn test_not_serializable_in_phantom() -> anyhow::Result<()> {
    // Full copy with a non-serializable type
    let obj = <DataFull<NotSerializableType>>::default();

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <DataFull<NotSerializableType>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <DataFull<NotSerializableType>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.a, obj.a);
    Ok(())
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[epserde(zero_copy)]
struct DataZero<A: Default + ZeroCopy> {
    a: usize,
    b: PhantomData<A>,
}

#[derive(Epserde, Debug, Copy, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[epserde(zero_copy)]
struct ZeroCopyType;

/// Test that we can serialize a PhantomData in a zero-copy type if the argument
/// of the PhantomData is zero-copy. This should be a no-op.
#[test]
fn test_phantom_zero_copy() -> anyhow::Result<()> {
    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let obj = <DataZero<ZeroCopyType>>::default();

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = unsafe { <DataZero<ZeroCopyType>>::deserialize_full(&mut cursor)? };
    assert_eq!(zero, obj);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <DataZero<ZeroCopyType>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.a, obj.a);
    Ok(())
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[epserde(zero_copy)]
struct OnlyPhantom<A: Default + ZeroCopy> {
    a: PhantomData<A>,
    b: PhantomData<(A, A)>,
}

/// Test that we can serialize a zero-copy type containing a single PhantomData.
/// This should be a no-op.
#[test]
fn test_only_phantom() -> anyhow::Result<()> {
    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let obj = <OnlyPhantom<ZeroCopyType>>::default();

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = unsafe { <OnlyPhantom<ZeroCopyType>>::deserialize_full(&mut cursor)? };
    assert_eq!(zero, obj);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <OnlyPhantom<ZeroCopyType>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.a, obj.a);

    // Zero copy needs a zero-copy type, even if inside a PhantomData
    let vec = vec![<OnlyPhantom<ZeroCopyType>>::default(); 10];

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { vec.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let zero = unsafe { <Vec<OnlyPhantom<ZeroCopyType>>>::deserialize_full(&mut cursor)? };
    assert_eq!(zero, vec);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <Vec<OnlyPhantom<ZeroCopyType>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, vec);
    Ok(())
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct DataWithPhantomDeserData<T> {
    data: T,
    phantom: PhantomDeserData<T>,
}

/// Test that PhantomDeserData works correctly with generic types that are
/// transformed during deserialization in a deep-copy type.
#[test]
fn test_deser_phantom_deep_copy() -> anyhow::Result<()> {
    let obj = DataWithPhantomDeserData {
        data: vec![1, 2, 3, 4],
        phantom: PhantomDeserData(PhantomData),
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <DataWithPhantomDeserData<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = unsafe { <DataWithPhantomDeserData<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };

    // The data field should be transformed from Vec<i32> to &[i32]
    assert_eq!(eps.data, obj.data.as_slice());

    // The phantom field should be PhantomData<&[i32]> (the DeserType of Vec<i32>)
    // We can't directly compare PhantomData types, but we can verify the deserialization worked
    let _phantom_check: PhantomDeserData<&[i32]> = eps.phantom;
    Ok(())
}

#[derive(Epserde, Copy, Debug, PartialEq, Eq, Clone, Default)]
#[repr(C)]
#[epserde(zero_copy)]
struct DataZeroWithPhantomDeserData<T: ZeroCopy> {
    data: T,
    phantom: PhantomDeserData<T>,
}

/// Test that PhantomDeserData works correctly with generic types that are
/// transformed during deserialization in a zero-copy type.
#[test]
fn test_deser_phantom_zero_copy() -> anyhow::Result<()> {
    let obj = DataZeroWithPhantomDeserData {
        data: [1, 2, 3, 4],
        phantom: PhantomDeserData(PhantomData),
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // Serialize
    let _bytes_written = unsafe { obj.serialize(&mut cursor)? };

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = unsafe { <DataZeroWithPhantomDeserData<[i32; 4]>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps =
        unsafe { <DataZeroWithPhantomDeserData<[i32; 4]>>::deserialize_eps(cursor.as_bytes())? };

    // The data field should be transformed from Vec<i32> to &[i32]
    assert_eq!(eps.data, obj.data.as_slice());

    // The phantom field should be PhantomDeserData<&[i32]> (the DeserType of Vec<i32>)
    // We can't directly compare PhantomData types, but we can verify the deserialization worked
    let _phantom_check: PhantomDeserData<[i32; 4]> = eps.phantom;
    Ok(())
}

#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct DataWithPhantomData<T> {
    data: T,
    phantom: PhantomData<T>,
}

#[test]
fn test_phantom_data_substitution() -> anyhow::Result<()> {
    let obj: DataWithPhantomData<Vec<i32>> = DataWithPhantomData {
        data: vec![1, 2, 3, 4],
        phantom: PhantomData,
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { obj.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <DataWithPhantomData<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    let eps = unsafe { <DataWithPhantomData<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    // The data field comes back as &[i32] (Vec<i32>::DeserType<'_>).
    assert_eq!(eps.data, obj.data.as_slice());
    // The phantom field has type PhantomData<&[i32]>. The annotation
    // forces the type-check; if PhantomData were not substituting its
    // parameter, this line would fail to compile.
    let _phantom_check: PhantomData<&[i32]> = eps.phantom;

    Ok(())
}

// PhantomDeserData<T> is not a barrier in the classifier (only
// PhantomData is), so an occurrence of T inside PhantomDeserData<T>
// is a variable position and T becomes ε-copy by default, even
// when no other field of the struct mentions T.
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
#[epserde(deep_copy)]
struct OnlyPhantomDeserData<T> {
    other: u32,
    phantom: PhantomDeserData<T>,
}

#[test]
fn test_only_phantom_deser_data() -> anyhow::Result<()> {
    let obj: OnlyPhantomDeserData<Vec<i32>> = OnlyPhantomDeserData {
        other: 42,
        phantom: PhantomDeserData(PhantomData),
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { obj.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <OnlyPhantomDeserData<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    let eps = unsafe { <OnlyPhantomDeserData<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.other, obj.other);
    // The phantom slot must be PhantomDeserData<&[i32]> after T is
    // substituted into <Vec<i32> as DeserInner>::DeserType<'_>.
    let _phantom_check: PhantomDeserData<&[i32]> = eps.phantom;

    Ok(())
}

/// A fully qualified PhantomData with a leading path separator must be
/// recognized by the derive's phantom detection.
#[test]
fn test_leading_separator_phantom_data() -> anyhow::Result<()> {
    #[derive(Epserde, Debug, PartialEq, Eq, Clone)]
    struct Data<T: DeepCopy> {
        data: Vec<T>,
        marker: ::core::marker::PhantomData<T>,
    }

    let obj = Data::<Vec<i32>> {
        data: vec![vec![1], vec![2, 3]],
        marker: PhantomData,
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { obj.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Data<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);
    let eps = unsafe { <Data<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.data, vec![&[1_i32][..], &[2, 3][..]]);
    Ok(())
}

/// The TypeInfo derive alone on a generic zero-copy type must not require
/// SerInner on the type parameter.
#[test]
fn test_type_info_generic_zero_copy() {
    use epserde::traits::{AlignHash, PadTo};

    #[allow(dead_code)]
    #[derive(TypeInfo, Copy, Clone)]
    #[repr(C)]
    #[epserde(zero_copy)]
    struct GenericZero<T: TypeHash + AlignHash + PadTo + Copy + 'static> {
        x: T,
    }

    let mut hasher = CryptoHasher::new();
    <GenericZero<u32>>::type_hash(&mut hasher);
    assert_ne!(hasher.finalize(), [0u8; 32]);
}

// PhantomData<fn() -> K> is the idiomatic marker for a type that does not own
// a K: it is Send and Sync whatever K is, and K can be unsized.
#[derive(Epserde, Debug, PartialEq, Eq)]
struct FnPtrMarker<K: ?Sized, A> {
    a: A,
    _marker: PhantomData<fn() -> K>,
}

#[test]
fn test_fn_ptr_phantom() -> anyhow::Result<()> {
    let obj = FnPtrMarker::<str, Vec<usize>> {
        a: vec![1, 2, 3],
        _marker: PhantomData,
    };

    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { obj.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <FnPtrMarker<str, Vec<usize>>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, obj);

    let eps = unsafe { <FnPtrMarker<str, Vec<usize>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps.a, obj.a.as_slice());

    // Rc<u8> is neither Send nor Sync, but the structure is
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FnPtrMarker<std::rc::Rc<u8>, Vec<usize>>>();
    Ok(())
}

/// Function pointers contribute to the type hash, as *const does, and so do
/// their argument and return types.
#[test]
fn test_fn_ptr_phantom_type_hash() {
    fn type_hash<T: ?Sized + TypeHash>() -> [u8; 32] {
        let mut hasher = CryptoHasher::new();
        T::type_hash(&mut hasher);
        hasher.finalize()
    }

    let fn_ptr = type_hash::<PhantomData<fn() -> str>>();
    assert_ne!(fn_ptr, type_hash::<PhantomData<str>>());
    assert_ne!(fn_ptr, type_hash::<PhantomData<*const str>>());
    assert_ne!(
        type_hash::<PhantomData<fn() -> *const str>>(),
        type_hash::<PhantomData<*const str>>()
    );

    assert_ne!(type_hash::<fn(u8) -> str>(), fn_ptr);
    assert_ne!(type_hash::<fn(u8) -> u16>(), type_hash::<fn(u16) -> u8>());
    assert_ne!(type_hash::<fn(u8, u16)>(), type_hash::<fn((u8, u16))>());
    assert_ne!(type_hash::<fn(u8)>(), type_hash::<fn() -> u8>());
    assert_ne!(type_hash::<fn(u8)>(), type_hash::<(u8,)>());
    // Unsized argument types are accepted, too
    assert_ne!(type_hash::<fn(str)>(), type_hash::<fn([u8])>());
}
