/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use anyhow::Result;
use epserde::prelude::*;
use epserde::ser;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data<A: PartialEq = usize, const Q: usize = 3> {
    a: A,
    b: [i32; Q],
}

#[test]
fn test_slices() -> Result<()> {
    let a = vec![1, 2, 3, 4];
    let s = a.as_slice();
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { s.serialize(&mut cursor)? };
    cursor.set_position(0);
    let b = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(a, b.as_slice());
    let b = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, b);

    cursor.set_position(0);
    let d = Data {
        a: vec![0, 1, 2, 3].into_boxed_slice(),
        b: [1, 2, 3],
    };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Box<[i32]>>::deserialize_full(&mut cursor)? };
    assert_eq!(e, d);

    cursor.set_position(0);
    let v = vec![0, 1, 2, 3];
    let d = Data {
        a: v.as_slice(),
        b: [1, 2, 3],
    };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Box<[i32]>>::deserialize_full(&mut cursor)? };
    assert_eq!(&*e.a, d.a);

    cursor.set_position(0);
    let d = Data { a: s, b: [1, 2, 3] };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(e, d);

    Ok(())
}

#[test]
fn test_mut_slices() -> Result<()> {
    let mut a = vec![1, 2, 3, 4];
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    {
        let s = a.as_mut_slice();
        unsafe { s.serialize(&mut cursor)? };
        cursor.set_position(0);
    }

    let b = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(a, b);
    let b = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(a, b);

    cursor.set_position(0);
    let d = Data {
        a: vec![0, 1, 2, 3].into_boxed_slice(),
        b: [1, 2, 3],
    };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Box<[i32]>>::deserialize_full(&mut cursor)? };
    assert_eq!(e, d);

    cursor.set_position(0);
    let mut v = vec![0, 1, 2, 3];
    let d = Data {
        a: v.as_mut_slice(),
        b: [1, 2, 3],
    };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Box<[i32]>>::deserialize_full(&mut cursor)? };
    assert_eq!(&*e.a, d.a);

    cursor.set_position(0);
    let d = Data {
        a: a.as_mut_slice(),
        b: [1, 2, 3],
    };
    unsafe { d.serialize(&mut cursor)? };
    cursor.set_position(0);
    let e = unsafe { Data::<Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(e.a, d.a);
    assert_eq!(e.b, d.b);
    Ok(())
}

/// A writer that accepts exactly `budget` bytes and then errors.
struct BudgetWriter {
    budget: usize,
}

// WriteNoStd is implemented directly, rather than through std::io::Write and
// the std-only blanket implementation, so that this test also compiles
// without the std feature.
impl ser::WriteNoStd for BudgetWriter {
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        let n = buf.len().min(self.budget);
        self.budget -= n;
        if n < buf.len() {
            return Err(ser::Error::WriteError);
        }
        Ok(())
    }

    fn flush(&mut self) -> ser::Result<()> {
        Ok(())
    }
}

#[test]
fn test_slice_ser_error_no_free() -> Result<()> {
    // Serializing a borrowed slice must not free the borrowed memory when the
    // writer errors, at whatever point the error happens; sweeping the budget
    // makes the write fail at every possible position (checked under Miri).
    let a = vec![1, 2, 3, 4];
    let s = &a[1..];
    let mut succeeded = false;
    for budget in 0..10_000 {
        let mut w = BudgetWriter { budget };
        if unsafe { s.serialize(&mut w) }.is_ok() {
            succeeded = true;
            break;
        }
    }
    assert!(succeeded);
    assert_eq!(a, vec![1, 2, 3, 4]);
    Ok(())
}
