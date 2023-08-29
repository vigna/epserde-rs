/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Epserde, Debug)]
#[repr(C)]
#[zero_copy]
struct Data {
    a: [usize; 100],
}

fn main() {
    // Create a vector to serialize

    let a = Data { a: [1_usize; 100] };

    // Create an aligned vector to serialize into so we can do an ε-copy
    // deserialization safely
    let len = 1000;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = a.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    let full = Data::deserialize_full_copy(&v).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<Data>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do a ε-copy deserialization (which will be a zero-copy deserialization)
    let eps = Data::deserialize_eps_copy(&v).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<Data as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
