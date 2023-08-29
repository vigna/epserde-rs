/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Serialization traits and types
//!
//! [`Serialize`] is the main serialization trait, providing a
//! [`Serialize::serialize`] method that serializes the type into a
//! generic [`WriteNoStd`] backend. The implementation of this trait
//! is based on [`SerializeInner`], which is automatically derived
//! with `#[derive(Serialize)]`.
use crate::*;
use core::hash::Hasher;

pub mod ser_impl;
pub use ser_impl::*;

pub mod ser_writers;
pub use ser_writers::*;

/// Inner trait to implement serialization of a type. This trait exists
/// to separate the user-facing [`Serialize`] trait from the low-level
/// serialization mechanism of [`SerializeInner::_serialize_inner`]. Moreover,
/// it makes it possible to behave slighly differently at the top
/// of the recursion tree (e.g., to write the endianness marker), and to prevent
/// the user from modifying the methods in [`Serialize`].
///
/// The user should not implement this trait directly, but rather derive it.
pub trait SerializeInner: TypeHash + Sized {
    /// Inner constant used to keep track recursively if we can optimize the
    /// serialization of the type; i.e., if we can serialize the type without
    /// recursively calling the serialization of the inner types.
    ///
    /// This is used to optimize the serialization of arrays, tuples, etc.
    const IS_ZERO_COPY: bool;

    /// Inner constant that keeps track of whether a type has been not declared
    /// full copy, has not been declared zero copy, but nonetheless all its
    /// fields are zero copy.
    const ZERO_COPY_MISMATCH: bool;

    /// Serialize this structure using the given backend.
    fn _serialize_inner<F: FieldWrite>(&self, backend: F) -> Result<F>;
}

/// Main serialization trait. It is separated from [`SerializeInner`] to
/// avoid that the user modify its behavior, and hide internal serialization
/// methods.
pub trait Serialize: SerializeInner {
    /// Serialize the type using the given backend.
    fn serialize<F: WriteNoStd>(&self, backend: F) -> Result<usize> {
        Ok(self
            .serialize_on_field_write(WriteWithPos::new(backend))?
            .get_pos())
    }

    /// Serialize the type using the given backend and return the schema.
    /// This method is mainly useful for debugging and cross-language
    /// interoperability.
    fn serialize_with_schema<F: WriteNoStd>(&self, backend: F) -> Result<Schema> {
        Ok(self
            .serialize_on_field_write(SchemaWriter::new(WriteWithPos::new(backend)))?
            .schema)
    }

    /// Serialize the type using the given [`FieldWrite`].
    fn serialize_on_field_write<F: FieldWrite>(&self, mut backend: F) -> Result<F> {
        backend = backend.add_field("MAGIC", &MAGIC)?;
        backend = backend.add_field("VERSION_MAJOR", &VERSION.0)?;
        backend = backend.add_field("VERSION_MINOR", &VERSION.1)?;
        backend = backend.add_field("USIZE_SIZE", &(core::mem::size_of::<usize>() as u16))?;

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        Self::type_hash(&mut hasher);
        backend = backend.add_field("TYPE_HASH", &hasher.finish())?;
        backend = backend.add_field("TYPE_NAME", &core::any::type_name::<Self>().to_string())?;

        backend = backend.add_field("ROOT", self)?;
        backend.flush()?;
        Ok(backend)
    }
}

/// Blanket implementation that prevents the user from overwriting the
/// methods in [`Serialize`].
impl<T: SerializeInner> Serialize for T {}
