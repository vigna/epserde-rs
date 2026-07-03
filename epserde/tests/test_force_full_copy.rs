/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Tests for the `#[epserde(force_full_copy)]` field marker.
//!
//! The marker pins a field to full-copy deserialization and keeps its
//! type verbatim in the deserialization type. Unmarked fields are
//! ε-copy deserialized and their type parameters at variable positions are
//! substituted by the corresponding `DeserType<'_>`.

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(deep_copy)]
struct A<T>(T);

// A user-defined wrapper whose only occurrence of T is inside force_full_copy.
// T does not appear at any variable position outside the marked field,
// so T is not in the ε-copy set, and inner keeps its verbatim
// slot in DeserType<'_>.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct PinnedWrapper<T> {
    #[epserde(force_full_copy)]
    inner: A<T>,
}

#[test]
fn test_force_full_copy_pins_wrapper() -> anyhow::Result<()> {
    let original: PinnedWrapper<Vec<u32>> = PinnedWrapper {
        inner: A(vec![1, 2, 3, 4]),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <PinnedWrapper<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // The eps form is verbatim: inner stays A<Vec<u32>>, not A<&[u32]>.
    let eps = unsafe { <PinnedWrapper<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let _check: &PinnedWrapper<Vec<u32>> = &eps;
    assert_eq!(original, eps);

    Ok(())
}

// force_full_copy on a Vec<T> field at T zero-copy: Vec is not delta-stable at the
// zero-copy kind, so without force_full_copy the derive would emit code that fails
// to type-check (Vec<T> ε-copy deserialises to &[T], not to
// Vec<T::DeserType<'_>>). The marker pins the field full-copy.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct InternalZeroCopy<T: ZeroCopy> {
    #[epserde(force_full_copy)]
    data: Vec<T>,
}

#[test]
fn test_force_full_copy_internal_zero_copy() -> anyhow::Result<()> {
    let original: InternalZeroCopy<i32> = InternalZeroCopy {
        data: vec![10, 20, 30],
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <InternalZeroCopy<i32>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // Vec<i32> stays Vec<i32> in the eps form, not &[i32].
    let eps = unsafe { <InternalZeroCopy<i32>>::deserialize_eps(cursor.as_bytes())? };
    let _check: &InternalZeroCopy<i32> = &eps;
    assert_eq!(original, eps);

    Ok(())
}

// force_full_copy on an enum variant field. The marker lives on the field,
// not on the variant.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
enum E<T> {
    Empty,
    Wrapped(#[epserde(force_full_copy)] A<T>),
    Named {
        #[epserde(force_full_copy)]
        value: A<T>,
    },
}

#[test]
fn test_force_full_copy_enum() -> anyhow::Result<()> {
    let original: E<Vec<u32>> = E::Wrapped(A(vec![5, 6, 7]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <E<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    // The Wrapped variant's A<Vec<u32>> field stays verbatim.
    let eps = unsafe { <E<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    match eps {
        E::Wrapped(a) => assert_eq!(vec![5u32, 6, 7], a.0),
        _ => panic!("expected E::Wrapped variant"),
    }

    Ok(())
}

// Default behaviour with no marker: T as a variable position inside
// A<T> is automatically replaced in DeserType<'_>, the field is
// ε-dispatched, and the inner Vec<u32> becomes &[u32].
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct DefaultWrapper<T> {
    inner: A<T>,
}

#[test]
fn test_default_substitution_through_wrapper() -> anyhow::Result<()> {
    let original: DefaultWrapper<Vec<u32>> = DefaultWrapper {
        inner: A(vec![1, 2, 3, 4]),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <DefaultWrapper<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <DefaultWrapper<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let inner_slice: &[u32] = eps.inner.0;
    assert_eq!([1u32, 2, 3, 4].as_slice(), inner_slice);

    Ok(())
}

// Two parameters: one (P) ε-copy via a variable position in an
// unmarked field, the other (Q) pinned through force_full_copy. DeserType<'_>
// substitutes P but keeps Q verbatim; the marked field's slot is
// A<Q>, not A<Q::DeserType<'_>>, and the test would not compile if
// Q ended up in the ε-copy set.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Split<P, Q> {
    a: P,
    #[epserde(force_full_copy)]
    b: A<Q>,
}

#[test]
fn test_force_full_copy_mixed_eps_copy_and_pinned() -> anyhow::Result<()> {
    let original: Split<Vec<u32>, Vec<u32>> = Split {
        a: vec![1, 2, 3],
        b: A(vec![4, 5, 6]),
    };
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <Split<Vec<u32>, Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let eps = unsafe { <Split<Vec<u32>, Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    let _check: &Split<&[u32], Vec<u32>> = &eps;
    let a_slice: &[u32] = eps.a;
    assert_eq!([1u32, 2, 3].as_slice(), a_slice);
    assert_eq!(vec![4u32, 5, 6], eps.b.0);

    Ok(())
}

// Default behaviour with T appearing both directly and inside a
// wrapper. Both occurrences are variable positions, both are
// substituted, both slots end up as <T as DeserInner>::DeserType<'_>.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Mixed<T> {
    direct: T,
    wrapped: A<T>,
}

#[test]
fn test_default_mixed_position() -> anyhow::Result<()> {
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
