/*
 * SPDX-FileCopyrightText: 2026 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Example of a structure containing a reference to a slice of a zero-copy
//! type.
//!
//! Such a structure can be serialized and ε-copy deserialized, but it is
//! obviously not fully deserializable, as there is no type with an owned inner
//! field to return. The trait implementations must be written by hand, as the
//! derive code does not at this time handle lifetimes. They follow closely the
//! derive-generated code one would obtain if the inner type was `Vec<u8>`, just
//! replacing the inner type where necessary, and keeping full-copy
//! deserialization unimplemented.
//!
//! Please compile with the "schema" feature to see the schema output.

use core::hash::Hash;
use epserde::{deser::deser_eps_slice_zero, prelude::*, ser::SerType, ser::WriteWithNames};

#[derive(Debug)]
struct S<'a>(&'a [u8]);

unsafe impl CopyType for S<'_> {
    type Copy = Deep;
}

impl TypeHash for S<'_> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "DeepCopy".hash(hasher);
        "S".hash(hasher);
        "0".hash(hasher);
        <SerType<&[u8]>>::type_hash(hasher);
    }
}

impl AlignHash for S<'_> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        <SerType<&[u8]> as AlignHash>::align_hash(hasher, &mut 0);
    }
}

impl<'a> SerInner for S<'a> {
    type SerType = S<'a>;
    const IS_ZERO_COPY: bool = false;
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> ser::Result<()> {
        unsafe { WriteWithNames::write(backend, "0", &self.0) }?;
        Ok(())
    }
}

impl DeserInner for S<'_> {
    type DeserType<'b> = S<'b>;

    fn __check_covariance<'__long: '__short, '__short>(
        proof: epserde::deser::CovariantProof<Self::DeserType<'__long>>,
    ) -> epserde::deser::CovariantProof<Self::DeserType<'__short>> {
        proof
    }

    unsafe fn _deser_full_inner(_backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        // There is no type with an owned inner field to return.
        unimplemented!();
    }

    unsafe fn _deser_eps_inner<'c>(
        backend: &mut SliceWithPos<'c>,
    ) -> deser::Result<Self::DeserType<'c>> {
        unsafe { Ok(S(deser_eps_slice_zero(backend)?)) }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Serializing type: {}", core::any::type_name::<S>());
    println!(
        "Associated serialization type: {}",
        core::any::type_name::<SerType<S>>()
    );
    println!();

    let s = [0_u8, 1, 2, 3];
    let v = S(&s);

    let mut cursor = <AlignedCursor<Aligned16>>::new();

    // Serialize
    #[cfg(feature = "schema")]
    {
        let schema = unsafe { v.serialize_with_schema(&mut cursor)? };
        println!("{}", schema.to_csv_with_data(cursor.as_bytes()));
        println!();
    }
    #[cfg(not(feature = "schema"))]
    let _bytes_written = unsafe { v.serialize(&mut cursor)? };

    // Do an ε-copy deserialization, which returns the structure with the field
    // borrowing the serialized bytes. Full-copy deserialization is not possible.
    let eps = unsafe { <S>::deserialize_eps(cursor.as_bytes())? };
    println!(
        "ε-copy deserialization: returns the associated deserialization type {}",
        core::any::type_name::<DeserType<'_, S>>(),
    );
    println!("Value: {:x?}", eps);
    assert_eq!(eps.0, v.0);

    #[cfg(not(feature = "schema"))]
    println!("\nPlease compile with the \"schema\" feature to see the schema output");
    Ok(())
}
