/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::traits::{CryptoHasher, TypeHash};
use std::collections::HashMap;
macro_rules! impl_test {
    ($hashes:expr, $value:expr) => {{
        let mut hasher = CryptoHasher::new();
        ($value).type_hash_val(&mut hasher);
        let type_hash = hasher.finalize();
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
    impl_test!(hashes, (1_u8, 3_u8, 2_u8));

    dbg!(hashes);
}

#[test]
/// A zero-copy enum is (de)serialized as raw bytes, so its discriminant values
/// are part of the encoding: re-numbering variants must change the type hash,
/// or old data would silently mis-decode. The enums below are local to sibling
/// functions, so they share the same name and `module_path!()`; only the
/// discriminant differs, isolating its contribution to the hash.
fn test_zero_copy_enum_discriminant_hash() {
    // Explicit discriminants 0, 1.
    fn explicit_0_1() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C)]
        #[epserde(zero_copy)]
        enum E {
            A = 0,
            B = 1,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // Implicit discriminants, which resolve to 0, 1.
    fn implicit_0_1() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C)]
        #[epserde(zero_copy)]
        enum E {
            A,
            B,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // Explicit discriminants 0, 5.
    fn explicit_0_5() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C)]
        #[epserde(zero_copy)]
        enum E {
            A = 0,
            B = 5,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }

    // Re-numbering a variant changes the hash (detects the silent mis-decode).
    assert_ne!(explicit_0_1(), explicit_0_5());
    // Resolved discriminants are hashed, so an implicit mapping hashes equal to
    // the explicit mapping with the same values (symmetric treatment).
    assert_eq!(explicit_0_1(), implicit_0_1());
}

#[test]
/// A data-carrying zero-copy enum hashes the resolved value of each variant
/// discriminant, not its surface syntax: an implicit discriminant hashes equal
/// to an explicit one with the same value, and a named constant hashes equal
/// to the literal it resolves to. The enums below are local to sibling
/// functions, so they share the same name and module path; only the
/// discriminants differ, isolating their contribution to the hash.
fn test_zero_copy_data_enum_discriminant_hash() -> anyhow::Result<()> {
    // Explicit discriminants 1, 2.
    fn explicit_1_2() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u8)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = 1,
            B(u8) = 2,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // Explicit discriminant 1, implicit discriminant that resolves to 2.
    fn explicit_1_implicit_2() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u8)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = 1,
            B(u8),
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // Explicit discriminant 3, implicit discriminant that resolves to 4.
    fn explicit_3_implicit_4() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u8)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = 3,
            B(u8),
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // A named constant discriminant that resolves to 1.
    fn named_const_1() -> [u8; 32] {
        const FOO: u8 = 1;
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u8)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = FOO,
            B(u8) = 2,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }

    // The resolved values are hashed, so implicit and explicit forms of the
    // same mapping hash equal.
    assert_eq!(explicit_1_implicit_2(), explicit_1_2());
    // Re-numbering a variant changes the hash.
    assert_ne!(explicit_3_implicit_4(), explicit_1_2());
    // A named constant hashes as the value it resolves to.
    assert_eq!(named_const_1(), explicit_1_2());
    Ok(())
}

#[test]
/// A deep-copy enum is serialized field by field with its own tag, so the
/// declared discriminant values are not part of the encoding: re-numbering
/// the variants of a deep-copy enum must not change the type hash.
fn test_deep_copy_enum_discriminant_hash() -> anyhow::Result<()> {
    // Explicit discriminants 0, 1.
    fn explicit_0_1() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        enum E {
            A = 0,
            B = 1,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // Explicit discriminants 3, 5.
    fn explicit_3_5() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        enum E {
            A = 3,
            B = 5,
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }

    // Deep-copy deserialization ignores the declared discriminants, so the
    // hash must not depend on them.
    assert_eq!(explicit_3_5(), explicit_0_1());
    Ok(())
}

#[test]
fn test_type_hash_const_type_parameters() {
    #[derive(epserde::Epserde)]
    struct S<const N: usize>(std::marker::PhantomData<[u8; N]>);

    let mut hasher0 = CryptoHasher::new();
    let mut hasher1 = CryptoHasher::new();
    S::<0>::type_hash(&mut hasher0);
    S::<1>::type_hash(&mut hasher1);
    let digest0 = hasher0.finalize();
    let digest1 = hasher1.finalize();
    dbg!(digest0, digest1);
    assert_ne!(digest0, digest1);
}

/// An explicit discriminant expression whose type depends on the enum's
/// declared repr: the generated hash code must evaluate it at that type
/// (the sum below overflows the default i32 inference, but fits a u32),
/// and the resolved value must be hashed, so the sum must hash equal to
/// the equivalent literal. The enums are local to sibling functions, so
/// they share the same name and module path.
#[test]
fn test_zero_copy_data_enum_wide_discriminant() -> anyhow::Result<()> {
    // The discriminant as an expression overflowing i32 inference.
    fn sum() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u32)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = 2_000_000_000 + 2_000_000_000,
            B(u8),
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // The same discriminant as a literal.
    fn literal() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u32)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = 4_000_000_000,
            B(u8),
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }

    assert_eq!(sum(), literal());
    Ok(())
}

#[test]
/// A discriminant expression may mention Self, either as a Self-qualified
/// associated constant or, in a fieldless enum, as a cast of an earlier
/// variant. The mirror enum generated for the type hash replaces Self with the
/// enum's name, so such enums both compile and hash equal to the same enums
/// with the resolved literal discriminants. The enums below are local to
/// sibling functions, so they share the same name and module path; only the
/// discriminant spelling differs.
fn test_zero_copy_enum_self_discriminant() -> anyhow::Result<()> {
    // Data-carrying: a Self-qualified associated constant.
    fn data_with_self() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u8)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = Self::K,
            B(u8),
        }
        impl E {
            const K: u8 = 7;
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // The same discriminants as literals.
    fn data_with_literals() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C, u8)]
        #[epserde(zero_copy)]
        enum E {
            A(u8) = 7,
            B(u8),
        }
        let mut h = CryptoHasher::new();
        E::type_hash(&mut h);
        h.finalize()
    }
    // Fieldless: a cast of an earlier variant through Self.
    fn fieldless_with_self() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C)]
        #[epserde(zero_copy)]
        enum F {
            A = 3,
            B = Self::A as isize + 10,
        }
        let mut h = CryptoHasher::new();
        F::type_hash(&mut h);
        h.finalize()
    }
    // The same discriminants as literals.
    fn fieldless_with_literals() -> [u8; 32] {
        #[allow(dead_code)]
        #[derive(epserde::Epserde, Clone, Copy)]
        #[repr(C)]
        #[epserde(zero_copy)]
        enum F {
            A = 3,
            B = 13,
        }
        let mut h = CryptoHasher::new();
        F::type_hash(&mut h);
        h.finalize()
    }

    assert_eq!(data_with_self(), data_with_literals());
    assert_eq!(fieldless_with_self(), fieldless_with_literals());
    Ok(())
}
