/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use core::hash::Hasher;
use epserde::traits::TypeHash;
use std::collections::HashMap;
use xxhash_rust::xxh3::Xxh3;
macro_rules! impl_test {
    ($hashes:expr, $value:expr) => {{
        let mut type_hasher = Xxh3::with_seed(0);
        ($value).type_hash_val(&mut type_hasher);
        let type_hash = type_hasher.finish();
        let res = $hashes.insert(type_hash, stringify!($value));
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

    // TODO doesn't compile anymore
    // impl_test!(hashes, vec![1_i8, 2, 3, 4, 5].as_slice());

    dbg!(hashes);
}
