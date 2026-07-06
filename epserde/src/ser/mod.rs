/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Serialization traits and types.
//!
//! [`Serialize`] is the main serialization trait, providing a
//! [`Serialize::serialize`] method that serializes the type into a generic
//! [`WriteNoStd`] backend, and a [`Serialize::serialize_with_schema`] method
//! that additionally returns a [`Schema`] describing the data that has been
//! written. The implementation of this trait is based on [`SerInner`],
//! which is automatically derived with `#[derive(Epserde)]`.

use crate::traits::*;
use crate::*;

use core::hash::Hasher;

pub mod write_with_names;
pub use write_with_names::*;
pub mod helpers;
pub use helpers::*;
pub mod write;
pub use write::*;

#[cfg(feature = "std")]
use std::{io::BufWriter, path::Path};

/// The result type for serialization, using the serialization [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// A shorthand for [`<T as SerInner>::SerType`].
///
/// [`<T as SerInner>::SerType`]: SerInner::SerType
pub type SerType<T> = <T as SerInner>::SerType;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// Errors that can happen during serialization.
pub enum Error {
    /// [`Serialize::store`] could not open the provided file.
    #[cfg(feature = "std")]
    #[error("Error opening file during ε-serde serialization: {0}")]
    FileOpenError(#[source] std::io::Error),
    /// The underlying [`std::io::Write`] returned an I/O error.
    #[cfg(feature = "std")]
    #[error("I/O error during ε-serde serialization: {0}")]
    IoError(#[source] std::io::Error),
    /// The underlying writer could not complete a write (e.g., an
    /// [`AlignedCursor`] reached the `usize::MAX` length limit in a `no_std`
    /// build).
    ///
    /// When the `std` feature is enabled, writers report their failures
    /// through [`IoError`] instead (in the example above, as
    /// an [`InvalidInput`] error).
    ///
    /// [`AlignedCursor`]: crate::utils::AlignedCursor
    /// [`IoError`]: https://docs.rs/epserde/latest/epserde/ser/enum.Error.html#variant.IoError
    /// [`InvalidInput`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.InvalidInput
    #[error("Write error during ε-serde serialization")]
    WriteError,
    /// The declared length of an iterator did not match
    /// the actual length.
    ///
    /// Note that when an iterator yields more items than its declared length,
    /// serialization stops without writing the excess items (one of which is
    /// consumed to detect the mismatch), so `actual` is just a lower bound
    /// (the declared length plus one).
    #[error(
        "Iterator length mismatch during ε-serde serialization: expected {expected} items, got {actual}"
    )]
    IteratorLengthMismatch { actual: usize, expected: usize },
    /// An exhausted [`RangeInclusive`] cannot be serialized, as
    /// deserialization cannot reconstruct it.
    ///
    /// [`RangeInclusive`]: core::ops::RangeInclusive
    #[error(
        "Cannot serialize an exhausted RangeInclusive, as deserialization cannot reconstruct it"
    )]
    ExhaustedRange,
}

/// Main serialization trait. It is separated from [`SerInner`] to avoid
/// that the user modify its behavior, and hide internal serialization methods.
///
/// It provides a convenience method [`Serialize::store`] that serializes the
/// type to a file.
///
/// # Safety
///
/// All serialization methods are unsafe as they write padding bytes.
/// Serializing such a type and then accessing the padding bytes of the output
/// will lead to undefined behavior as padding bytes are uninitialized.
///
/// If you are concerned about this issue, you must organize your structures so
/// that they do not contain any padding (e.g., by creating explicit padding
/// bytes). Traits like [`FromBytes`] can provide this guarantee.
///
/// For example, this code reads a portion of the stack:
///
/// ```no_run
/// use epserde::{ser::Serialize, Epserde};
///
/// #[derive(Clone,Copy,Epserde)]
/// #[repr(C)]
/// #[repr(align(1024))]
/// #[epserde(zero_copy)]
/// struct Example(u8);
///
/// let value = [Example(0), Example(1)];
///
/// let mut bytes = vec![];
/// unsafe { value.serialize(&mut bytes).unwrap(); }
///
/// for chunk in bytes.chunks(8) {
///     println!("{:016x}", u64::from_ne_bytes(chunk.try_into().unwrap()));
/// }
/// ```
///
/// [`FromBytes`]: https://docs.rs/zerocopy/latest/zerocopy/trait.FromBytes.html
/// [`Serialize::store`]: https://docs.rs/epserde/latest/epserde/ser/trait.Serialize.html#method.store
pub trait Serialize {
    /// Serializes the type using the given backend.
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [trait documentation]: Serialize
    unsafe fn serialize(&self, backend: &mut impl WriteNoStd) -> Result<usize> {
        let mut write_with_pos = WriterWithPos::new(backend);
        unsafe { self.ser_on_field_write(&mut write_with_pos) }?;
        Ok(write_with_pos.pos())
    }

    /// Serializes the type using the given backend and return a [schema]
    /// describing the data that has been written.
    ///
    /// This method is mainly useful for debugging and to check cross-language
    /// interoperability.
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [schema]: Schema
    /// [trait documentation]: Serialize
    unsafe fn serialize_with_schema(&self, backend: &mut impl WriteNoStd) -> Result<Schema> {
        let mut writer_with_pos = WriterWithPos::new(backend);
        let mut schema_writer = SchemaWriter::new(&mut writer_with_pos);
        unsafe { self.ser_on_field_write(&mut schema_writer) }?;
        Ok(schema_writer.schema)
    }

    /// Serializes the type using the given [`WriteWithNames`].
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [trait documentation]: Serialize
    unsafe fn ser_on_field_write(&self, backend: &mut impl WriteWithNames) -> Result<()>;

    /// Convenience method to serialize to a file.
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [trait documentation]: Serialize
    #[cfg(feature = "std")]
    unsafe fn store(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = std::fs::File::create(path).map_err(Error::FileOpenError)?;
        let mut buf_writer = BufWriter::new(file);
        unsafe { self.serialize(&mut buf_writer)? };
        Ok(())
    }
}

/// Inner trait to implement serialization of a type. This trait exists
/// to separate the user-facing [`Serialize`] trait from the low-level
/// serialization mechanism of [`SerInner::_ser_inner`]. Moreover,
/// it makes it possible to behave slightly differently at the top
/// of the recursion tree (e.g., to write the endianness marker).
///
/// The user should not implement this trait directly, but rather derive it.
pub trait SerInner {
    /// This is the type whose [`TypeHash`] will be written in the header of the
    /// file. It is defined so that this type hash only depends on the
    /// serialization type.
    type SerType;
    /// Inner constant used by the derive macros to keep track recursively of
    /// whether the type satisfies the conditions for being zero-copy. It is
    /// checked at runtime against the trait implemented by the type, and if a
    /// [`ZeroCopy`] type has this constant set to `false` serialization will
    /// panic. It is also used by the derive macros to decide whether to suggest
    /// declaring a struct as zero-copy.
    const IS_ZERO_COPY: bool;

    /// Serializes this structure using the given backend.
    ///
    /// # Safety
    ///
    /// See the documentation of [`Serialize`].
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> Result<()>;
}

/// Blanket implementation that prevents the user from overwriting the
/// methods in [`Serialize`].
///
/// This implementation [writes a header] containing a magic cookie, some hashes
/// and debug information and then delegates to [WriteWithNames::write].
///
/// # Implementation Notes
///
/// Note the bound on the [`SerType`] of `T`: we need to be able to compute type
/// and alignment hashes for it. We could bound it in the definition of
/// [`SerInner`], but having the bound here instead gives us more flexibility
/// and makes the implementation of [`Owned`] easier.
///
/// [writes a header]: write_header
/// [`SerType`]: SerInner::SerType
/// [`Owned`]: crate::deser::Owned
impl<T: SerInner<SerType: TypeHash + AlignHash>> Serialize for T {
    unsafe fn ser_on_field_write(&self, backend: &mut impl WriteWithNames) -> Result<()> {
        write_header::<SerType<Self>>(backend)?;
        unsafe { backend.write("ROOT", self) }?;
        backend.flush()
    }
}

/// Writes the header.
///
/// Note that `S` must be the [`SerType`] associated with the serializing type,
/// not the serializing type itself: callers pass [`SerType<Self>`] (see
/// [`Serialize::ser_on_field_write`]), so the header hashes are computed on the
/// serialization type.
///
/// Must be kept in sync with [`crate::deser::check_header`].
///
/// [`SerType<Self>`]: SerType
pub fn write_header<S: TypeHash + AlignHash>(backend: &mut impl WriteWithNames) -> Result<()> {
    // SAFETY (for all the unsafe blocks in this function): the header
    // contains only primitive values and a str, whose memory representation
    // has no padding, so the uninitialized-bytes hazard that makes write
    // unsafe cannot arise; this is why this function can be safe.
    unsafe { backend.write("MAGIC", &MAGIC) }?;
    unsafe { backend.write("VERSION_MAJOR", &VERSION.0) }?;
    unsafe { backend.write("VERSION_MINOR", &VERSION.1) }?;
    unsafe { backend.write("USIZE_SIZE", &(core::mem::size_of::<usize>() as u8)) }?;

    let mut type_hasher = xxhash_rust::xxh3::Xxh3::new();
    S::type_hash(&mut type_hasher);

    let mut align_hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut offset_of = 0;
    S::align_hash(&mut align_hasher, &mut offset_of);

    unsafe { backend.write("TYPE_HASH", &type_hasher.finish()) }?;
    unsafe { backend.write("ALIGN_HASH", &align_hasher.finish()) }?;
    unsafe { backend.write("TYPE_NAME", &core::any::type_name::<S>()) }
}

/// A helper trait that makes it possible to implement differently serialization
/// for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types. See
/// [`crate::traits::CopyType`] for more information.
pub trait SerHelper<T: CopySelector> {
    /// # Safety
    ///
    /// See the documentation of [`Serialize`].
    unsafe fn _ser_inner(&self, backend: &mut impl WriteWithNames) -> Result<()>;
}
