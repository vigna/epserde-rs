//! Serialization traits and types
use crate::*;
use core::hash::Hasher;

pub mod ser_impl;
pub use ser_impl::*;

pub mod ser_writers;
pub use ser_writers::*;

/// Inner trait to implement serialization of a type.
/// The user should not implement this trait directly, but rather derive it.
pub trait SerializeInner: Sized {
    /// Inner constant used to keep track recursivelly if we can optimize the
    /// serialization of the type; i.e., if we can serialize the type without
    /// recursivelly calling the serialization of the inner types.
    ///
    /// This is used to optimize the serialization of arrays, tuples, etc.
    const WRITE_ALL_OPTIMIZATION: bool;

    /// The deserialization type. E.g. Vec<T> should serialize as &[T].
    type SerType<'a>: TypeName;

    /// Serialize the type using the given backend.
    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F>;
}

/// User-facing trait.
/// The user should implement this trait directly but rather derive it.
pub trait Serialize: SerializeInner + Sized {
    /// Serialize the type using the given backend.
    fn serialize<F: WriteNoStd>(&self, backend: F) -> Result<usize> {
        Ok(self.serialize_on(WriteWithPos::new(backend))?.get_pos())
    }

    /// Serialize the type using the given backend and return the schema.
    fn serialize_with_schema<F: WriteNoStd>(&self, backend: F) -> Result<Schema> {
        Ok(self
            .serialize_on(SchemaWriter::new(WriteWithPos::new(backend)))?
            .schema)
    }

    /// Serialize the type using the given backend, this has a stricter
    /// requirement than `serialize` and is generally not needed by the user.
    fn serialize_on<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        backend = backend.add_field("MAGIC", &MAGIC)?;
        backend = backend.add_field("VERSION_MAJOR", &VERSION.0)?;
        backend = backend.add_field("VERSION_MINOR", &VERSION.1)?;

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        Self::SerType::type_hash(&mut hasher);
        backend = backend.add_field("TYPE_HASH", &hasher.finish())?;
        backend = backend.add_field("TYPE_NAME", &Self::SerType::type_name())?;

        backend = backend.add_field("ROOT", self)?;
        backend.flush()?;
        Ok(backend)
    }
}
/// blanket impl so the user canno overwrite serialize and pad_align_to
impl<T: SerializeInner> Serialize for T {}
