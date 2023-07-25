use crate::*;

pub mod ser_impl;
pub use ser_impl::*;

pub mod ser_writers;
pub use ser_writers::*;

/// Compute the padding needed for alignement, i.e., the number so that
/// `((value + pad_align_to(value, bits) & (bits - 1) == 0`.
///
/// ```
/// use epserde_trait::pad_align_to;
/// assert_eq!(7 + pad_align_to(7, 8), 8);
/// assert_eq!(8 + pad_align_to(8, 8), 8);
/// assert_eq!(9 + pad_align_to(9, 8), 16);
/// ```
pub fn pad_align_to(value: usize, bits: usize) -> usize {
    value.wrapping_neg() & (bits - 1)
}

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

    /// Write 0 as padding to align to the size of `T`.
    fn pad_align_to<F: WriteWithPosNoStd>(mut backend: F) -> Result<F> {
        let file_pos = backend.get_pos();
        let padding = pad_align_to(file_pos, core::mem::size_of::<Self>());
        for _ in 0..padding {
            backend.write(&[0])?;
        }
        Ok(backend)
    }
}
/// blanket impl so the user canno overwrite serialize and pad_align_to
impl<T: SerializeInner> Serialize for T {}
