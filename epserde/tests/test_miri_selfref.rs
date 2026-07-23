/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Reproduction harness for the Yoke-class self-referential soundness issues
//! adapted to MemCase:
//!
//! - unicode-org/icu4x#2095 (noalias on the owning Box)
//! - unicode-org/icu4x#3696 (strong protection on by-value pass + drop)
//!
//! Run with, e.g.:
//!   cargo +nightly miri test --test test_miri_selfref                       # Stacked Borrows
//!   MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test --test test_miri_selfref
//!
//! A backend-backed MemCase is required (the Memory/Box variant), built via
//! read_mem exactly as epserde's own tests do. A Vec<i32> eps-deserializes to a
//! &[i32] pointing into that Box, making the MemCase self-referential like a Yoke.

use epserde::prelude::*;

/// Build a backend-backed MemCase<Vec<i32>> (Memory/Box variant) from real
/// serialized bytes, so the inner &[i32] points into the owned Box. The MemCase
/// is *returned*, i.e. moved out of this frame.
fn make() -> anyhow::Result<MemCase<Vec<i32>>> {
    let v: Vec<i32> = vec![1, 2, 3, 4];
    let mut buffer = Vec::new();
    unsafe { v.serialize(&mut buffer)? };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mc = unsafe { <Vec<i32>>::read_mem(cursor, buffer.len())? };
    Ok(mc)
}

/// #2095 facet: move the MemCase out of make(), then read through the interior
/// reference. Under Stacked Borrows the move retags the owning Box, which can
/// pop the borrow-stack entry for the aliasing &[i32].
#[test]
fn test_move_then_read() -> anyhow::Result<()> {
    let mc = make()?;
    let s: &[i32] = mc.uncase();
    assert_eq!(s, &[1, 2, 3, 4]);
    assert_eq!(mc.len(), 4);
    assert_eq!(mc[0], 1);
    Ok(())
}

/// #3696 facet: pass the MemCase BY VALUE into a function, use the inner
/// reference, then let the MemCase drop inside the callee (deallocating the
/// Box). Same shape as the failing Yoke `example` in #3696.
fn consume_by_value(mc: MemCase<Vec<i32>>) -> i32 {
    let s: &[i32] = mc.uncase();
    s.iter().sum()
    // mc (and its Box backend) is dropped here.
}

#[test]
fn test_by_value_then_drop() -> anyhow::Result<()> {
    let mc = make()?;
    assert_eq!(consume_by_value(mc), 10);
    Ok(())
}

#[inline(never)]
fn frame_b(mc: MemCase<Vec<i32>>) -> i32 {
    mc.uncase()[3]
}
#[inline(never)]
fn frame_a(mc: MemCase<Vec<i32>>) -> i32 {
    frame_b(mc)
}

#[test]
fn test_by_value_nested_frames() -> anyhow::Result<()> {
    let mc = make()?;
    assert_eq!(frame_a(mc), 4);
    Ok(())
}

// ---- Controls to isolate the cause ----

/// CONTROL A: the same Vec<i32> read via plain deserialize_eps into a local
/// borrow (NOT wrapped in a MemCase). No self-reference inside an owning struct.
/// If this is clean while the MemCase paths are not, the MemCase wrapper / the
/// move is the cause, not the data or the (de)serialization itself.
#[test]
fn test_control_eps_no_memcase() -> anyhow::Result<()> {
    let v: Vec<i32> = vec![1, 2, 3, 4];
    // Use an AlignedCursor so the bytes meet ε-serde's alignment requirement
    // (a plain Vec<u8> is not guaranteed aligned, which Miri rejects).
    let mut cursor = <AlignedCursor>::new();
    unsafe { v.serialize(&mut cursor)? };
    let s: &[i32] = unsafe { <Vec<i32>>::deserialize_eps(cursor.as_bytes())? };
    assert_eq!(s, &[1, 2, 3, 4]);
    assert_eq!(s.iter().sum::<i32>(), 10);
    Ok(())
}

/// CONTROL B: build a backend-backed MemCase and read it strictly IN PLACE,
/// never moving it after construction.
#[test]
fn test_control_memcase_in_place() -> anyhow::Result<()> {
    let v: Vec<i32> = vec![1, 2, 3, 4];
    let mut buffer = Vec::new();
    unsafe { v.serialize(&mut buffer)? };
    let cursor = <AlignedCursor>::from_slice(&buffer);
    let mc = unsafe { <Vec<i32>>::read_mem(cursor, buffer.len())? };
    let s: &[i32] = mc.uncase();
    assert_eq!(s, &[1, 2, 3, 4]);
    Ok(())
}
