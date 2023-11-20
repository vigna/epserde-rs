#![cfg(test)]

use core::marker::PhantomData;
use epserde::new_aligned_cursor;
use epserde::prelude::*;
use epserde::TypeInfo;

#[test]
/// Test that we can serialize and desertialize a PhantomData
/// This should be a NOOP
fn test_phantom() {
    // Create a new value to serialize
    let obj = <PhantomData<usize>>::default();
    let mut buf = new_aligned_cursor();

    // Serialize
    let _bytes_written = obj.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <PhantomData<usize>>::deserialize_full(&mut buf).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    buf.set_position(0);
    let bytes = buf.into_inner();
    let eps = <PhantomData<usize>>::deserialize_eps(&bytes).unwrap();
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
    // Create a new value to serialize
    let obj = <Data<NotSerializable>>::default();
    let mut buf = new_aligned_cursor();

    // Serialize
    let _bytes_written = obj.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <Data<NotSerializable>>::deserialize_full(&mut buf).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    buf.set_position(0);
    let bytes = buf.into_inner();
    let eps = <Data<NotSerializable>>::deserialize_eps(&bytes).unwrap();
    assert_eq!(obj.a, eps.a);
}
