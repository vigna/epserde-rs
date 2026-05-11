/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy)]
struct A<T>(T);

// T does not appear as a direct field; only inside A<T>. Marking the
// field lifts A<T>'s parameter occurrence into the replaceable set
// while preserving the README's no-overlap invariant.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct B<T> {
    #[epserde(force_repl)]
    inner: A<T>,
}

#[test]
fn test_force_repl_wrapper() -> anyhow::Result<()> {
    let original: B<Vec<u32>> = B { inner: A(vec![1, 2, 3, 4]) };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <B<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <B<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner_slice: &[u32] = eps.inner.0;
    assert_eq!([1u32, 2, 3, 4].as_slice(), inner_slice);

    Ok(())
}

// T appears both as a direct field AND inside A<T>, but the wrapping
// field is marked, so T's occurrence inside A<T> contributes to
// replaceability rather than irreplaceability. No conflict; both
// slots are substituted uniformly in Self::DeserType<'a>.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Mixed<T> {
    direct: T,
    #[epserde(force_repl)]
    wrapped: A<T>,
}

#[test]
fn test_force_repl_mixed_position() -> anyhow::Result<()> {
    let original: Mixed<Vec<u32>> = Mixed {
        direct: vec![10, 20],
        wrapped: A(vec![30, 40, 50]),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Mixed<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Mixed<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let direct: &[u32] = eps.direct;
    let through_wrapper: &[u32] = eps.wrapped.0;
    assert_eq!([10u32, 20].as_slice(), direct);
    assert_eq!([30u32, 40, 50].as_slice(), through_wrapper);

    Ok(())
}

// Marked-field substitution propagates bounds onto the substituted
// form, matching today's natural-repl behaviour for T: Clone etc.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Bounded<T: Clone> {
    #[epserde(force_repl)]
    inner: A<T>,
}

#[test]
fn test_force_repl_bounded() -> anyhow::Result<()> {
    let original: Bounded<Vec<u32>> = Bounded { inner: A(vec![7, 8, 9]) };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Bounded<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Bounded<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.inner.0;
    let _cloned = inner;
    assert_eq!([7u32, 8, 9].as_slice(), inner);

    Ok(())
}

// Marker on a single-segment-param field is a no-op (the field would
// already be eps-dispatched by the natural rule). Compiles and round-
// trips identically.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Redundant<T> {
    #[epserde(force_repl)]
    inner: T,
}

#[test]
fn test_force_repl_redundant_on_natural() -> anyhow::Result<()> {
    let original: Redundant<Vec<u32>> = Redundant { inner: vec![100, 200] };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Redundant<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Redundant<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner: &[u32] = eps.inner;
    assert_eq!([100u32, 200].as_slice(), inner);

    Ok(())
}

// Marker on a parameterless field is a silent no-op: the field is
// eps-dispatched (returns its type unchanged), no parameter contribution.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy)]
struct ParameterlessMarker {
    #[epserde(force_repl)]
    inner: u32,
}

#[test]
fn test_force_repl_parameterless_field() -> anyhow::Result<()> {
    let original = ParameterlessMarker { inner: 42 };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { ParameterlessMarker::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { ParameterlessMarker::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(42, eps.inner);

    Ok(())
}

// Force-repl on an enum variant field. The marker lives on the field,
// not on the variant.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
enum E<T> {
    Empty,
    Wrapped(#[epserde(force_repl)] A<T>),
    Named {
        #[epserde(force_repl)]
        value: A<T>,
    },
}

#[test]
fn test_force_repl_enum() -> anyhow::Result<()> {
    let original: E<Vec<u32>> = E::Wrapped(A(vec![5, 6, 7]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <E<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <E<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    match eps {
        E::Wrapped(a) => {
            let inner: &[u32] = a.0;
            assert_eq!([5u32, 6, 7].as_slice(), inner);
        }
        _ => panic!("expected E::Wrapped variant"),
    }

    Ok(())
}

// force_irrepl flips a direct generic field from replaceable to
// irreplaceable AND from eps-dispatch to full-dispatch. This resolves
// the historical "T both as direct field and as a type argument" case
// by pinning T as irreplaceable from both sides.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct ForceIrreplDirect<T> {
    #[epserde(force_irrepl)]
    direct: T,
    wrapped: A<T>,
}

#[test]
fn test_force_irrepl_direct() -> anyhow::Result<()> {
    let original: ForceIrreplDirect<u32> = ForceIrreplDirect {
        direct: 7,
        wrapped: A(11),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <ForceIrreplDirect<u32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // T is irreplaceable from both fields. Self::DeserType<'a> does NOT
    // substitute T, so the eps form's `direct` field is plain T, not
    // T::DeserType<'a>. Type-annotated binding pins this.
    let eps = unsafe { <ForceIrreplDirect<u32>>::deserialize_eps(cursor.as_bytes())? };
    let _direct_check: u32 = eps.direct;
    assert_eq!(7, eps.direct);

    Ok(())
}
