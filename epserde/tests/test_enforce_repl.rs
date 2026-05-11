/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

// A naturally-replaceable wrapper: T appears as a direct field, so its
// `DeserType<'a>` substitutes T transitively for any T.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct A<T>(T);

// T does *not* appear as a direct field, only inside A<T>. Without
// `enforce_repl(T)`, T would be non-replaceable in B and the ε-copy
// deserialized form would keep T as-is.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(enforce_repl(T))]
struct B<T>(A<T>);

#[test]
fn test_enforce_repl_wrapper() -> anyhow::Result<()> {
    let original: B<Vec<u32>> = B(A(vec![1, 2, 3, 4]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    // Full-copy round-trip.
    cursor.set_position(0);
    let full = unsafe { <B<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // ε-copy round-trip: inner Vec<u32> must come back as &[u32].
    let eps = unsafe { <B<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner_slice: &[u32] = eps.0.0;
    assert_eq!([1u32, 2, 3, 4].as_slice(), inner_slice);

    Ok(())
}

// T appears both as a direct field *and* through a wrapper. Without
// `enforce_repl(T)` this is rejected because the generated code's
// `DeserType<'_>` would have inconsistent slots. With `enforce_repl(T)`
// both slots are substituted uniformly.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(enforce_repl(T))]
struct Mixed<T>(T, A<T>);

#[test]
fn test_enforce_repl_mixed_position() -> anyhow::Result<()> {
    let original: Mixed<Vec<u32>> = Mixed(vec![10, 20], A(vec![30, 40, 50]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Mixed<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Mixed<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let direct: &[u32] = eps.0;
    let through_a: &[u32] = eps.1.0;
    assert_eq!([10u32, 20].as_slice(), direct);
    assert_eq!([30u32, 40, 50].as_slice(), through_a);

    Ok(())
}

// `enforce_repl` on a parameter with trait bounds must propagate those
// bounds onto the substituted form (`DeserType<'_, T>: Clone`).
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(enforce_repl(T))]
struct Bounded<T: Clone>(A<T>);

#[test]
fn test_enforce_repl_bounded() -> anyhow::Result<()> {
    let original: Bounded<Vec<u32>> = Bounded(A(vec![7, 8, 9]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Bounded<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Bounded<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.0.0;
    // Exercise the propagated Clone bound on DeserType<'_, T>.
    let _cloned = inner;
    assert_eq!([7u32, 8, 9].as_slice(), inner);

    Ok(())
}

// `enforce_repl(T)` on a parameter that is already naturally replaceable
// is a no-op: the derived code must behave identically to A<T> above.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(enforce_repl(T))]
struct Redundant<T>(T);

#[test]
fn test_enforce_repl_redundant() -> anyhow::Result<()> {
    let original: Redundant<Vec<u32>> = Redundant(vec![100, 200]);
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Redundant<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Redundant<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.0;
    assert_eq!([100u32, 200].as_slice(), inner);

    Ok(())
}

// Forced replaceability works on enum parameters across all variant
// shapes (unit, unnamed, named).
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(enforce_repl(T))]
enum E<T> {
    Empty,
    Single(A<T>),
    Named { value: A<T> },
}

#[test]
fn test_enforce_repl_enum() -> anyhow::Result<()> {
    let original: E<Vec<u32>> = E::Single(A(vec![5, 6, 7]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <E<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <E<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    match eps {
        E::Single(a) => {
            let inner: &[u32] = a.0;
            assert_eq!([5u32, 6, 7].as_slice(), inner);
        }
        _ => panic!("expected E::Single variant"),
    }

    Ok(())
}
