/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;

macro_rules! impl_test {
    ($ty:ty, $data:expr) => {{
        let mut cursor = <AlignedCursor<A16>>::new();

        let _ = unsafe { $data.ser_with_schema(&mut cursor).unwrap() };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deser_full(&mut cursor).unwrap() };
        assert_eq!($data, full_copy);

        let eps_copy = unsafe { <$ty>::deser_eps(cursor.as_bytes()).unwrap() };
        assert_eq!($data, *eps_copy);
    }
    {
        let mut cursor = <AlignedCursor<A16>>::new();
        unsafe { $data.serialize(&mut cursor).unwrap() };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deser_full(&mut cursor).unwrap() };
        assert_eq!($data, full_copy);

        let eps_copy = unsafe { <$ty>::deser_eps(cursor.as_bytes()).unwrap() };
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
test_zero!(test_tuple_0, (i32, i32), (-1_i32, 1_i32));
test_zero!(test_tuple_1, (i64, i64), (-1_i64, 1_i64));
test_zero!(
    test_tuple_2,
    ((i64, i64), (i64, i64)),
    ((-1_i64, 1_i64), (-1_i64, 1_i64))
);
test_zero!(
    test_tuple_3,
    ((i32, i32), (i32, i32)),
    ((-1_i32, 1_i32), (-1_i32, 1_i32))
);
