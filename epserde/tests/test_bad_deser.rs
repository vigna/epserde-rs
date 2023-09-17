/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use core::hash::Hasher;
use epserde::prelude::*;
use epserde::*;
use xxhash_rust::xxh3::Xxh3;

#[test]
fn test_wrong_endianess() {
    let data = 1337_usize;

    let len = 20;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    let mut buf = std::io::Cursor::new(&mut v);

    let schema = data.serialize_with_schema(&mut buf).unwrap();
    println!("{}", schema.debug(&v));
    println!("{:02x?}", &v);

    // set the reversed endianess
    v[0..8].copy_from_slice(&MAGIC_REV.to_ne_bytes());

    let err = <usize>::deserialize_full(&mut std::io::Cursor::new(&v));
    assert!(err.is_err());
    assert!(matches!(err.unwrap_err(), deser::Error::EndiannessError));

    let err = <usize>::deserialize_eps(&v);
    assert!(err.is_err());
    assert!(matches!(err.unwrap_err(), deser::Error::EndiannessError));

    // set a wrong magic cookie
    let bad_magic: u64 = 0x8989898989898989;
    v[0..8].copy_from_slice(&bad_magic.to_ne_bytes());

    let err = <usize>::deserialize_full(&mut std::io::Cursor::new(&v));
    if let Err(deser::Error::MagicCookieError(bad_magic_read)) = err {
        assert_eq!(bad_magic_read, bad_magic);
    } else {
        panic!("wrong error type: {:?}", err);
    }

    let err = <usize>::deserialize_eps(&v);
    if let Err(deser::Error::MagicCookieError(bad_magic_read)) = err {
        assert_eq!(bad_magic_read, bad_magic);
    } else {
        panic!("wrong error type: {:?}", err);
    }
    // reset the magic, but set a wrong version
    v[0..8].copy_from_slice(&MAGIC.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    v[8..10].copy_from_slice(&bad_version.to_ne_bytes());

    let err = <usize>::deserialize_full(&mut std::io::Cursor::new(&v));
    if let Err(deser::Error::MajorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type: {:?}", err);
    }

    let err = <usize>::deserialize_eps(&v);
    if let Err(deser::Error::MajorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type: {:?}", err);
    }

    // reset the Major version, but set a wrong minor version
    v[8..10].copy_from_slice(&VERSION.0.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    v[10..12].copy_from_slice(&bad_version.to_ne_bytes());

    let err = <usize>::deserialize_full(&mut std::io::Cursor::new(&v));
    if let Err(deser::Error::MinorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type {:?}", err);
    }

    let err = <usize>::deserialize_eps(&v);
    if let Err(deser::Error::MinorVersionMismatch(bad_version_read)) = err {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type {:?}", err);
    }

    // reset the minor version, but deserialize with the wrong type
    v[10..12].copy_from_slice(&VERSION.1.to_ne_bytes());

    let mut type_hasher = Xxh3::with_seed(0);
    <usize>::type_hash(&mut type_hasher);
    let usize_type_hash = type_hasher.finish();

    let mut type_hasher = Xxh3::with_seed(0);
    <i8>::type_hash(&mut type_hasher);
    let i8_hash = type_hasher.finish();

    let err = <i8>::deserialize_full(&mut std::io::Cursor::new(&v));
    if let Err(deser::Error::WrongTypeHash {
        got_type_name,
        got,
        expected,
        expected_type_name,
    }) = err
    {
        assert_eq!(got_type_name, "i8");
        assert_eq!(got, i8_hash);
        assert_eq!(expected, usize_type_hash);
        assert_eq!(expected_type_name, "usize");
    } else {
        panic!("wrong error type: {:?}", err);
    }
    let err = <i8>::deserialize_eps(&v);
    if let Err(deser::Error::WrongTypeHash {
        got_type_name,
        got,
        expected,
        expected_type_name,
    }) = err
    {
        assert_eq!(got_type_name, "i8");
        assert_eq!(got, i8_hash);
        assert_eq!(expected, usize_type_hash);
        assert_eq!(expected_type_name, "usize");
    } else {
        panic!("wrong error type: {:?}", err);
    }
}
