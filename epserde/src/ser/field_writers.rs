/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use super::*;

/// Trait providing methods to write fields and bytes associating
/// a name with each operation; moreover,
/// implementors need to keep track of the current position
/// in the [`WriteNoStd`] stream. This is needed to guarante the correct
/// alignment of the data to make zero-copy deserialization possible.
///
/// There are two implementations of [`FieldWrite`]: one is [`WriterWithPos`],
/// which simply delegates to [`SerializeInner::_serialize_inner`], and
/// [`SchemaWriter`], which additionally records the [`Schema`]
/// of the serialized data
///
/// Note that the some default methods of [`FieldWrite`]
/// are reimplemented for [`SchemaWriter`], so it is fundamental to keep
/// the two implementations in sync.
pub trait FieldWrite: WriteNoStd + Sized {
    /// Get how many bytes we wrote since the start of the serialization.
    fn pos(&self) -> usize;

    /// Add some zero padding so that `self.pos() % V:max_size_of() == 0.`
    fn align<V: MaxSizeOf>(&mut self) -> Result<()> {
        let padding = pad_align_to(self.pos(), V::max_size_of());
        for _ in 0..padding {
            self.write_all(&[0])?;
        }
        Ok(())
    }

    /// This is the actual implementation of [`FieldWrite::write_field`], which
    /// delegates to [`SerializeInner::_serialize_inner`].
    ///
    /// It can be used
    /// by implementing types to simulate a call to the default implementation.
    #[inline(always)]
    fn do_write_field(&mut self, _field_name: &str, value: &impl SerializeInner) -> Result<()> {
        value._serialize_inner(self)
    }

    /// Writes a field to the given backend.
    ///
    /// This method is used for full-copy types and ancillary data such as slice lengths.
    /// Data written by this method can be always full-copy deserialized.
    fn write_field<V: SerializeInner>(&mut self, field_name: &str, value: &V) -> Result<()> {
        self.do_write_field(field_name, value)
    }

    /// This is the actual implementation of [`FieldWrite::write_bytes`]. It can be used
    /// by implementing types to simulate a call to the default implementation.
    #[inline(always)]
    fn do_write_bytes<V: ZeroCopy>(&mut self, _field_name: &str, value: &[u8]) -> Result<()> {
        self.align::<V>()?;
        self.write_all(value)
    }

    /// Write raw bytes [aligned](FieldWrite::align) using `V`.
    ///
    /// This method is used by [`FieldWrite::write_field_zero`] and
    /// [`FieldWrite::write_slice_zero`] to write zero-copy types. Data written
    /// by this method be zero-copy or full-copy deserialized.
    fn write_bytes<V: ZeroCopy>(&mut self, field_name: &str, value: &[u8]) -> Result<()> {
        self.do_write_bytes::<V>(field_name, value)
    }

    /// Writes an [aligned](FieldWrite::align) zero-copy value.
    ///
    /// Here we check [that the type is actually zero-copy](SerializeInner::IS_ZERO_COPY).
    fn write_field_zero<V: ZeroCopy + SerializeInner>(
        &mut self,
        field_name: &str,
        value: &V,
    ) -> super::ser::Result<()> {
        if !V::IS_ZERO_COPY {
            panic!(
                "Cannot serialize deep-copy type {} declared as zero-copy",
                core::any::type_name::<Self>()
            );
        }
        let buffer = unsafe {
            #[allow(clippy::manual_slice_size_calculation)]
            core::slice::from_raw_parts(value as *const V as *const u8, core::mem::size_of::<V>())
        };
        self.write_bytes::<V>(field_name, buffer)
    }

    /// Write a slice by encoding its length first, and then the contents.
    fn write_slice<V: SerializeInner>(&mut self, data: &[V]) -> Result<()> {
        let len = data.len();
        self.write_field("len", &len)?;
        if V::ZERO_COPY_MISMATCH {
            eprintln!("Type {} is zero-copy, but it has not declared as such; use the #full_copy attribute to silence this warning", core::any::type_name::<V>());
        }
        for item in data.iter() {
            self.write_field("item", item)?;
        }
        Ok(())
    }

    /// Write a slice of zero-copy structures by encoding
    /// its length first, and then the contents properly [aligned](FieldWrite::align).
    ///
    /// Note that this method uses a single [`WriteNoStd::write_all`]
    /// call to write the entire slice.
    ///
    /// Here we check [that the type is actually zero-copy](SerializeInner::IS_ZERO_COPY).
    fn write_slice_zero<V: SerializeInner + ZeroCopy>(&mut self, data: &[V]) -> Result<()> {
        let len = data.len();
        self.write_field("len", &len)?;
        if !V::IS_ZERO_COPY {
            panic!(
                "Cannot serialize non zero-copy type {} declared as zero-copy",
                core::any::type_name::<V>()
            );
        }
        let buffer = unsafe {
            #[allow(clippy::manual_slice_size_calculation)]
            core::slice::from_raw_parts(data.as_ptr() as *const u8, len * core::mem::size_of::<V>())
        };
        self.write_bytes::<V>("items", buffer)
    }
}

#[derive(Debug, Clone)]
pub struct SchemaRow {
    /// Name of the field.
    pub field: String,
    /// Type of the field.
    pub ty: String,
    /// Offset of the field from the start of the file.
    pub offset: usize,
    /// The length in bytes of the field.
    pub size: usize,
    /// The alignment needed by the field.
    pub align: usize,
}

#[derive(Default, Debug, Clone)]
/// A vector containing all the fields written during serialization, including
/// ancillary data such as slice length and [`Option`] tags.
pub struct Schema(pub Vec<SchemaRow>);

impl Schema {
    /// Sort the values of the schema by offset and then by type size.
    pub fn sort(&mut self) {
        self.0.sort_by_key(|row| (row.offset, -(row.size as isize)));
    }
    /// Return a CSV representation of the schema, including data.
    ///
    /// WARNING: the size of the CSV will be larger than the size of the
    /// serialized file, so it is not a good idea to call this method
    /// on big structures.
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

    /// Return a CSV representation of the schema, excluding data.
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
    /// The schema so far.
    pub schema: Schema,
    /// The "path" of the previous fields names.
    path: Vec<String>,
    /// What we actually write on.
    writer: W,
}

impl<W: FieldWrite> SchemaWriter<W> {
    #[inline(always)]
    /// Create a new empty [`SchemaWriter`] on top of a generic writer `W`.
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
    fn align<T: MaxSizeOf>(&mut self) -> Result<()> {
        let padding = pad_align_to(self.pos(), T::max_size_of());
        if padding == 0 {
            return Ok(());
        }

        self.schema.0.push(SchemaRow {
            field: "PADDING".into(),
            ty: format!("[u8; {}]", padding),
            offset: self.pos(),
            size: padding,
            align: 1,
        });

        for _ in 0..padding {
            self.write_all(&[0])?;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_field<V: SerializeInner>(&mut self, field_name: &str, value: &V) -> Result<()> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        let pos = self.pos();
        <Self as FieldWrite>::do_write_field(self, field_name, value)?;

        // Note that we are writing the schema row of the field after
        // having written its content.
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: core::any::type_name::<V>().to_string(),
            offset: pos,
            align: 0, // TODO V::align_of(),
            size: self.pos() - pos,
        });
        self.path.pop();
        Ok(())
    }

    #[inline(always)]
    fn write_bytes<V: ZeroCopy>(&mut self, field_name: &str, value: &[u8]) -> Result<()> {
        let align = core::mem::align_of::<V>();
        let type_name = core::any::type_name::<V>().to_string();

        self.path.push(field_name.into());
        // Note that we are writing the schema row of the field before
        // having written its content.
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
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.writer.write_all(buf)
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}
