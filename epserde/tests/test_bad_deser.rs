#![cfg(test)]

use epserde::*;
use std::hash::Hasher;
use xxhash_rust::xxh3::Xxh3;

#[test]
fn test_wrong_endianess() {
    let data = 1337_usize;

    let mut v = vec![];
    let mut buf = std::io::Cursor::new(&mut v);

    let schema = data.serialize_with_schema(&mut buf).unwrap();
    println!("{}", schema.debug(&v));

    // set the reversed endianess
    v[0..8].copy_from_slice(&MAGIC_REV.to_ne_bytes());

    let err = <usize>::deserialize_full_copy(std::io::Cursor::new(&v));
    assert!(err.is_err());
    assert!(matches!(
        err.unwrap_err(),
        DeserializeError::EndiannessError
    ));

    let err = <usize>::deserialize_eps_copy(&v);
    assert!(err.is_err());
    assert!(matches!(
        err.unwrap_err(),
        DeserializeError::EndiannessError
    ));

    // set a wrong magic number
    let bad_magic: u64 = 0x8989898989898989;
    v[0..8].copy_from_slice(&bad_magic.to_ne_bytes());

    let err = <usize>::deserialize_full_copy(std::io::Cursor::new(&v));
    if let DeserializeError::MagicNumberError(bad_magic_read) = err.unwrap_err() {
        assert_eq!(bad_magic_read, bad_magic);
    } else {
        panic!("wrong error type");
    }

    let err = <usize>::deserialize_eps_copy(&v);
    if let DeserializeError::MagicNumberError(bad_magic_read) = err.unwrap_err() {
        assert_eq!(bad_magic_read, bad_magic);
    } else {
        panic!("wrong error type");
    }
    // reset the magic, but set a wrong version
    v[0..8].copy_from_slice(&MAGIC.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    v[8..10].copy_from_slice(&bad_version.to_ne_bytes());

    let err = <u16>::deserialize_full_copy(std::io::Cursor::new(&v));
    if let DeserializeError::MajorVersionMismatch(bad_version_read) = err.unwrap_err() {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type");
    }

    let err = <u16>::deserialize_eps_copy(&v);
    if let DeserializeError::MajorVersionMismatch(bad_version_read) = err.unwrap_err() {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type");
    }

    // reset the Major version, but set a wrong minor version
    v[8..10].copy_from_slice(&VERSION.0.to_ne_bytes());
    let bad_version: u16 = 0xffff;
    v[10..12].copy_from_slice(&bad_version.to_ne_bytes());

    let err = <u16>::deserialize_full_copy(std::io::Cursor::new(&v));
    if let DeserializeError::MinorVersionMismatch(bad_version_read) = err.unwrap_err() {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type");
    }

    let err = <u16>::deserialize_eps_copy(&v);
    if let DeserializeError::MinorVersionMismatch(bad_version_read) = err.unwrap_err() {
        assert_eq!(bad_version_read, bad_version);
    } else {
        panic!("wrong error type");
    }

    // reset the minor version, but deserialize with the wrong type
    v[10..12].copy_from_slice(&VERSION.1.to_ne_bytes());

    let mut hasher = Xxh3::with_seed(0);
    <usize>::type_hash(&mut hasher);
    let usize_hash = hasher.finish();

    let mut hasher = Xxh3::with_seed(0);
    <i128>::type_hash(&mut hasher);
    let i128_hash = hasher.finish();

    let err = <i128>::deserialize_full_copy(std::io::Cursor::new(&v));
    if let DeserializeError::WrongTypeHash {
        got_type_name,
        got,
        expected,
        expected_type_name,
    } = err.unwrap_err()
    {
        assert_eq!(got_type_name, "i128");
        assert_eq!(got, i128_hash);
        assert_eq!(expected, usize_hash);
        assert_eq!(expected_type_name, "usize");
    } else {
        panic!("wrong error type");
    }
    let err = <i128>::deserialize_eps_copy(&v);
    if let DeserializeError::WrongTypeHash {
        got_type_name,
        got,
        expected,
        expected_type_name,
    } = err.unwrap_err()
    {
        assert_eq!(got_type_name, "i128");
        assert_eq!(got, i128_hash);
        assert_eq!(expected, usize_hash);
        assert_eq!(expected_type_name, "usize");
    } else {
        panic!("wrong error type");
    }
}
