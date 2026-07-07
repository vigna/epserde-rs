/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Tests for deserialization errors caused by corrupted headers, corrupted
//! data, misaligned backends, and missing files.
//!
//! Header layout (see write_header): MAGIC (u64) at offset 0, VERSION_MAJOR
//! (u16) at 8, VERSION_MINOR (u16) at 10, USIZE_SIZE (u8) at 12, TYPE_HASH
//! (u64) at 13, ALIGN_HASH (u64) at 21, then TYPE_NAME (length as u64,
//! followed by the bytes).

use epserde::deser;
use epserde::prelude::*;

/// Serializes the given value and returns the raw byte stream.
fn serialized_bytes<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { value.serialize(&mut cursor)? };
    Ok(cursor.as_bytes().to_vec())
}

#[test]
fn test_align_hash_mismatch() -> anyhow::Result<()> {
    let mut bytes = serialized_bytes(&vec![1_i64, 2, 3])?;
    // Corrupt the alignment hash, which follows the 32-byte type hash.
    bytes[45] ^= 0xff;

    let mut cursor = <AlignedCursor>::from_slice(&bytes);
    let err = match unsafe { <Vec<i64>>::deserialize_full(&mut cursor) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    assert!(matches!(err, deser::Error::AlignHashMismatch { .. }));

    let err = match unsafe { <Vec<i64>>::deserialize_eps(cursor.as_bytes()) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    assert!(matches!(err, deser::Error::AlignHashMismatch { .. }));
    Ok(())
}

#[test]
fn test_usize_size_mismatch() -> anyhow::Result<()> {
    let mut bytes = serialized_bytes(&vec![1_i64, 2, 3])?;
    // Overwrite the usize width at offset 12 with a width that no
    // architecture has.
    bytes[12] = 3;

    let mut cursor = <AlignedCursor>::from_slice(&bytes);
    let err = match unsafe { <Vec<i64>>::deserialize_full(&mut cursor) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    match err {
        deser::Error::UsizeSizeMismatch(size) => assert_eq!(size, 3),
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    let err = match unsafe { <Vec<i64>>::deserialize_eps(cursor.as_bytes()) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    match err {
        deser::Error::UsizeSizeMismatch(size) => assert_eq!(size, 3),
        err => anyhow::bail!("wrong error type: {:?}", err),
    }
    Ok(())
}

#[test]
fn test_invalid_tag() -> anyhow::Result<()> {
    let a: Option<u8> = None;
    let mut bytes = serialized_bytes(&a)?;
    // The tag of the Option is the last byte of the stream.
    let last = bytes.len() - 1;
    bytes[last] = 7;

    let mut cursor = <AlignedCursor>::from_slice(&bytes);
    let err = match unsafe { <Option<u8>>::deserialize_full(&mut cursor) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    match err {
        deser::Error::InvalidTag(tag) => assert_eq!(tag, 7),
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    let err = match unsafe { <Option<u8>>::deserialize_eps(cursor.as_bytes()) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    match err {
        deser::Error::InvalidTag(tag) => assert_eq!(tag, 7),
        err => anyhow::bail!("wrong error type: {:?}", err),
    }
    Ok(())
}

#[test]
fn test_alignment_error() -> anyhow::Result<()> {
    let bytes = serialized_bytes(&vec![1_i64, 2, 3])?;
    // Copy the stream at offset 1 of an aligned buffer, so that the
    // resulting slice is misaligned for i64.
    let mut padded = vec![0_u8];
    padded.extend_from_slice(&bytes);
    let cursor = <AlignedCursor>::from_slice(&padded);

    let err = match unsafe { <Vec<i64>>::deserialize_eps(&cursor.as_bytes()[1..]) } {
        Ok(_) => anyhow::bail!("deserialization should have failed"),
        Err(err) => err,
    };
    assert!(matches!(err, deser::Error::AlignmentError));
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn test_file_open_error() -> anyhow::Result<()> {
    let result = unsafe { <Vec<i64>>::load_full("/nonexistent/epserde-test") };
    let err = match result {
        Ok(_) => anyhow::bail!("loading a nonexistent file should have failed"),
        Err(err) => err,
    };
    assert!(matches!(
        err.downcast_ref::<deser::Error>(),
        Some(deser::Error::FileOpenError(_))
    ));
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn test_ser_file_open_error() -> anyhow::Result<()> {
    use std::error::Error;

    let v = vec![1_i64];
    let err = match unsafe { v.store("/nonexistent/epserde-ser-test") } {
        Ok(_) => anyhow::bail!("storing to a nonexistent directory should have failed"),
        Err(err) => err,
    };
    assert!(matches!(err, epserde::ser::Error::FileOpenError(_)));
    assert!(err.source().is_some());
    Ok(())
}
