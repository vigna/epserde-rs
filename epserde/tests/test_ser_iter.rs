/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Tests for SerIter, which serializes exact-size iterators as vectors.

use epserde::impls::iter::SerIter;
use epserde::prelude::*;
use epserde::ser;

#[test]
fn test_ser_iter_zero_copy() -> anyhow::Result<()> {
    let data = [1_i32, 2, 3];
    let iter = SerIter::<i32, _>::new(data.iter());
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { iter.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, vec![1, 2, 3]);

    let eps = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, &[1, 2, 3]);
    Ok(())
}

#[test]
fn test_ser_iter_deep_copy() -> anyhow::Result<()> {
    let data = vec!["a".to_string(), "b".to_string(), "longer".to_string()];
    let iter = SerIter::<String, _>::new(data.iter());
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { iter.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Vec<String>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, data);

    let eps = unsafe { <Vec<String>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(eps, vec!["a", "b", "longer"]);
    Ok(())
}

/// An exact-size iterator whose reported length is off by a given delta.
struct LyingLen<I: ExactSizeIterator> {
    inner: I,
    delta: isize,
}

impl<I: ExactSizeIterator> Iterator for LyingLen<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    // The lie must be consistent between len and size_hint, as
    // ExactSizeIterator requires.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len().checked_add_signed(self.delta).unwrap_or(0);
        (len, Some(len))
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for LyingLen<I> {
    fn len(&self) -> usize {
        self.inner.len().checked_add_signed(self.delta).unwrap_or(0)
    }
}

#[test]
fn test_ser_iter_len_over_reported() -> anyhow::Result<()> {
    let data = [1_i32, 2, 3];
    let iter = SerIter::<i32, _>::new(LyingLen {
        inner: data.iter(),
        delta: 1,
    });
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let err = match unsafe { iter.serialize(&mut cursor) } {
        Ok(_) => anyhow::bail!("serialization should have failed"),
        Err(err) => err,
    };
    match err {
        ser::Error::IteratorLengthMismatch { actual, expected } => {
            // The iterator returned three items but declared four.
            assert_eq!(actual, 3);
            assert_eq!(expected, 4);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }
    Ok(())
}

#[test]
fn test_ser_iter_len_under_reported() -> anyhow::Result<()> {
    let data = [1_i32, 2, 3];
    let iter = SerIter::<i32, _>::new(LyingLen {
        inner: data.iter(),
        delta: -1,
    });
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    let err = match unsafe { iter.serialize(&mut cursor) } {
        Ok(_) => anyhow::bail!("serialization should have failed"),
        Err(err) => err,
    };
    match err {
        ser::Error::IteratorLengthMismatch { actual, expected } => {
            // The iterator declared two items; the reported actual count is a
            // lower bound, as serialization stops writing at the declared
            // length.
            assert_eq!(actual, 3);
            assert_eq!(expected, 2);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }
    Ok(())
}

#[test]
fn test_ser_iter_serialized_twice() -> anyhow::Result<()> {
    let data = [1_i32, 2, 3];
    let iter = SerIter::<i32, _>::new(data.iter());
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { iter.serialize(&mut cursor)? };

    // Serialization consumes the wrapped iterator: a second serialization
    // must produce a valid, empty sequence.
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { iter.serialize(&mut cursor)? };
    cursor.set_position(0);
    let full = unsafe { <Vec<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(full, Vec::<i32>::new());
    Ok(())
}
