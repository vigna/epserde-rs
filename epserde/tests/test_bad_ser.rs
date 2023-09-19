/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![cfg(test)]

use epserde::prelude::*;

#[test]
fn test_fake_zero() {
    #[derive(Epserde)]
    struct NewType {
        data: Vec<usize>,
    }

    impl MaxSizeOf for NewType {
        fn max_size_of() -> usize {
            0
        }
    }
    #[derive(Epserde)]
    #[zero_copy]
    #[repr(C)]
    struct FakeZero {
        a: NewType,
    }

    let result = std::panic::catch_unwind(|| {
        let mut cursor = epserde::new_aligned_cursor();
        let a = FakeZero {
            a: NewType {
                data: vec![0x89; 6],
            },
        };
        // This must panic.
        let _ = a.serialize(&mut cursor);
    });
    assert!(result.is_err());
}
