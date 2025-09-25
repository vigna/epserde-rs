/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use core::hash::Hasher;
use epserde::prelude::*;
use epserde::*;
use maligned::A16;
use xxhash_rust::xxh3::Xxh3;

#[test]
fn test_wrong_endianness() {
    let data = 1337_usize;

    let mut cursor = <AlignedCursor<A16>>::new();

    let schema = unsafe { data.ser_with_schema(&mut cursor).unwrap() };
    println!("{}", schema.debug(cursor.as_bytes()));
    println!("{:02x?}", cursor.as_bytes());

    // set the reversed endianness
    cursor.as_bytes_mut()[0..8].copy_from_slice(&MAGIC_REV.to_ne_bytes());

    let err = unsafe { <usize>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())) };
    assert!(err.is_err());
    assert!(matches!(err.unwrap_err(), deser::Error::EndiannessError));

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    assert!(err.is_err());
    assert!(matches!(err.unwrap_err(), deser::Error::EndiannessError));

    // set a wrong magic cookie
    let bad_magic: u64 = 0x8989898989898989;
    cursor.as_bytes_mut()[0..8].copy_from_slice(&bad_magic.to_ne_bytes());

    let err = unsafe { <usize>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())) };
    if let Err(deser::Error::MagicCookieError(bad_magic_read)) = err {
        assert_eq!(bad_magic_read, bad_magic);
    } else {
        panic!("wrong error type: {:?}", err);
    }

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    if let Err(deser::Error::MagicCookieError(bad_magic_read)) = err {
        assert_eq!(bad_magic_read, bad_magic);
    } else {
        panic!("wrong error type: {:?}", err);
    }
    // reset the magic, but set a wrong version
    cursor.as_bytes_mut()[0..8].copy_from_slice(&MAGIC.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    cursor.as_bytes_mut()[8..10].copy_from_slice(&bad_version.to_ne_bytes());

    let err = unsafe { <usize>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())) };
    if let Err(deser::Error::MajorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type: {:?}", err);
    }

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    if let Err(deser::Error::MajorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type: {:?}", err);
    }

    // reset the Major version, but set a wrong minor version
    cursor.as_bytes_mut()[8..10].copy_from_slice(&VERSION.0.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    cursor.as_bytes_mut()[10..12].copy_from_slice(&bad_version.to_ne_bytes());

    let err = unsafe { <usize>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())) };
    if let Err(deser::Error::MinorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type {:?}", err);
    }

    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    if let Err(deser::Error::MinorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type {:?}", err);
    }

    // reset the minor version, but deserialize with the wrong type
    cursor.as_bytes_mut()[10..12].copy_from_slice(&VERSION.1.to_ne_bytes());

    let mut type_hasher = Xxh3::with_seed(0);
    <usize>::type_hash(&mut type_hasher);
    let usize_type_hash = type_hasher.finish();

    let mut type_hasher = Xxh3::with_seed(0);
    <i8>::type_hash(&mut type_hasher);
    let i8_hash = type_hasher.finish();

    let result = unsafe { <i8>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())) };
    if let Err(err) = result {
        eprintln!("{err}");
        if let deser::Error::WrongTypeHash {
            ser_type_name,
            ser_type_hash,
            self_type_name,
            self_type_hash,
        } = err
        {
            assert_eq!(ser_type_name, "usize");
            assert_eq!(ser_type_hash, usize_type_hash);
            assert_eq!(self_type_name, "i8");
            assert_eq!(self_type_hash, i8_hash);
        } else {
            panic!("wrong error type: {:?}", err);
        }
    } else {
        panic!("No error: {:?}", err);
    }

    let result = unsafe { <i8>::deserialize_eps(cursor.as_bytes()) };
    if let Err(err) = result {
        eprintln!("{err}");
        if let deser::Error::WrongTypeHash {
            ser_type_name,
            ser_type_hash,
            self_type_name,
            self_type_hash,
        } = err
        {
            assert_eq!(ser_type_name, "usize");
            assert_eq!(ser_type_hash, usize_type_hash);
            assert_eq!(self_type_name, "i8");
            assert_eq!(self_type_hash, i8_hash);
        } else {
            panic!("wrong error type: {:?}", err);
        }
    } else {
        panic!("No error: {:?}", err);
    }
}

#[test]
fn test_error_at_eof() {
    let data = 1337_usize;

    let mut cursor = <AlignedCursor<A16>>::new();

    unsafe { data.serialize(&mut cursor).unwrap() };
    cursor.set_len(cursor.position() - 1);
    let err = unsafe { <usize>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes())) };
    assert!(err.is_err());
    let err = unsafe { <usize>::deserialize_eps(cursor.as_bytes()) };
    assert!(err.is_err());
}
