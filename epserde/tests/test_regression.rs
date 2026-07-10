/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use epserde::ser::SerType;

fn hex(hasher: CryptoHasher) -> String {
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn get_type_hash<T: TypeHash + ?Sized>() -> String {
    let mut hasher = CryptoHasher::new();
    T::type_hash(&mut hasher);
    hex(hasher)
}

fn get_align_hash<T: AlignHash + ?Sized>() -> String {
    let mut hasher = CryptoHasher::new();
    let mut offset = 0;
    T::align_hash(&mut hasher, &mut offset);
    hex(hasher)
}

#[derive(Epserde, Debug, PartialEq)]
struct MyStruct {
    a: i32,
    b: f64,
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructGeneric<T: PartialEq> {
    a: T,
}

#[derive(Epserde, Debug, PartialEq)]
enum MyEnum {
    A,
    B(i32),
    C { a: i32, b: f64 },
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructConst<const N: usize> {
    a: [i32; N],
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructMixed<T: PartialEq, const N: usize> {
    a: T,
    b: [i32; N],
}

#[derive(Epserde, Debug, PartialEq, Eq)]
struct MyStructConstThenType<const N: usize, T: PartialEq> {
    a: [i32; N],
    b: T,
}

#[derive(Epserde, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
#[epserde(zero_copy)]
struct MyStructZero {
    a: u32,
    b: u64,
}

#[allow(dead_code)]
#[derive(Epserde, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
#[epserde(zero_copy)]
enum MyEnumZeroFieldless {
    A = 4,
    B = 7,
}

#[allow(dead_code)]
#[derive(Epserde, Debug, Copy, Clone, PartialEq)]
#[repr(C, u8)]
#[epserde(zero_copy)]
enum MyEnumZeroData {
    A(u16) = 1,
    B(u32),
}

// x86_64 / aarch64 regression tests

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_primitive_types() {
    assert_eq!(
        get_type_hash::<isize>(),
        "26aa76ebace27789430c777e0c04fa3d71a968ae56d5f8f14d11822998a8f536"
    );
    assert_eq!(
        get_align_hash::<isize>(),
        "9361df19a7dba8ecbaf3672d56a618df1e16228e53c82b716272ba93f2b48b9a"
    );
    assert_eq!(
        get_type_hash::<i8>(),
        "de8149503adb288c0ce1c9c5b4e2c956edfea81c0465b9d3c0f050fccf244ccb"
    );
    assert_eq!(
        get_align_hash::<i8>(),
        "9d34149fbd1fe777eb238799054c8cbfbce372255f219f8740838def9bfd02db"
    );
    assert_eq!(
        get_type_hash::<i16>(),
        "f71059cbe6fdc1ffc100f3ef2a09091073347e532574dd35ff12741696dc94c4"
    );
    assert_eq!(
        get_align_hash::<i16>(),
        "c571327cb01ac1de6972713cbf6cc1fc3c2cab8b581ee0bc3fe6d8b56963fd5b"
    );
    assert_eq!(
        get_type_hash::<i32>(),
        "7ad565a94d26a4b30ad89d558fbc5ea3cd52dcf22ed81297d30af4633dc5dee2"
    );
    assert_eq!(
        get_align_hash::<i32>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<i64>(),
        "cf5bf2606144d2c099ee70be3e152ddb962ea5ecfdb3c57ec668f572653dca0c"
    );
    assert_eq!(
        get_align_hash::<i64>(),
        "9361df19a7dba8ecbaf3672d56a618df1e16228e53c82b716272ba93f2b48b9a"
    );
    assert_eq!(
        get_type_hash::<i128>(),
        "9af12edf72c50dea45ae226d140d82cdf7ffbaaa8c8870894ec9b2a4ba1fc8f1"
    );
    assert_eq!(
        get_align_hash::<i128>(),
        "ef4097f995a2af33eb31599bb6844f37d4c3057ff7a9fd1285ab72b4cca9405d"
    );
    assert_eq!(
        get_type_hash::<usize>(),
        "b4fa078d60195784f9d4f694dd4f30bbba2cb462272c5d327040a381c6ae77f7"
    );
    assert_eq!(
        get_align_hash::<usize>(),
        "9361df19a7dba8ecbaf3672d56a618df1e16228e53c82b716272ba93f2b48b9a"
    );
    assert_eq!(
        get_type_hash::<u8>(),
        "2ce06a9947e4470c43d93ede0e8e3bf92176e9ff7e2c04e4f073ca232427afae"
    );
    assert_eq!(
        get_align_hash::<u8>(),
        "9d34149fbd1fe777eb238799054c8cbfbce372255f219f8740838def9bfd02db"
    );
    assert_eq!(
        get_type_hash::<u16>(),
        "1bf075168a76a943c07e5122d9b278fd905ba06df82f01f5b99e7a8f034a431b"
    );
    assert_eq!(
        get_align_hash::<u16>(),
        "c571327cb01ac1de6972713cbf6cc1fc3c2cab8b581ee0bc3fe6d8b56963fd5b"
    );
    assert_eq!(
        get_type_hash::<u32>(),
        "d5bc92426537dbd470c84313cb22a506b2b20439b1d45e7cbefbbe25a38012d5"
    );
    assert_eq!(
        get_align_hash::<u32>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<u64>(),
        "7b222d4e10df9de66f232c57f8a9588098ea1e4e388dbec8925e36529ea3bca2"
    );
    assert_eq!(
        get_align_hash::<u64>(),
        "9361df19a7dba8ecbaf3672d56a618df1e16228e53c82b716272ba93f2b48b9a"
    );
    assert_eq!(
        get_type_hash::<u128>(),
        "2f9c12cb6a62e167d8276fb93d4d1e4f74a8bea9a8cf866e0594019490d64681"
    );
    assert_eq!(
        get_align_hash::<u128>(),
        "ef4097f995a2af33eb31599bb6844f37d4c3057ff7a9fd1285ab72b4cca9405d"
    );
    assert_eq!(
        get_type_hash::<f32>(),
        "ce9f74c1cf24011934b70f660063397498e14d60cb1f4526be8f09c2133bdffa"
    );
    assert_eq!(
        get_align_hash::<f32>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<f64>(),
        "83688707591704899bedbbae65a119b4efda4eba243b97bad9ab2729a5321ac2"
    );
    assert_eq!(
        get_align_hash::<f64>(),
        "9361df19a7dba8ecbaf3672d56a618df1e16228e53c82b716272ba93f2b48b9a"
    );
    assert_eq!(
        get_type_hash::<bool>(),
        "8bf37b1213caa3a008e89d8e411d3e82ba087f5ae90ebab85c9dcb7978d7be0b"
    );
    assert_eq!(
        get_align_hash::<bool>(),
        "9d34149fbd1fe777eb238799054c8cbfbce372255f219f8740838def9bfd02db"
    );
    assert_eq!(
        get_type_hash::<char>(),
        "0726080cc8cfd70487aaae75ae5bcc4796f04a824d3c4c930132edba4d572ff1"
    );
    assert_eq!(
        get_align_hash::<char>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<()>(),
        "970836783bf8d2f55e715aec918c9c10a8c20e307e4ac12303be9656f91e1f5b"
    );
    assert_eq!(
        get_align_hash::<()>(),
        "374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_option() {
    assert_eq!(
        get_type_hash::<Option<i32>>(),
        "867bfd959e18db0c7a371c241c0567f1834d567ae14726dce6049465860addd3"
    );
    assert_eq!(
        get_align_hash::<Option<i32>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_string_types() {
    assert_eq!(
        get_type_hash::<String>(),
        "ecd29ea964a8615b52c923b3063c39862579c454b581c6415dd610723f3f6231"
    );
    assert_eq!(
        get_type_hash::<str>(),
        "54256eb9ffa1d7bfcf20fbe81b78da2422fa6a1e2e9d5a738c984be53e49b70e"
    );
    assert_eq!(
        get_type_hash::<Box<str>>(),
        "297d7f9125c63e3d8291a244958acfc26980438f582f39260001ae76dc12e0f5"
    );
    assert_eq!(
        get_align_hash::<Box<str>>(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_array_types() {
    assert_eq!(
        get_type_hash::<[i32; 5]>(),
        "d22562afb0314aa707aebc5b7cbbb2d31c1cf7c74cc6487de1fd6fc646fc1c9f"
    );
    assert_eq!(
        get_align_hash::<[i32; 5]>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_slice_types() {
    assert_eq!(
        get_type_hash::<[i32]>(),
        "8b568b709c9248214234f340680707bf83fb59b69a1ee40293e9f3d6a39da15e"
    );
    assert_eq!(
        get_type_hash::<SerType<&[i32]>>(),
        "5ac428c2c5b1037df4cf42b3a09ddddfd7d32e03a49ae7d8eca91600e91658ae"
    );
    assert_eq!(
        get_align_hash::<SerType<&[i32]>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_boxed_slice_types() {
    assert_eq!(
        get_type_hash::<Box<[i32]>>(),
        "5ac428c2c5b1037df4cf42b3a09ddddfd7d32e03a49ae7d8eca91600e91658ae"
    );
    assert_eq!(
        get_align_hash::<Box<[i32]>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_tuple_types() {
    assert_eq!(
        get_type_hash::<(i32,)>(),
        "6ed3b7a66ab7d3ae9c964920365e4a67307c764995e72e286e422bd316c0e789"
    );
    assert_eq!(
        get_align_hash::<(i32,)>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<(i32, f64)>(),
        "8d4a091fe4ceeff9defabe1fb1188db2681f5da46551cfd1e7e8b4f75e318986"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_vec_types() {
    assert_eq!(
        get_type_hash::<Vec<i32>>(),
        "63048e9756b0e7c3d9797c5fad3e094365519fcbce95e2a16952cdce237d950b"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_stdlib_types() {
    use core::ops::{
        Bound, ControlFlow, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    };
    #[cfg(feature = "std")]
    use std::collections::hash_map::DefaultHasher;
    #[cfg(feature = "std")]
    assert_eq!(
        get_type_hash::<DefaultHasher>(),
        "33a2beddd67db4ca89828a9f8e28dae4730f4711eca7cf66df069b5ef735d4c6"
    );

    assert_eq!(
        get_type_hash::<Range<i32>>(),
        "7aa2d6d8c1a2469eae352b8a87965da0976c0275ff0d2b3a265af688f0e0c655"
    );
    assert_eq!(
        get_align_hash::<Range<i32>>(),
        "43e693d085b4ec151dfaec66086d1544ef78689442df813dc5dad36bc2b954ac"
    );
    assert_eq!(
        get_type_hash::<RangeFrom<i32>>(),
        "0f38a0340423a0e81ca00914c19894c3e262452848b5c452f91efdf00e11e721"
    );
    assert_eq!(
        get_align_hash::<RangeFrom<i32>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<RangeInclusive<i32>>(),
        "11a66d40ec31d4d466549957508fccff92036caa24044aa9c61a587874585164"
    );
    assert_eq!(
        get_align_hash::<RangeInclusive<i32>>(),
        "43e693d085b4ec151dfaec66086d1544ef78689442df813dc5dad36bc2b954ac"
    );
    assert_eq!(
        get_type_hash::<RangeTo<i32>>(),
        "cbe6df9099d5a8b74172cdeccddc30319971ba266a26814dec2b7f7a7dc757bb"
    );
    assert_eq!(
        get_align_hash::<RangeTo<i32>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<RangeToInclusive<i32>>(),
        "29c267f9547318b53d22ecb8073f40f010422082cf41d4f544d0bf96db6b5ba5"
    );
    assert_eq!(
        get_align_hash::<RangeToInclusive<i32>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<RangeFull>(),
        "13bf760cac28f00e41adbf477dfbf313814ffaae7667fedd16ff16b142c31b00"
    );
    assert_eq!(
        get_align_hash::<RangeFull>(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        get_type_hash::<Bound<i32>>(),
        "e93388e9ea61c43d62ec38cc72ed9766b53808e963ee0170aec5876ed87d7136"
    );
    assert_eq!(
        get_align_hash::<Bound<i32>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
    assert_eq!(
        get_type_hash::<ControlFlow<i32, f64>>(),
        "88b260dd33b97d950ca33e2abdd26373286c0e6fdeaa2df6c6130a4be267e076"
    );
    assert_eq!(
        get_align_hash::<ControlFlow<i32, f64>>(),
        "7d57949616540e133b8fd4f04f41ce0fa0703fc34c08dd42b6478ccc7553e980"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct() {
    assert_eq!(
        get_type_hash::<MyStruct>(),
        "ac58449be9408036bb7813c14df87716d8e2754a1e7a47ba8e45498cd4580104"
    );
    assert_eq!(
        get_align_hash::<MyStruct>(),
        "7d57949616540e133b8fd4f04f41ce0fa0703fc34c08dd42b6478ccc7553e980"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_generic() {
    assert_eq!(
        get_type_hash::<MyStructGeneric<i32>>(),
        "5ff9e8caaaf60611515be8a44b38845e7e545de6e781508efed8c09c92207e8c"
    );
    assert_eq!(
        get_align_hash::<MyStructGeneric<i32>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_enum() {
    assert_eq!(
        get_type_hash::<MyEnum>(),
        "095d5e7e9381cb7c331550a26c629149c759d4e5fd253158d49324cad40dac66"
    );
    assert_eq!(
        get_align_hash::<MyEnum>(),
        "71cf5f63575cbec9c621747521966f345d078b1d0c49933e9877fe4ca1f69487"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_const() {
    assert_eq!(
        get_type_hash::<MyStructConst<5>>(),
        "2e756714a0f84c2ddae547169c08d20e87c92e250747f8f3d8e4a7d912d425a1"
    );
    assert_eq!(
        get_align_hash::<MyStructConst<5>>(),
        "f7548c023e431138b11357593f5cceb9dd35eb0b0a2041f0b1560212eeb6f13e"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_mixed() {
    assert_eq!(
        get_type_hash::<MyStructMixed<i32, 5>>(),
        "a0d9d24ae04d95f63b1e104ad3b145427a1f3bd7f41959d2f38a6fce07e05f8e"
    );
    assert_eq!(
        get_align_hash::<MyStructMixed<i32, 5>>(),
        "43e693d085b4ec151dfaec66086d1544ef78689442df813dc5dad36bc2b954ac"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_const_then_type() {
    assert_eq!(
        get_type_hash::<MyStructConstThenType<5, i32>>(),
        "c2897a0165a1042788dd9b6d2f8db5aa0be720ff1c1a7b8cc501f2c5f1a3e182"
    );
    assert_eq!(
        get_align_hash::<MyStructConstThenType<5, i32>>(),
        "43e693d085b4ec151dfaec66086d1544ef78689442df813dc5dad36bc2b954ac"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_struct_zero() {
    assert_eq!(
        get_type_hash::<MyStructZero>(),
        "759e1aa6ebf3cf7f00cfeed42d1def790e2b466d1dfe666b983a084e70ac950f"
    );
    assert_eq!(
        get_align_hash::<MyStructZero>(),
        "4934f0830c8623b520b889729488db8dbe20e520dd7f6b67a95da34182c56216"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_enum_zero_fieldless() {
    assert_eq!(
        get_type_hash::<MyEnumZeroFieldless>(),
        "072582b16b825ec64f7cb925b17c13e7bb4d5f86c2ca673f3f04eece954f30c7"
    );
    assert_eq!(
        get_align_hash::<MyEnumZeroFieldless>(),
        "f415ba9c015fb3c193268fcead0954f48fab8514d7111c8230d8302ff9d4701c"
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn test_derive_enum_zero_data() {
    assert_eq!(
        get_type_hash::<MyEnumZeroData>(),
        "6a2f6c6e06c1490b8a75a44fb24a922456d7da09f10037ed79e9200f400fd9c5"
    );
    assert_eq!(
        get_align_hash::<MyEnumZeroData>(),
        "00669cb5621aadf37adc93c370e7626851c76f5d87d505d9a330388c8ba1f176"
    );
}

// i686 regression tests

#[cfg(target_arch = "x86")]
#[test]
fn test_primitive_types() {
    assert_eq!(
        get_type_hash::<isize>(),
        "26aa76ebace27789430c777e0c04fa3d71a968ae56d5f8f14d11822998a8f536"
    );
    assert_eq!(
        get_align_hash::<isize>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<i8>(),
        "de8149503adb288c0ce1c9c5b4e2c956edfea81c0465b9d3c0f050fccf244ccb"
    );
    assert_eq!(
        get_align_hash::<i8>(),
        "01acecb507abfe1a354aa8064f4af5d3f1acd019e37db3c11c97523b71c76e9d"
    );
    assert_eq!(
        get_type_hash::<i16>(),
        "f71059cbe6fdc1ffc100f3ef2a09091073347e532574dd35ff12741696dc94c4"
    );
    assert_eq!(
        get_align_hash::<i16>(),
        "2fcd151b8295e8b3bf8ec64ede173523417960a8db6cbc569de9a25a458f9135"
    );
    assert_eq!(
        get_type_hash::<i32>(),
        "7ad565a94d26a4b30ad89d558fbc5ea3cd52dcf22ed81297d30af4633dc5dee2"
    );
    assert_eq!(
        get_align_hash::<i32>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<i64>(),
        "cf5bf2606144d2c099ee70be3e152ddb962ea5ecfdb3c57ec668f572653dca0c"
    );
    assert_eq!(
        get_align_hash::<i64>(),
        "7b742c398b1a841a160d67298c4e11857acc1db71c3f90509722ca74733cb814"
    );
    assert_eq!(
        get_type_hash::<i128>(),
        "9af12edf72c50dea45ae226d140d82cdf7ffbaaa8c8870894ec9b2a4ba1fc8f1"
    );
    assert_eq!(
        get_align_hash::<i128>(),
        "48443c868f27fd5d4e1749506108b7419f6c508bd93fa3cb4f99b5745241e476"
    );
    assert_eq!(
        get_type_hash::<usize>(),
        "b4fa078d60195784f9d4f694dd4f30bbba2cb462272c5d327040a381c6ae77f7"
    );
    assert_eq!(
        get_align_hash::<usize>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<u8>(),
        "2ce06a9947e4470c43d93ede0e8e3bf92176e9ff7e2c04e4f073ca232427afae"
    );
    assert_eq!(
        get_align_hash::<u8>(),
        "01acecb507abfe1a354aa8064f4af5d3f1acd019e37db3c11c97523b71c76e9d"
    );
    assert_eq!(
        get_type_hash::<u16>(),
        "1bf075168a76a943c07e5122d9b278fd905ba06df82f01f5b99e7a8f034a431b"
    );
    assert_eq!(
        get_align_hash::<u16>(),
        "2fcd151b8295e8b3bf8ec64ede173523417960a8db6cbc569de9a25a458f9135"
    );
    assert_eq!(
        get_type_hash::<u32>(),
        "d5bc92426537dbd470c84313cb22a506b2b20439b1d45e7cbefbbe25a38012d5"
    );
    assert_eq!(
        get_align_hash::<u32>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<u64>(),
        "7b222d4e10df9de66f232c57f8a9588098ea1e4e388dbec8925e36529ea3bca2"
    );
    assert_eq!(
        get_align_hash::<u64>(),
        "7b742c398b1a841a160d67298c4e11857acc1db71c3f90509722ca74733cb814"
    );
    assert_eq!(
        get_type_hash::<u128>(),
        "2f9c12cb6a62e167d8276fb93d4d1e4f74a8bea9a8cf866e0594019490d64681"
    );
    assert_eq!(
        get_align_hash::<u128>(),
        "48443c868f27fd5d4e1749506108b7419f6c508bd93fa3cb4f99b5745241e476"
    );
    assert_eq!(
        get_type_hash::<f32>(),
        "ce9f74c1cf24011934b70f660063397498e14d60cb1f4526be8f09c2133bdffa"
    );
    assert_eq!(
        get_align_hash::<f32>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<f64>(),
        "83688707591704899bedbbae65a119b4efda4eba243b97bad9ab2729a5321ac2"
    );
    assert_eq!(
        get_align_hash::<f64>(),
        "7b742c398b1a841a160d67298c4e11857acc1db71c3f90509722ca74733cb814"
    );
    assert_eq!(
        get_type_hash::<bool>(),
        "8bf37b1213caa3a008e89d8e411d3e82ba087f5ae90ebab85c9dcb7978d7be0b"
    );
    assert_eq!(
        get_align_hash::<bool>(),
        "01acecb507abfe1a354aa8064f4af5d3f1acd019e37db3c11c97523b71c76e9d"
    );
    assert_eq!(
        get_type_hash::<char>(),
        "0726080cc8cfd70487aaae75ae5bcc4796f04a824d3c4c930132edba4d572ff1"
    );
    assert_eq!(
        get_align_hash::<char>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<()>(),
        "970836783bf8d2f55e715aec918c9c10a8c20e307e4ac12303be9656f91e1f5b"
    );
    assert_eq!(
        get_align_hash::<()>(),
        "af5570f5a1810b7af78caf4bc70a660f0df51e42baf91d4de5b2328de0e83dfc"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_option() {
    assert_eq!(
        get_type_hash::<Option<i32>>(),
        "867bfd959e18db0c7a371c241c0567f1834d567ae14726dce6049465860addd3"
    );
    assert_eq!(
        get_align_hash::<Option<i32>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_string_types() {
    assert_eq!(
        get_type_hash::<String>(),
        "ecd29ea964a8615b52c923b3063c39862579c454b581c6415dd610723f3f6231"
    );
    assert_eq!(
        get_type_hash::<str>(),
        "54256eb9ffa1d7bfcf20fbe81b78da2422fa6a1e2e9d5a738c984be53e49b70e"
    );
    assert_eq!(
        get_type_hash::<Box<str>>(),
        "297d7f9125c63e3d8291a244958acfc26980438f582f39260001ae76dc12e0f5"
    );
    assert_eq!(
        get_align_hash::<Box<str>>(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_array_types() {
    assert_eq!(
        get_type_hash::<[i32; 5]>(),
        "9f1e6a76664b5865875fa528703c78044a1dee5abc219ed90e204c12612a03b7"
    );
    assert_eq!(
        get_align_hash::<[i32; 5]>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_slice_types() {
    assert_eq!(
        get_type_hash::<[i32]>(),
        "8b568b709c9248214234f340680707bf83fb59b69a1ee40293e9f3d6a39da15e"
    );
    assert_eq!(
        get_type_hash::<SerType<&[i32]>>(),
        "5ac428c2c5b1037df4cf42b3a09ddddfd7d32e03a49ae7d8eca91600e91658ae"
    );
    assert_eq!(
        get_align_hash::<SerType<&[i32]>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_boxed_slice_types() {
    assert_eq!(
        get_type_hash::<Box<[i32]>>(),
        "5ac428c2c5b1037df4cf42b3a09ddddfd7d32e03a49ae7d8eca91600e91658ae"
    );
    assert_eq!(
        get_align_hash::<Box<[i32]>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_tuple_types() {
    assert_eq!(
        get_type_hash::<(i32,)>(),
        "6ed3b7a66ab7d3ae9c964920365e4a67307c764995e72e286e422bd316c0e789"
    );
    assert_eq!(
        get_align_hash::<(i32,)>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<(i32, f64)>(),
        "8d4a091fe4ceeff9defabe1fb1188db2681f5da46551cfd1e7e8b4f75e318986"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_vec_types() {
    assert_eq!(
        get_type_hash::<Vec<i32>>(),
        "63048e9756b0e7c3d9797c5fad3e094365519fcbce95e2a16952cdce237d950b"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_stdlib_types() {
    use core::ops::{
        Bound, ControlFlow, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    };
    #[cfg(feature = "std")]
    use std::collections::hash_map::DefaultHasher;
    #[cfg(feature = "std")]
    assert_eq!(
        get_type_hash::<DefaultHasher>(),
        "33a2beddd67db4ca89828a9f8e28dae4730f4711eca7cf66df069b5ef735d4c6"
    );

    assert_eq!(
        get_type_hash::<Range<i32>>(),
        "7aa2d6d8c1a2469eae352b8a87965da0976c0275ff0d2b3a265af688f0e0c655"
    );
    assert_eq!(
        get_align_hash::<Range<i32>>(),
        "c0558ba930d5fd0e21fec90a630e730fd8432721402b21511acab19eabb47e4a"
    );
    assert_eq!(
        get_type_hash::<RangeFrom<i32>>(),
        "0f38a0340423a0e81ca00914c19894c3e262452848b5c452f91efdf00e11e721"
    );
    assert_eq!(
        get_align_hash::<RangeFrom<i32>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<RangeInclusive<i32>>(),
        "11a66d40ec31d4d466549957508fccff92036caa24044aa9c61a587874585164"
    );
    assert_eq!(
        get_align_hash::<RangeInclusive<i32>>(),
        "c0558ba930d5fd0e21fec90a630e730fd8432721402b21511acab19eabb47e4a"
    );
    assert_eq!(
        get_type_hash::<RangeTo<i32>>(),
        "cbe6df9099d5a8b74172cdeccddc30319971ba266a26814dec2b7f7a7dc757bb"
    );
    assert_eq!(
        get_align_hash::<RangeTo<i32>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<RangeToInclusive<i32>>(),
        "29c267f9547318b53d22ecb8073f40f010422082cf41d4f544d0bf96db6b5ba5"
    );
    assert_eq!(
        get_align_hash::<RangeToInclusive<i32>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<RangeFull>(),
        "13bf760cac28f00e41adbf477dfbf313814ffaae7667fedd16ff16b142c31b00"
    );
    assert_eq!(
        get_align_hash::<RangeFull>(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        get_type_hash::<Bound<i32>>(),
        "e93388e9ea61c43d62ec38cc72ed9766b53808e963ee0170aec5876ed87d7136"
    );
    assert_eq!(
        get_align_hash::<Bound<i32>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
    assert_eq!(
        get_type_hash::<ControlFlow<i32, f64>>(),
        "88b260dd33b97d950ca33e2abdd26373286c0e6fdeaa2df6c6130a4be267e076"
    );
    assert_eq!(
        get_align_hash::<ControlFlow<i32, f64>>(),
        "16dd6c0ad3590c7b6b69e65dcd601f940f597d2508ba5059a804c67d6b04f152"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct() {
    assert_eq!(
        get_type_hash::<MyStruct>(),
        "ac58449be9408036bb7813c14df87716d8e2754a1e7a47ba8e45498cd4580104"
    );
    assert_eq!(
        get_align_hash::<MyStruct>(),
        "16dd6c0ad3590c7b6b69e65dcd601f940f597d2508ba5059a804c67d6b04f152"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_generic() {
    assert_eq!(
        get_type_hash::<MyStructGeneric<i32>>(),
        "5ff9e8caaaf60611515be8a44b38845e7e545de6e781508efed8c09c92207e8c"
    );
    assert_eq!(
        get_align_hash::<MyStructGeneric<i32>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_enum() {
    assert_eq!(
        get_type_hash::<MyEnum>(),
        "095d5e7e9381cb7c331550a26c629149c759d4e5fd253158d49324cad40dac66"
    );
    assert_eq!(
        get_align_hash::<MyEnum>(),
        "72cd150a08d2169e3bd582e39801f02d45e3654c5409d9d791754d7747cbef4f"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_const() {
    assert_eq!(
        get_type_hash::<MyStructConst<5>>(),
        "8c35a43ce5777a649c1233ad7b80f8cdaa029ee019dbe0feb2ac7088824a39a5"
    );
    assert_eq!(
        get_align_hash::<MyStructConst<5>>(),
        "1b03ab083d0fb41e44d480f48d5bba181c623c0594bda1aa8ea71a3b67dbf3b1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_mixed() {
    assert_eq!(
        get_type_hash::<MyStructMixed<i32, 5>>(),
        "276f0f7f22cd7a5d0f2192e93d26db5525af73dcadef3170078b7ea43a58c842"
    );
    assert_eq!(
        get_align_hash::<MyStructMixed<i32, 5>>(),
        "c0558ba930d5fd0e21fec90a630e730fd8432721402b21511acab19eabb47e4a"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_const_then_type() {
    assert_eq!(
        get_type_hash::<MyStructConstThenType<5, i32>>(),
        "1a41548859bde83e33edff7f106ed8102d77189930c36d76357912e19ce81c66"
    );
    assert_eq!(
        get_align_hash::<MyStructConstThenType<5, i32>>(),
        "c0558ba930d5fd0e21fec90a630e730fd8432721402b21511acab19eabb47e4a"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_struct_zero() {
    assert_eq!(
        get_type_hash::<MyStructZero>(),
        "759e1aa6ebf3cf7f00cfeed42d1def790e2b466d1dfe666b983a084e70ac950f"
    );
    assert_eq!(
        get_align_hash::<MyStructZero>(),
        "94199429c07282d4eb1283e432f54aa333c47d1f9ea52d9a558b873169a6b6a1"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_enum_zero_fieldless() {
    assert_eq!(
        get_type_hash::<MyEnumZeroFieldless>(),
        "072582b16b825ec64f7cb925b17c13e7bb4d5f86c2ca673f3f04eece954f30c7"
    );
    assert_eq!(
        get_align_hash::<MyEnumZeroFieldless>(),
        "80cc2d6c78df483d7e7ffd165363f7fee285b7488af64d4ac28af6aab0283374"
    );
}

#[cfg(target_arch = "x86")]
#[test]
fn test_derive_enum_zero_data() {
    assert_eq!(
        get_type_hash::<MyEnumZeroData>(),
        "6a2f6c6e06c1490b8a75a44fb24a922456d7da09f10037ed79e9200f400fd9c5"
    );
    assert_eq!(
        get_align_hash::<MyEnumZeroData>(),
        "350ffe5eff291d1443e7d6515e78a2a4fbaa75614f399f3288e38fb8f7fa5866"
    );
}
