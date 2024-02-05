#![cfg(test)]

use core::marker::PhantomData;
use epserde::prelude::*;
use epserde::TypeInfo;
use maligned::A16;

#[test]
/// Test that we can serialize and desertialize a PhantomData
/// This should be a NOOP
fn test_phantom() {
    // Create a new value to serialize
    let obj = <PhantomData<usize>>::default();
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
struct Data<A> {
    a: PhantomData<A>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, TypeInfo)]
struct NotSerializable;

#[test]
/// Test that we can serialize a Phantom Data of a non-serializable type
/// This should be a NOOP
fn test_not_serializable_in_phantom() {
    let obj = <Data<NotSerializable>>::default();
    let mut cursor = <AlignedCursor<A16>>::new();
    // Serialize
    let _bytes_written = obj.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = <Data<NotSerializable>>::deserialize_full(&mut cursor).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    cursor.set_position(0);
    let eps = <Data<NotSerializable>>::deserialize_eps(cursor.as_bytes()).unwrap();
    assert_eq!(obj.a, eps.a);
}
