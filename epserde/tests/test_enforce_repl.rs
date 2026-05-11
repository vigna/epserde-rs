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
