/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

// Option<T>'s DeserType<'a> is Option<T::DeserType<'a>> (structural
// substitution). Under the default classification rule, T is a variable
// position inside Option<T>, so the derive automatically substitutes it.

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct OptionWrapper<T> {
    inner: Option<T>,
}

#[test]
fn test_option_wrapper_some() -> anyhow::Result<()> {
    let original: OptionWrapper<Vec<u32>> = OptionWrapper {
        inner: Some(vec![1, 2, 3]),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <OptionWrapper<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <OptionWrapper<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    match eps.inner {
        Some(inner) => {
            let slice: &[u32] = inner;
            assert_eq!([1u32, 2, 3].as_slice(), slice);
        }
        None => panic!("expected Some"),
    }

    Ok(())
}

#[test]
fn test_option_wrapper_none() -> anyhow::Result<()> {
    let original: OptionWrapper<Vec<u32>> = OptionWrapper { inner: None };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <OptionWrapper<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <OptionWrapper<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    assert!(eps.inner.is_none());

    Ok(())
}
