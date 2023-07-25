use super::*;
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
    pub fn new(backend: W) -> Self {
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
    pub fn new(backend: F) -> Self {
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
