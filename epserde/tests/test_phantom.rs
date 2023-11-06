#![cfg(test)]

use core::marker::PhantomData;
use epserde::prelude::*;
use epserde::TypeInfo;

#[test]
/// Test that we can serialize and desertialize a PhantomData
/// This should be a NOOP
fn test_phantom() {
    // Create a new value to serialize
    let obj = <PhantomData<usize>>::default();
    // Create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // Wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = obj.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    let full = <PhantomData<usize>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <PhantomData<usize>>::deserialize_eps(&v).unwrap();
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
    // Create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // Wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = obj.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    let full = <Data<NotSerializable>>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
    assert_eq!(obj, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <Data<NotSerializable>>::deserialize_eps(&v).unwrap();
    assert_eq!(obj.a, eps.a);
}
