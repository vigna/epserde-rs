/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

macro_rules! impl_test {
    ($ty:ty, $data:expr) => {{
        let mut cursor = <AlignedCursor<Aligned16>>::new();

        let _ = unsafe { $data.serialize_with_schema(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(full_copy, $data);

        let eps_copy = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(*eps_copy, $data);
    }
    {
        let mut cursor = <AlignedCursor<Aligned16>>::new();
        unsafe { $data.serialize(&mut cursor)? };

        cursor.set_position(0);
        let full_copy = unsafe { <$ty>::deserialize_full(&mut cursor)? };
        assert_eq!(full_copy, $data);

        let eps_copy = unsafe { <$ty>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(*eps_copy, $data);
    }};
}

macro_rules! test_zero {
    ($test_name:ident, $ty:ty, $data: expr) => {
        #[test]
        fn $test_name() -> anyhow::Result<()> {
            impl_test!($ty, $data);
            Ok(())
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

#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
#[repr(C, align(16))]
#[epserde(zero_copy)]
struct ReprCAlign {
    a: i32,
    b: i32,
}

// repr(C) must be recognized also when combined with other
// representation hints in the same attribute
test_zero!(test_repr_c_align, ReprCAlign, ReprCAlign { a: -1, b: 1 });

#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
#[repr(align(16))]
#[repr(C)]
#[epserde(zero_copy)]
struct ReprCAlignSplit {
    a: i32,
    b: i32,
}

#[test]
fn test_repr_hash_normalization() {
    // Equivalent repr spellings must produce the same alignment hash
    let mut h1 = CryptoHasher::new();
    ReprCAlign::align_hash(&mut h1, &mut 0);
    let mut h2 = CryptoHasher::new();
    ReprCAlignSplit::align_hash(&mut h2, &mut 0);
    assert_eq!(h1.finalize(), h2.finalize());
}

// A data-carrying zero-copy enum with a sized discriminant representation
#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
#[repr(C, u8)]
#[epserde(zero_copy)]
enum DataEnum {
    A(u16) = 1,
    B(u32),
    C { a: i32, b: i32 } = 7,
}

test_zero!(test_data_enum_a, DataEnum, DataEnum::A(42));
test_zero!(test_data_enum_b, DataEnum, DataEnum::B(1000));
test_zero!(test_data_enum_c, DataEnum, DataEnum::C { a: -1, b: 1 });
