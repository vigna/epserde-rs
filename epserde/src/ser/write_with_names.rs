/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Traits and implementations to write named field during serialization.

[`SerializeInner::_serialize_inner`] writes on a [`WriteWithNames`], rather
than on a [`WriteWithPos`], with the purpose of easily recording write
events happening during a serialization.

*/

use super::*;
use mem_dbg::{MemDbg, MemSize};

/// Trait extending [`WriteWithPos`] with methods providing
/// alignment, serialization of named data, and writing of byte slices
/// of zero-copy types.
///
/// The purpose of this trait is that of interposing between [`SerializeInner`]
/// and the underlying [`WriteWithPos`] a layer in which serialization operations
/// can be easily intercepted and recorded. In particular, serialization methods
/// must use the methods of this trait if they want to record the schema of the
/// serialized data; this is true (maybe counterintuitively) even of ancillary
/// data such as tags and slice lengths: see [`helpers`] or the
/// [implementation of `Option`](impls::prim) for examples.
/// All methods have a default
/// implementation that must be replicated in other implementations.
///
/// There are two implementations of [`WriteWithNames`]: [`WriterWithPos`],
/// which uses the default implementation, and [`SchemaWriter`],
/// which additionally records a [`Schema`] of the serialized data.
pub trait WriteWithNames: WriteWithPos + Sized {
    /// Add some zero padding so that `self.pos() % V:max_size_of() == 0.`
    ///
    /// Other implementations must write the same number of zeros.
    fn align<V: MaxSizeOf>(&mut self) -> Result<()> {
        let padding = pad_align_to(self.pos(), V::max_size_of());
        for _ in 0..padding {
            self.write_all(&[0])?;
        }
        Ok(())
    }

    /// Write a value with an associated name.
    ///
    /// The default implementation simply delegates to [`SerializeInner::_serialize_inner`].
    /// Other implementations might use the name information (e.g., [`SchemaWriter`]),
    /// but they must in the end delegate to [`SerializeInner::_serialize_inner`].
    fn write<V: SerializeInner>(&mut self, _field_name: &str, value: &V) -> Result<()> {
        unsafe { value._serialize_inner(self) }
    }

    /// Write the memory representation of a (slice of a) zero-copy type.
    ///
    /// The default implementation simply delegates to [`WriteNoStd::write_all`].
    /// Other implementations might use the type information in `V` (e.g., [`SchemaWriter`]),
    /// but they must in the end delegate to [`WriteNoStd::write_all`].
    fn write_bytes<V: SerializeInner + ZeroCopy>(&mut self, value: &[u8]) -> Result<()> {
        self.write_all(value)
    }
}

impl<F: WriteNoStd> WriteWithNames for WriterWithPos<'_, F> {}

/// Information about data written during serialization, either fields or
/// ancillary data such as option tags and slice lengths.
#[derive(Debug, Clone, MemDbg, MemSize)]
pub struct SchemaRow {
    /// Name of the piece of data.
    pub field: String,
    /// Type of the piece of data.
    pub ty: String,
    /// Offset from the start of the file.
    pub offset: usize,
    /// Length in bytes of the piece of data.
    pub size: usize,
    /// The alignment needed by the piece of data, zero if not applicable
    /// (e.g., primitive fields, ancillary data, or structures).
    pub align: usize,
}

#[derive(Default, Debug, Clone, MemDbg, MemSize)]
/// A vector containing all the fields written during serialization, including
/// ancillary data such as slice lengths and [`Option`] tags.
pub struct Schema(pub Vec<SchemaRow>);

impl Schema {
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

/// A [`WriteWithNames`] that keeps track of the data written on an underlying
/// [`WriteWithPos`] in a [`Schema`].
#[derive(Debug, MemDbg, MemSize)]
pub struct SchemaWriter<'a, W> {
    /// The schema so far.
    pub schema: Schema,
    /// A recursively-built sequence of previous names.
    path: Vec<String>,
    /// What we actually write on.
    writer: &'a mut W,
}

impl<'a, W: WriteWithPos> SchemaWriter<'a, W> {
    /// Create a new empty [`SchemaWriter`] on top of a generic writer `W`.
    pub fn new(backend: &'a mut W) -> Self {
        Self {
            schema: Default::default(),
            path: vec![],
            writer: backend,
        }
    }
}
impl<W: WriteNoStd> WriteNoStd for SchemaWriter<'_, W> {
    fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
        self.writer.write_all(buf)
    }

    fn flush(&mut self) -> ser::Result<()> {
        self.writer.flush()
    }
}

impl<W: WriteWithPos> WriteWithPos for SchemaWriter<'_, W> {
    fn pos(&self) -> usize {
        self.writer.pos()
    }
}

/// WARNING: these implementations must be kept in sync with the ones
/// in the default implementation of [`WriteWithNames`].
impl<W: WriteWithPos> WriteWithNames for SchemaWriter<'_, W> {
    fn align<T: MaxSizeOf>(&mut self) -> Result<()> {
        let padding = pad_align_to(self.pos(), T::max_size_of());
        if padding != 0 {
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
        }

        Ok(())
    }

    fn write<V: SerializeInner>(&mut self, field_name: &str, value: &V) -> Result<()> {
        // prepare a row with the field name and the type
        self.path.push(field_name.into());
        let pos = self.pos();

        let len = self.schema.0.len();
        unsafe { value._serialize_inner(self)? };

        // This is slightly inefficient because we have to shift
        // the whole vector, but it's not a big deal and it keeps
        // the schema in the correct order.
        self.schema.0.insert(
            len,
            SchemaRow {
                field: self.path.join("."),
                ty: core::any::type_name::<V>().to_string(),
                offset: pos,
                align: 0,
                size: self.pos() - pos,
            },
        );
        self.path.pop();
        Ok(())
    }

    fn write_bytes<V: SerializeInner + ZeroCopy>(&mut self, value: &[u8]) -> Result<()> {
        self.path.push("zero".to_string());
        // Note that we are writing the schema row of the field before
        // having written its content.
        self.schema.0.push(SchemaRow {
            field: self.path.join("."),
            ty: core::any::type_name::<V>().to_string(),
            offset: self.pos(),
            size: value.len(),
            align: V::max_size_of(),
        });
        self.path.pop();

        self.write_all(value)
    }
}
