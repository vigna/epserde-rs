/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Serialization traits and types.

[`Serialize`] is the main serialization trait, providing a
[`Serialize::serialize`] method that serializes the type into a
generic [`WriteNoStd`] backend. The implementation of this trait
is based on [`SerializeInner`], which is automatically derived
with `#[derive(Serialize)]`.

*/

use crate::traits::*;
use crate::*;

use core::hash::Hasher;
use std::{io::BufWriter, path::Path};

pub mod writers;
pub use writers::*;

pub type Result<T> = core::result::Result<T, Error>;

/// Main serialization trait. It is separated from [`SerializeInner`] to
/// avoid that the user modify its behavior, and hide internal serialization
/// methods.
///
/// It provides a convenience method [`Serialize::store`] that serializes
/// the type to a file.
pub trait Serialize {
    /// Serialize the type using the given backend.
    fn serialize(&self, backend: &mut impl WriteNoStd) -> Result<usize> {
        let mut write_with_pos = WriteWithPos::new(backend);
        self.serialize_on_field_write(&mut write_with_pos)?;
        Ok(write_with_pos.pos())
    }

    /// Serialize the type using the given backend and return the schema.
    /// This method is mainly useful for debugging and cross-language
    /// interoperability.
    fn serialize_with_schema(&self, backend: &mut impl WriteNoStd) -> Result<Schema> {
        let mut schema_writer = SchemaWriter::new(WriteWithPos::new(backend));
        self.serialize_on_field_write(&mut schema_writer)?;
        let mut schema = schema_writer.schema;
        // sort the schema before returning it because 99% of the times the user
        // will want it sorted, and it won't take too much time.
        // If the user doesn't want it sorted, they can just call
        // ```rust
        //  let mut schema = self
        // .serialize_on_field_write(SchemaWriter::new(WriteWithPos::new(backend)))?
        // .schema;
        // ```
        schema.sort();
        Ok(schema)
    }

    /// Serialize the type using the given [`FieldWrite`].
    fn serialize_on_field_write(&self, backend: &mut impl FieldWrite) -> Result<()>;

    /// Commodity method to serialize to a file.
    fn store(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = std::fs::File::create(path).map_err(Error::FileOpenError)?;
        let mut buf_writer = BufWriter::new(file);
        self.serialize(&mut buf_writer)?;
        Ok(())
    }
}

/// Blanket implementation that prevents the user from overwriting the
/// methods in [`Serialize`].
///
/// This implementation [writes a header](`write_header`) containing some hashes
/// and debug information.
impl<T: SerializeInner> Serialize for T {
    /// Serialize the type using the given [`FieldWrite`].
    fn serialize_on_field_write(&self, backend: &mut impl FieldWrite) -> Result<()> {
        write_header::<Self>(backend)?;
        backend.write_field("ROOT", self)?;
        backend.flush()
    }
}

/// Inner trait to implement serialization of a type. This trait exists
/// to separate the user-facing [`Serialize`] trait from the low-level
/// serialization mechanism of [`SerializeInner::_serialize_inner`]. Moreover,
/// it makes it possible to behave slighly differently at the top
/// of the recursion tree (e.g., to write the endianness marker), and to prevent
/// the user from modifying the methods in [`Serialize`].
///
/// The user should not implement this trait directly, but rather derive it.
pub trait SerializeInner: CopyType + TypeHash + Sized {
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
    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> Result<()>;
}

/// Common code for both full-copy and zero-copy deserialization.
/// Must be kept in sync with [`crate::des::check_header`].
pub fn write_header<T: TypeHash>(backend: &mut impl FieldWrite) -> Result<()> {
    backend.write_field("MAGIC", &MAGIC)?;
    backend.write_field("VERSION_MAJOR", &VERSION.0)?;
    backend.write_field("VERSION_MINOR", &VERSION.1)?;
    backend.write_field("USIZE_SIZE", &(core::mem::size_of::<usize>() as u8))?;

    let mut type_hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut repr_hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut offset_of = 0;

    T::type_hash(&mut type_hasher, &mut repr_hasher, &mut offset_of);
    backend.write_field("TYPE_HASH", &type_hasher.finish())?;
    backend.write_field("TYPE_REPR_HASH", &repr_hasher.finish())?;
    backend.write_field("TYPE_NAME", &core::any::type_name::<T>().to_string())
}

/// A helper trait that makes it possible to implement differently
/// serialization for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types.
/// See [`crate::traits::CopyType`] for more information.
pub trait SerializeHelper<T: CopySelector> {
    fn _serialize_inner(&self, backend: &mut impl FieldWrite) -> Result<()>;
}

#[derive(Debug)]
/// Errors that can happen during serialization.
pub enum Error {
    /// The underlying writer returned an error.
    WriteError,
    /// [`Serialize::store`] could not open the provided file.
    FileOpenError(std::io::Error),
}

impl std::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::WriteError => write!(f, "Write error during ε-serde serialization"),
            Self::FileOpenError(error) => {
                write!(f, "Write error during ε-serde serialization: {}", error)
            }
        }
    }
}

/// [`std::io::Write`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly
/// this trait as we provide a blanket
/// implementation that implements [`WriteNoStd`] for all types that implement
/// [`std::io::Write`]. In particular, in such a context you can use [`std::io::Cursor`]
/// for in-memory serialization.
pub trait WriteNoStd {
    /// Write some bytes.
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;

    /// Flush all changes to the underlying storage if applicable.
    fn flush(&mut self) -> Result<()>;
}

#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
impl<W: Write> WriteNoStd for W {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        Write::write_all(self, buf).map_err(|_| Error::WriteError)
    }
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self).map_err(|_| Error::WriteError)
    }
}

/// A little wrapper around a writer that keeps track of the current position
/// so we can align the data.
///
/// This is needed because the [`Write`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
pub struct WriteWithPos<'a, F: WriteNoStd> {
    /// What we actually write on
    backend: &'a mut F,
    /// How many bytes we have written from the start
    pos: usize,
}

impl<'a, F: WriteNoStd> WriteWithPos<'a, F> {
    #[inline(always)]
    /// Create a new [`WriteWithPos`] on top of a generic writer `F`
    pub fn new(backend: &'a mut F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<'a, F: WriteNoStd> FieldWrite for WriteWithPos<'a, F> {
    #[inline(always)]
    fn pos(&self) -> usize {
        self.pos
    }
}

impl<'a, F: WriteNoStd> WriteNoStd for WriteWithPos<'a, F> {
    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.backend.write_all(buf)?;
        self.pos += buf.len();
        Ok(())
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }
}
