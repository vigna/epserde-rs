#![cfg(test)]

use epserde::*;
use std::hash::Hasher;
use xxhash_rust::xxh3::Xxh3;

#[test]
fn test_wrong_endianess() {
    let data = 1337_usize;

    let mut v = vec![];
    let mut buf = std::io::Cursor::new(&mut v);

    let _ = data.serialize_with_schema(&mut buf).unwrap();

    // set the reversed endianess
    v[0..8].copy_from_slice(&MAGIC_REV.to_ne_bytes());

    assert_eq!(
        <usize>::deserialize_full_copy(&v),
        Err(DeserializeError::EndiannessError)
    );
    assert_eq!(
        <usize>::deserialize_eps_copy(&v),
        Err(DeserializeError::EndiannessError)
    );
    println!("{:?}", <usize>::deserialize_eps_copy(&v));
    println!("{}", <usize>::deserialize_eps_copy(&v).unwrap_err());

    // set a wrong magic number
    let bad_magic: u64 = 0x8989898989898989;
    v[0..8].copy_from_slice(&bad_magic.to_ne_bytes());

    assert_eq!(
        <usize>::deserialize_full_copy(&v).unwrap_err(),
        DeserializeError::MagicNumberError(bad_magic),
    );
    assert_eq!(
        <usize>::deserialize_eps_copy(&v).unwrap_err(),
        DeserializeError::MagicNumberError(bad_magic),
    );
    println!("{:?}", <usize>::deserialize_eps_copy(&v));
    println!("{}", <usize>::deserialize_eps_copy(&v).unwrap_err());

    // reset the magic, but set a wrong version
    v[0..8].copy_from_slice(&MAGIC.to_ne_bytes());
    let bad_version: u32 = 0xffffffff;
    v[8..12].copy_from_slice(&bad_version.to_ne_bytes());

    assert_eq!(
        <usize>::deserialize_full_copy(&v).unwrap_err(),
        DeserializeError::MajorVersionMismatch(bad_version),
    );
    assert_eq!(
        <usize>::deserialize_eps_copy(&v).unwrap_err(),
        DeserializeError::MajorVersionMismatch(bad_version),
    );
    println!("{:?}", <usize>::deserialize_eps_copy(&v));
    println!("{}", <usize>::deserialize_eps_copy(&v).unwrap_err());

    // reset the Major version, but set a wrong minor version
    v[8..12].copy_from_slice(&VERSION.0.to_ne_bytes());
    let bad_version: u32 = 0xffffffff;
    v[12..16].copy_from_slice(&bad_version.to_ne_bytes());

    assert_eq!(
        <usize>::deserialize_full_copy(&v).unwrap_err(),
        DeserializeError::MinorVersionMismatch(bad_version),
    );
    assert_eq!(
        <usize>::deserialize_eps_copy(&v).unwrap_err(),
        DeserializeError::MinorVersionMismatch(bad_version),
    );
    println!("{:?}", <usize>::deserialize_eps_copy(&v));
    println!("{}", <usize>::deserialize_eps_copy(&v).unwrap_err());

    // reset the minor version, but deserialize with the wrong type
    v[12..16].copy_from_slice(&VERSION.1.to_ne_bytes());

    let mut hasher = Xxh3::with_seed(0);
    <usize>::type_hash(&mut hasher);
    let usize_hash = hasher.finish();

    let mut hasher = Xxh3::with_seed(0);
    <i128>::type_hash(&mut hasher);
    let i128_hash = hasher.finish();

    assert_eq!(
        <i128>::deserialize_full_copy(&v).unwrap_err(),
        DeserializeError::WrongTypeHash {
            got_type_name: "i128".to_string(),
            got: i128_hash,
            expected: usize_hash,
            expected_type_name: "usize".to_string(),
        },
    );
    assert_eq!(
        <i128>::deserialize_eps_copy(&v).unwrap_err(),
        DeserializeError::WrongTypeHash {
            got_type_name: "i128".to_string(),
            got: i128_hash,
            expected: usize_hash,
            expected_type_name: "usize".to_string(),
        },
    );
    println!("{:?}", <i128>::deserialize_eps_copy(&v));
    println!("{}", <i128>::deserialize_eps_copy(&v).unwrap_err());
}
