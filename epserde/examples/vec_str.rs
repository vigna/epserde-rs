/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/// Example showing that ε-copy deserialization can be used with
/// a `Vec<String>`, giving back a `Vec<&str>`.
use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    a: A,
}

type StringData = Data<Vec<String>>;

fn main() {
    let data = StringData {
        a: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
    };
    let mut aligned_buf = <Vec<u128>>::with_capacity(1024);
    let mut cursor = std::io::Cursor::new(bytemuck::cast_slice_mut(aligned_buf.as_mut_slice()));

    // Serialize
    let _bytes_written = data.serialize(&mut cursor).unwrap();

    // Do a full-copy deserialization
    cursor.set_position(0);
    let full = StringData::deserialize_full(&mut cursor).unwrap();
    println!(
        "Full-copy deserialization type: {}",
        std::any::type_name::<StringData>(),
    );
    println!("Value: {:x?}", full);

    println!();

    // Do an ε-copy deserialization
    let buf = cursor.into_inner();
    let eps = StringData::deserialize_eps(&buf).unwrap();
    println!(
        "ε-copy deserialization type: {}",
        std::any::type_name::<<StringData as DeserializeInner>::DeserType<'_>>(),
    );
    println!("Value: {:x?}", eps);
}
