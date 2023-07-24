use crate::*;

pub type Result<T> = core::result::Result<T, core::fmt::Error>;

pub trait WriteNoStd {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
}

pub trait WriteWithPosNoStd: WriteNoStd + Sized {
    #[inline(always)]
    fn add_field<V: SerializeInner>(self, _field_name: &str, _value: &V) -> Result<Self> {
        Ok(self)
    }
    #[inline(always)]
    fn add_field_bytes(self, _field_name: &str, _type_name: String, _value: &[u8]) -> Result<Self> {
        Ok(self)
    }
    fn get_pos(&self) -> usize;
}

#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
/// Forward the write impl so we can natively use Files, etc.
impl<W: Write> WriteNoStd for W {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Write::write(self, buf).map_err(|_| core::fmt::Error)
    }
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self).map_err(|_| core::fmt::Error)
    }
}

#[derive(Debug, Clone)]
pub struct SchemaRow {
    pub field: String,
    pub ty: String,
    pub offset: usize,
    pub size: usize,
}

#[derive(Default, Debug, Clone)]
pub struct Schema(pub Vec<SchemaRow>);

impl Schema {
    pub fn to_csv(&self) -> String {
        let mut result = "field,offset,size,ty\n".to_string();
        for row in &self.0 {
            result.push_str(&format!(
                "{},{},{},{}\n",
                row.field, row.offset, row.size, row.ty
            ));
        }
        result
    }
}

pub struct SchemaWriter<W: WriteWithPosNoStd> {
    pub schema: Schema,
    path: Vec<String>,
    writer: W,
}

impl<W: WriteWithPosNoStd> SchemaWriter<W> {
    #[inline(always)]
    fn new(backend: W) -> Self {
        Self {
            schema: Default::default(),
            path: vec![],
            writer: backend,
        }
    }
}

impl<W: WriteWithPosNoStd> WriteWithPosNoStd for SchemaWriter<W> {
    #[inline(always)]
    fn add_field<V: SerializeInner>(mut self, field_name: &str, value: &V) -> Result<Self> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        let struct_idx = self.schema.0.len();
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: V::type_name(),
            offset: self.get_pos(),
            size: 0,
        });
        // serialize the value
        self = value._serialize_inner(self)?;
        // compute the serialized size and update the schema
        let size = self.get_pos() - self.schema.0[struct_idx].offset;
        self.schema.0[struct_idx].size = size;
        self.path.pop();
        Ok(self)
    }

    #[inline(always)]
    fn add_field_bytes(
        mut self,
        field_name: &str,
        type_name: String,
        value: &[u8],
    ) -> Result<Self> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: type_name,
            offset: self.get_pos(),
            size: value.len(),
        });
        self.writer.write(value)?;
        self.path.pop();
        Ok(self)
    }

    #[inline(always)]
    fn get_pos(&self) -> usize {
        self.writer.get_pos()
    }
}

impl<W: WriteWithPosNoStd> WriteNoStd for SchemaWriter<W> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}

/// A little wrapper around a writer that keeps track of the current position
/// so we can align the data.
pub struct WriteWithPos<F: WriteNoStd> {
    backend: F,
    pos: usize,
}

impl<F: WriteNoStd> WriteWithPos<F> {
    #[inline(always)]
    fn new(backend: F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: WriteNoStd> WriteWithPosNoStd for WriteWithPos<F> {
    #[inline(always)]
    fn get_pos(&self) -> usize {
        self.pos
    }
}

impl<F: WriteNoStd> WriteNoStd for WriteWithPos<F> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let res = self.backend.write(buf)?;
        self.pos += res;
        Ok(res)
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }
}

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
