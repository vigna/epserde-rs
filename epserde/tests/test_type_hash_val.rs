#![cfg(test)]

use epserde::*;
use std::collections::HashMap;
use std::hash::Hasher;
use xxhash_rust::xxh3::Xxh3;

macro_rules! impl_test {
    ($hashes:expr, $value:expr) => {{
        let mut hasher = Xxh3::with_seed(0);
        ($value).type_hash_val(&mut hasher);
        let hash = hasher.finish();
        let res = $hashes.insert(hash, stringify!($value));
        assert!(
            res.is_none(),
            "Collision on type {} with {}",
            stringify!($value),
            res.unwrap()
        );
    }};
}

macro_rules! impl_test_type {
    ($hashes:expr, $value:ty) => {{
        let mut hasher = Xxh3::with_seed(0);
        <$value>::type_hash(&mut hasher);
        let hash = hasher.finish();
        let res = $hashes.insert(hash, stringify!($value));
        assert!(
            res.is_none(),
            "Collision on type {} with {}",
            stringify!($value),
            res.unwrap()
        );
    }};
}

#[test]
/// Check that we don't have any collision on most types
fn test_type_hash_collision() {
    let mut hashes = HashMap::new();
    impl_test!(hashes, ());
    impl_test!(hashes, true);
    impl_test!(hashes, '🔥');
    impl_test!(hashes, Some('🔥'));
    impl_test!(hashes, Some(1_u8));
    impl_test!(hashes, <std::result::Result<char, bool>>::Ok('🔥'));
    impl_test!(hashes, <std::result::Result<char, char>>::Ok('🔥'));

    impl_test!(hashes, 1_u8);
    impl_test!(hashes, 1_u16);
    impl_test!(hashes, 1_u32);
    impl_test!(hashes, 1_u64);
    impl_test!(hashes, 1_u128);
    impl_test!(hashes, 1_usize);
    impl_test!(hashes, 1_i8);
    impl_test!(hashes, 1_i16);
    impl_test!(hashes, 1_i32);
    impl_test!(hashes, 1_i64);
    impl_test!(hashes, 1_i128);
    impl_test!(hashes, 1_isize);

    impl_test!(hashes, vec![1_usize, 2, 3, 4, 5]);
    impl_test!(hashes, vec![1_u8, 2, 3, 4, 5]);
    impl_test!(hashes, vec![1_i8, 2, 3, 4, 5]);
    impl_test!(hashes, (1_u8, 3_u16, '🔥'));

    impl_test!(hashes, vec![1_i8, 2, 3, 4, 5].as_slice());

    impl_test_type!(hashes, &[u8]);

    #[cfg(feature = "mmap-rs")]
    {
        impl_test!(
            hashes,
            mmap_rs::MmapOptions::new(1024).unwrap().map().unwrap()
        );
        impl_test!(
            hashes,
            mmap_rs::MmapOptions::new(1024).unwrap().map_mut().unwrap()
        );
    }
    dbg!(hashes);
}
