/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

// Type-level #[epserde(force_full(T))] pins a type parameter to full-copy
// deserialization: it is removed from the DeserType substitution set and kept
// verbatim, while SerType keeps normalizing it.
//
// The motivating example is a parameter that the derive's syntactic walk would
// classify as ε-copy (it occurs at a variable position in an unmarked field),
// but that the enclosing field type actually holds full-copy. Here Inner holds
// T in a field-level force_full slot, so Inner<T>::DeserType<'a> = Inner<T>;
// without the marker, Outer would wrongly substitute T and fail to type-check.

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq)]
struct Inner<T> {
    #[epserde(force_full)]
    x: T,
}

#[derive(Epserde, Debug, PartialEq)]
#[epserde(force_full(T))]
struct Outer<T> {
    inner: Inner<T>,
}

#[test]
fn test_force_full_param_round_trip() -> anyhow::Result<()> {
    let original = Outer {
        inner: Inner {
            x: vec![1i32, 2, 3],
        },
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Outer<Vec<i32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // DeserType<'a> keeps T verbatim, so the ε-copy form is also Outer<Vec<i32>>.
    let eps: Outer<Vec<i32>> = unsafe { <Outer<Vec<i32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(original, eps);

    Ok(())
}

// A forced parameter T coexists with a naturally ε-copy parameter Y: only Y is
// substituted in DeserType, while T stays verbatim and Y becomes a slice.
#[derive(Epserde, Debug, PartialEq)]
#[epserde(force_full(T))]
struct Mixed<T, Y> {
    inner: Inner<T>,
    y: Y,
}

#[test]
fn test_force_full_param_mixed() -> anyhow::Result<()> {
    let original = Mixed {
        inner: Inner {
            x: vec![1i32, 2, 3],
        },
        y: vec![10u32, 20, 30],
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Mixed<Vec<i32>, Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // T stays verbatim (Vec<i32>), Y is substituted by its DeserType (&[u32]).
    let eps: Mixed<Vec<i32>, &[u32]> =
        unsafe { <Mixed<Vec<i32>, Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(original.inner, eps.inner);
    assert_eq!([10u32, 20, 30].as_slice(), eps.y);

    Ok(())
}
