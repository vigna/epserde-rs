/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use super::*;

/// Trait providing methods to write fields and bytes; moreover,
/// implementors need to keep track of the current position
/// in the [`WriteNoStd`] stream. This is needed to guarante the correct
/// alignment of the data to make zero-copy deserialization possible.
///
/// Note that the most default methods of [`FieldWrite`]
/// are reimplemented for [`SchemaWriter`], so it is fundamental to keep
/// the two implementations in sync.
pub trait FieldWrite: WriteNoStd + Sized {
    /// Get how many bytes we wrote since the start of the serialization.
    fn pos(&self) -> usize;

    #[inline(always)]
    /// Add some zero padding so that `self.get_pos() % align_of::<V>() == 0.`
    fn align<V>(&mut self) -> Result<()> {
        let padding = pad_align_to(self.pos(), core::mem::align_of::<V>());
        for _ in 0..padding {
            self.write(&[0])?;
        }
        Ok(())
    }

    /// This is the actual implementation of [`FieldWrite::write_field`]. It can be used
    /// by implementing types to simulate a call to the default implementation.
    #[inline(always)]
    fn do_write_field<V: SerializeInner>(self, _field_name: &str, value: &V) -> Result<Self> {
        if V::ZERO_COPY_MISMATCH {
            eprintln!("Type {} is zero copy, but it has not declared as such; use the #full_copy attribute to silence this warning", core::any::type_name::<V>());
        }
        value._serialize_inner(self)
    }

    fn write_field<V: SerializeInner>(self, field_name: &str, value: &V) -> Result<Self> {
        self.do_write_field::<V>(field_name, value)
    }

    /// This is the actual implementation of [`FieldWrite::write_bytes`]. It can be used
    /// by implementing types to simulate a call to the default implementation.
    #[inline(always)]
    fn do_write_bytes<V>(mut self, _field_name: &str, value: &[u8]) -> Result<Self> {
        self.write(value)?;
        Ok(self)
    }

    /// Write raw bytes.
    fn write_bytes<V>(self, field_name: &str, value: &[u8]) -> Result<Self> {
        self.do_write_bytes::<V>(field_name, value)
    }

    /// Writes a single aligned zero-copy value.
    fn write_field_zero<V: ZeroCopy + SerializeInner>(
        mut self,
        field_name: &str,
        value: &V,
    ) -> super::ser::Result<Self> {
        if !V::IS_ZERO_COPY {
            panic!(
                "Cannot serialize non zero-copy type {} declared as zero copy",
                core::any::type_name::<Self>()
            );
        }
        let buffer = unsafe {
            #[allow(clippy::manual_slice_size_calculation)]
            core::slice::from_raw_parts(value as *const V as *const u8, core::mem::size_of::<V>())
        };
        self.align::<V>()?;
        self.do_write_bytes::<V>(field_name, buffer)
    }

    /// Write a slice by encoding its length first, and then the contents properly aligned.
    fn write_slice<V: Serialize>(mut self, data: &[V]) -> Result<Self> {
        let len = data.len();
        self = self.write_field("len", &len)?;
        if V::ZERO_COPY_MISMATCH {
            eprintln!("Type {} is zero copy, but it has not declared as such; use the #full_copy attribute to silence this warning", core::any::type_name::<V>());
        }
        for item in data.iter() {
            self = self.write_field("item", item)?;
        }
        Ok(self)
    }

    /// Write an aligned slice by encoding its length first, and then the contents properly aligned.
    fn write_slice_zero<V: Serialize>(mut self, data: &[V]) -> Result<Self> {
        let len = data.len();
        self = self.write_field("len", &len)?;
        if !V::IS_ZERO_COPY {
            panic!(
                "Cannot serialize non zero-copy type {} declared as zero copy",
                core::any::type_name::<V>()
            );
        }
        let buffer = unsafe {
            #[allow(clippy::manual_slice_size_calculation)]
            core::slice::from_raw_parts(data.as_ptr() as *const u8, len * core::mem::size_of::<V>())
        };
        self.align::<V>()?;
        self.do_write_bytes::<V>("items", buffer)
    }
}

#[derive(Debug, Clone)]
/// A row in the schema csv
pub struct SchemaRow {
    /// Name of the field
    pub field: String,
    /// Type of the field
    pub ty: String,
    /// Offset of the field from the start of the file
    pub offset: usize,
    /// The length in bytes of the field
    pub size: usize,
    /// The alignment needed by the field, this is mostly to check if the
    /// serialization is correct
    pub align: usize,
}

#[derive(Default, Debug, Clone)]
/// All the informations needed to decode back the data from another language.
///
/// The schma is not guaranteed to be sorted.
pub struct Schema(pub Vec<SchemaRow>);

impl Schema {
    /// Return in a String the csv representation of the schema
    /// also printing the bytes of the data used to decode each leaf field.
    ///
    /// The schema is not guaranteed to be sorted, so if you need it sorted use:
    ///  `schema.0.sort_by_key(|row| row.offset);`
    ///
    /// WARNING: the size of the csv will be bigger than the size of the
    /// serialized file, so it's a bad idea calling this on big data structures.
    pub fn debug(&self, data: &[u8]) -> String {
        let mut result = "field,offset,align,size,ty,bytes\n".to_string();
        for i in 0..self.0.len().saturating_sub(1) {
            let row = &self.0[i];
            // if it's a composed type, don't print the bytes
            if row.offset == self.0[i + 1].offset {
                result.push_str(&format!(
                    "{},{},{},{},{},\n",
                    row.field, row.offset, row.align, row.size, row.ty,
                ));
            } else {
                result.push_str(&format!(
                    "{},{},{},{},{},{:02x?}\n",
                    row.field,
                    row.offset,
                    row.align,
                    row.size,
                    row.ty,
                    &data[row.offset..row.offset + row.size],
                ));
            }
        }

        // the last field can't be a composed type by definition
        if let Some(row) = self.0.last() {
            result.push_str(&format!(
                "{},{},{},{},{},{:02x?}\n",
                row.field,
                row.offset,
                row.align,
                row.size,
                row.ty,
                &data[row.offset..row.offset + row.size],
            ));
        }

        result
    }

    /// Return in a String the csv representation of the schema.
    ///
    /// The schema is not guaranteed to be sorted, so if you need it sorted use:
    ///  `schema.0.sort_by_key(|row| row.offset);`
    pub fn to_csv(&self) -> String {
        let mut result = "field,offset,align,size,ty\n".to_string();
        for row in &self.0 {
            result.push_str(&format!(
                "{},{},{},{},{}\n",
                row.field, row.offset, row.align, row.size, row.ty
            ));
        }
        result
    }
}

/// Internal writer that keeps track of the schema and the path of the field
/// we are serializing
pub struct SchemaWriter<W: FieldWrite> {
    /// The schema so far
    pub schema: Schema,
    /// The "path" of the previous fields names
    path: Vec<String>,
    /// What we actually write on
    writer: W,
}

impl<W: FieldWrite> SchemaWriter<W> {
    #[inline(always)]
    /// Create a new empty [`SchemaWriter`] on top of a generic writer `W`
    pub fn new(backend: W) -> Self {
        Self {
            schema: Default::default(),
            path: vec![],
            writer: backend,
        }
    }
}

impl<W: FieldWrite> FieldWrite for SchemaWriter<W> {
    #[inline(always)]
    fn pos(&self) -> usize {
        self.writer.pos()
    }

    #[inline(always)]
    fn align<V>(&mut self) -> Result<()> {
        let padding = pad_align_to(self.pos(), core::mem::align_of::<V>());
        if padding == 0 {
            return Ok(());
        }

        let off = self.schema.0.last_mut().unwrap().offset;

        for row in self.schema.0.iter_mut().rev() {
            if row.offset < off {
                break;
            }
            row.offset += padding;
        }

        self.schema.0.push(SchemaRow {
            field: "PADDING".into(),
            ty: format!("[u8; {}]", padding),
            offset: self.pos(),
            size: padding,
            align: 1,
        });

        for _ in 0..padding {
            self.write(&[0])?;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_field<V: SerializeInner>(mut self, field_name: &str, value: &V) -> Result<Self> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        let struct_idx = self.schema.0.len();
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: core::any::type_name::<V>().to_string(),
            offset: self.pos(),
            align: core::mem::align_of::<V>(),
            size: 0,
        });
        // compute the serialized size and update the schema
        let size = self.pos() - self.schema.0[struct_idx].offset;
        self.schema.0[struct_idx].size = size;
        self.path.pop();

        <Self as FieldWrite>::do_write_field::<V>(self, field_name, value)
    }

    #[inline(always)]
    fn write_bytes<V>(mut self, field_name: &str, value: &[u8]) -> Result<Self> {
        let align = core::mem::align_of::<V>();
        let type_name = core::any::type_name::<V>().to_string();
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: type_name,
            offset: self.pos(),
            size: value.len(),
            align,
        });
        self.path.pop();

        <Self as FieldWrite>::do_write_bytes::<V>(self, field_name, value)
    }
}

impl<W: FieldWrite> WriteNoStd for SchemaWriter<W> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}
