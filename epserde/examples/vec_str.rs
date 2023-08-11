/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(Serialize, Deserialize, TypeHash, Debug, PartialEq, Eq, Default, Clone)]
/// Random struct we will use to test the nested serialization and deserialization.
struct Data<A> {
    a: A,
}

type StringData = Data<Vec<String>>;

fn main() {
    // Create a new value to serialize
    let data = StringData {
        a: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
    };

    // Create an aligned vector to serialize into so we can do an ε-copy
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
    // wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = data.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    let full = StringData::deserialize_full_copy(&v).unwrap();
    println!(
        "Full-deserialization type: {}",
        std::any::type_name::<StringData>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization
    let eps = StringData::deserialize_eps_copy(&v).unwrap();
    println!(
        "ε-deserialization type: {}",
        std::any::type_name::<<StringData as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
