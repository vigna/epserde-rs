/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use epserde::*;

#[test]
fn test_wrong_endianness() -> anyhow::Result<()> {
    let data = 1337_usize;

    let mut cursor = <AlignedCursor<Aligned16>>::new();

    let schema = unsafe { data.serialize_with_schema(&mut cursor)? };
    println!("{}", schema.to_csv_with_data(cursor.as_bytes()));
    println!("{:02x?}", cursor.as_bytes());

    // set the reversed endianness
    cursor.as_bytes_mut()[0..8].copy_from_slice(&MAGIC_REV.to_ne_bytes());

    let err =
        unsafe { <usize>::deserialize_full(&mut <AlignedCursor>::from_slice(cursor.as_bytes())) };
    assert!(matches!(err, Err(deser::Error::EndiannessMismatch)));

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    assert!(matches!(err, Err(deser::Error::EndiannessMismatch)));

    // set a wrong magic cookie
    let bad_magic: u64 = 0x8989898989898989;
    cursor.as_bytes_mut()[0..8].copy_from_slice(&bad_magic.to_ne_bytes());

    let err =
        unsafe { <usize>::deserialize_full(&mut <AlignedCursor>::from_slice(cursor.as_bytes())) };
    match err {
        Err(deser::Error::InvalidMagicCookie(bad_magic_read)) => {
            assert_eq!(bad_magic_read, bad_magic);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    match err {
        Err(deser::Error::InvalidMagicCookie(bad_magic_read)) => {
            assert_eq!(bad_magic_read, bad_magic);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }
    // reset the magic, but set a wrong version
    cursor.as_bytes_mut()[0..8].copy_from_slice(&MAGIC.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    cursor.as_bytes_mut()[8..10].copy_from_slice(&bad_version.to_ne_bytes());

    let err =
        unsafe { <usize>::deserialize_full(&mut <AlignedCursor>::from_slice(cursor.as_bytes())) };
    match err {
        Err(deser::Error::MajorVersionMismatch(bad_version_read)) => {
            assert_eq!(bad_version_read, bad_version);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    match err {
        Err(deser::Error::MajorVersionMismatch(bad_version_read)) => {
            assert_eq!(bad_version_read, bad_version);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    // reset the Major version, but set a wrong minor version
    cursor.as_bytes_mut()[8..10].copy_from_slice(&VERSION.0.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    cursor.as_bytes_mut()[10..12].copy_from_slice(&bad_version.to_ne_bytes());

    let err =
        unsafe { <usize>::deserialize_full(&mut <AlignedCursor>::from_slice(cursor.as_bytes())) };
    match err {
        Err(deser::Error::MinorVersionMismatch(bad_version_read)) => {
            assert_eq!(bad_version_read, bad_version);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    match err {
        Err(deser::Error::MinorVersionMismatch(bad_version_read)) => {
            assert_eq!(bad_version_read, bad_version);
        }
        err => anyhow::bail!("wrong error type: {:?}", err),
    }

    // reset the minor version, but deserialize with the wrong type
    cursor.as_bytes_mut()[10..12].copy_from_slice(&VERSION.1.to_ne_bytes());

    let mut type_hasher = CryptoHasher::new();
    <usize>::type_hash(&mut type_hasher);
    let usize_type_hash = type_hasher.finalize();

    let mut type_hasher = CryptoHasher::new();
    <i8>::type_hash(&mut type_hasher);
    let i8_hash = type_hasher.finalize();

    let result =
        unsafe { <i8>::deserialize_full(&mut <AlignedCursor>::from_slice(cursor.as_bytes())) };
    match result {
        Err(deser::Error::TypeHashMismatch {
            ser_type_name,
            ser_type_hash,
            self_type_name,
            self_ser_type_name,
            self_type_hash,
        }) => {
            assert_eq!(ser_type_name, "usize");
            assert_eq!(ser_type_hash.0, usize_type_hash);
            assert_eq!(self_type_name, "i8");
            assert_eq!(self_ser_type_name, "i8");
            assert_eq!(self_type_hash.0, i8_hash);
        }
        result => anyhow::bail!("wrong result: {:?}", result),
    }

    let result = unsafe { <i8>::deserialize_eps(cursor.as_bytes()) };
    match result {
        Err(deser::Error::TypeHashMismatch {
            ser_type_name,
            ser_type_hash,
            self_type_name,
            self_ser_type_name,
            self_type_hash,
        }) => {
            assert_eq!(ser_type_name, "usize");
            assert_eq!(ser_type_hash.0, usize_type_hash);
            assert_eq!(self_type_name, "i8");
            assert_eq!(self_ser_type_name, "i8");
            assert_eq!(self_type_hash.0, i8_hash);
        }
        result => anyhow::bail!("wrong result: {:?}", result),
    }
    Ok(())
}

#[test]
fn test_error_at_eof() -> anyhow::Result<()> {
    let data = 1337_usize;

    let mut cursor = <AlignedCursor<Aligned16>>::new();

    unsafe { data.serialize(&mut cursor)? };
    cursor.set_len(cursor.position() - 1);
    cursor.set_position(0);
    let err = unsafe { <usize>::deserialize_full(&mut cursor) };
    assert!(err.is_err());
    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    assert!(err.is_err());
    Ok(())
}

#[test]
fn test_array_deep_deser_error_no_leak() -> anyhow::Result<()> {
    // Deserialization of a deep-copy array must not leak the elements already
    // deserialized when a later element fails; sweeping the truncation point
    // makes the failure happen at every possible position (leaks are checked
    // under Miri).
    let data = [vec![1_i32, 2], vec![3, 4]];
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { data.serialize(&mut cursor)? };
    let full = cursor.as_bytes().to_vec();
    for len in 0..full.len() {
        let mut cursor = <AlignedCursor>::from_slice(&full[..len]);
        assert!(unsafe { <[Vec<i32>; 2]>::deserialize_full(&mut cursor) }.is_err());
        let err = unsafe { <[Vec<i32>; 2]>::deserialize_eps(cursor.as_bytes()) };
        assert!(err.is_err());
    }
    Ok(())
}

#[test]
fn test_read_mem_error_no_leak() -> anyhow::Result<()> {
    // A deserialization failure inside read_mem must drop the memory backend
    // (leaks are checked under Miri).
    let data = 1337_usize;
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { data.serialize(&mut cursor)? };
    cursor.as_bytes_mut()[0..8].copy_from_slice(&0x8989898989898989_u64.to_ne_bytes());
    let bytes = cursor.as_bytes();
    let res = unsafe { <usize>::read_mem(bytes, bytes.len()) };
    assert!(res.is_err());
    Ok(())
}

#[test]
fn test_read_mem_empty() {
    // read_mem with no data must report an error without allocating a
    // zero-size layout.
    let res = unsafe { <usize>::read_mem(&b""[..], 0) };
    assert!(res.is_err());
}
