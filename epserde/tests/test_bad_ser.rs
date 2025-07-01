/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*

// This test should not compile, as the field of a zero-copy structure is not zero-copy.

#[test]
fn test_fake_zero() {
    use epserde::prelude::*;
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
        let mut cursor = <AlignedCursor<A16>>::new();
        let a = FakeZero {
            a: NewType {
                data: vec![0x89; 6],
            },
        };
        // This must panic.
        let _ = unsafe { a.serialize(&mut cursor) };
    });
    assert!(result.is_err());
}
*/
