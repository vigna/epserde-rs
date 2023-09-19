/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use epserde::prelude::*;

macro_rules! impl_test {
    ($ty:ty, $data:expr) => {{
        let mut cursor = epserde::new_aligned_cursor();

        let _ = $data.serialize_with_schema(&mut cursor).unwrap();

        cursor.set_position(0);
        let full_copy = <$ty>::deserialize_full(&mut cursor).unwrap();
        assert_eq!($data, full_copy);

        cursor.set_position(0);
        let slice = cursor.into_inner();
        let eps_copy = <$ty>::deserialize_eps(&slice).unwrap();
        assert_eq!($data, *eps_copy);
    }
    {
        let mut cursor = epserde::new_aligned_cursor();

        $data.serialize(&mut cursor).unwrap();

        cursor.set_position(0);
        let full_copy = <$ty>::deserialize_full(&mut cursor).unwrap();
        assert_eq!($data, full_copy);

        cursor.set_position(0);
        let slice = cursor.into_inner();
        let eps_copy = <$ty>::deserialize_eps(&slice).unwrap();
        assert_eq!($data, *eps_copy);
    }};
}

macro_rules! test_zero {
    ($test_name:ident, $ty:ty, $data: expr) => {
        #[test]
        fn $test_name() {
            impl_test!($ty, $data);
        }
    };
}

test_zero!(test_array_i32_2, [i32; 2], [-1, 1]);
test_zero!(test_array_i64_2, [i64; 2], [-1_i64, 1]);
test_zero!(test_tuple_0, (i32, i64), (-1, 1_i64));
test_zero!(test_tuple_1, (i64, i32), (-1_i64, 1));
test_zero!(test_tuple_2, ((i64, i32), i32), ((-1_i64, 1), -1));
test_zero!(test_tuple_3, ((i32, i64), i32), ((-1, 1_i64), -1));
test_zero!(
    test_array_tuple_0,
    [((i32, i64), i32); 2],
    [((-1, 1_i64), -1), ((-2, 2_i64), -2)]
);
test_zero!(
    test_array_tuple_1,
    [((i64, i32), i32); 2],
    [((-1_i64, 1), -1), ((-2_i64, 2), -2)]
);
