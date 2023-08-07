/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Given a float, return it in a human readable format using SI suffixes.
pub fn humanize_float(mut x: f64) -> (f64, &'static str) {
    const UOM: &[&str] = &[
        "q", "r", "y", "z", "a", "f", "p", "n", "μ", "m", "", "K", "M", "G", "T", "P", "E", "Z",
        "Y", "R", "Q",
    ];
    let mut uom_idx = 10;
    debug_assert_eq!(UOM[uom_idx], "");

    if x.abs() > 1.0 {
        while x.abs() > 1000.0 {
            uom_idx += 1;
            x /= 1000.0;
        }
    } else {
        while x.abs() < 0.001 {
            uom_idx -= 1;
            x *= 1000.0;
        }
    }

    (x, UOM[uom_idx])
}
