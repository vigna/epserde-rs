/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use anyhow::Result;
use epserde::prelude::*;

#[test]
fn test_slices() -> Result<()> {
    let v = vec![0, 1, 2];
    let s = v.as_slice();
    let t = vec![s, s, s];
    let mut cursor = AlignedCursor::<Aligned16>::new();
    unsafe { t.serialize(&mut cursor) }?;
    Ok(())
}

#[test]
fn test_ref_str() -> Result<()> {
    let t = vec!["a", "b", "c"];
    let mut cursor = AlignedCursor::<Aligned16>::new();
    unsafe { t.serialize(&mut cursor) }?;
    Ok(())
}
