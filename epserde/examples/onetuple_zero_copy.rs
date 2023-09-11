/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*
 * This example shows how the standard behavior of ε-serde on primitive
 * types (returning a value rather than a reference) is somewhat custom:
 * the deserialization type associated to a one-element tuple containing
 * just a `usize` is a reference.
 */
use epserde::*;

fn main() {
    // Create a new value to serialize
    let x = (0_usize,);

    // Create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // Wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = x.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = <(usize,)>::deserialize_full_copy(buf).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<(usize,)>(),
    );
    println!("Value: {:x?}", full);
    assert_eq!(x, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <(usize,)>::deserialize_eps_copy(&v).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<(usize,) as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(x, *eps);
}
