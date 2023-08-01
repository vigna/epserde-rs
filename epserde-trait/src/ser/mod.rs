use crate::*;

pub mod ser_impl;
pub use ser_impl::*;

pub mod ser_writers;
pub use ser_writers::*;

pub trait SerializeInner: TypeName + Sized {
    /// Inner constant used to keep track recursivelly if we can optimize the
    /// serialization of the type; i.e., if we can serialize the type without
    /// recursivelly calling the serialization of the inner types.
    ///
    /// This is used to optimize the serialization of arrays, tuples, etc.
    const WRITE_ALL_OPTIMIZATION: bool;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, backend: F) -> Result<F>;
}

pub trait Serialize: SerializeInner + Sized {
    fn serialize<F: WriteNoStd>(&self, backend: F) -> Result<usize> {
        Ok(self.serialize_on(WriteWithPos::new(backend))?.get_pos())
    }

    fn serialize_with_schema<F: WriteNoStd>(&self, backend: F) -> Result<Schema> {
        Ok(self
            .serialize_on(SchemaWriter::new(WriteWithPos::new(backend)))?
            .schema)
    }

    fn serialize_on<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        backend = backend.add_field("MAGIC", &MAGIC)?;
        backend = backend.add_field("MAJOR_VERSION", &VERSION.0)?;
        backend = backend.add_field("MINOR_VERSION", &VERSION.1)?;
        backend = backend.add_field("ROOT", self)?;
        Ok(backend)
    }
}
/// blanket impl so the user canno overwrite serialize and pad_align_to
impl<T: SerializeInner> Serialize for T {}
