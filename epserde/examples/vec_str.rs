/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
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
    let mut buf = epserde::new_aligned_cursor();
    // Serialize
    let _bytes_written = data.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    buf.set_position(0);
    let full = StringData::deserialize_full_copy(&mut buf).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<StringData>(),
    );
    println!("Value: {:x?}", full);

    println!("\n");

    // Do an ε-copy deserialization
    let buf = buf.into_inner();
    let eps = StringData::deserialize_eps_copy(&buf).unwrap();
    println!(
        " ε-copy deserialization type: {}",
        std::any::type_name::<<StringData as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
