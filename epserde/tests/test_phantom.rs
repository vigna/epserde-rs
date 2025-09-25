/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::marker::PhantomData;
use epserde::PhantomDeserData;
use epserde::TypeInfo;
use epserde::prelude::*;
use maligned::A16;

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
