/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Deserialization traits and types

[`Deserialize`] is the main deserialization trait, providing methods
[`Deserialize::deserialize_eps_copy`] and [`Deserialize::deserialize_full_copy`]
which implement ε-copy and full-copy deserialization, respectively,
starting from a slice of bytes. The implementation of this trait
is based on [`DeserializeInner`], which is automatically derived
with `#[derive(Deserialize)]`.

Note that [`Deserialize::deserialize_full_copy`] is internally necessary
to deserialize fields whose type is not a parameter, but technically
it could be hidden from the user interface. It can however be useful
for debugging and in cases in which a full copy is necessary.

*/
use crate::{Serialize, TypeHash, MAGIC, MAGIC_REV, VERSION};
use core::hash::Hasher;
use std::{io::BufReader, path::Path};

mod des_impl;
pub use des_impl::*;

pub type Result<T> = core::result::Result<T, DeserializeError>;

/// Inner trait to implement deserialization of a type. This trait exists
/// to separate the user-facing [`Deserialize`] trait from the low-level
/// deserialization mechanisms of [`DeserializeInner::_deserialize_full_copy_inner`]
/// and [`DeserializeInner::_deserialize_eps_copy_inner`]. Moreover,
/// it makes it possible to behave slighly differently at the top
/// of the recursion tree (e.g., to check the endianness marker), and to prevent
/// the user from modifying the methods in [`Deserialize`].
///
/// The user should not implement this trait directly, but rather derive it.
pub trait DeserializeInner: TypeHash + Sized {
    type DeserType<'a>: TypeHash;
    fn _deserialize_full_copy_inner<R: ReadWithPos>(backend: R) -> Result<(Self, R)>;

    fn _deserialize_eps_copy_inner(
        backend: SliceWithPos,
    ) -> Result<(Self::DeserType<'_>, SliceWithPos)>;
}

/// Main serialization trait. It is separated from [`DeserializeInner`] to
/// avoid that the user modify its behavior, and hide internal serialization
/// methods.
pub trait Deserialize: DeserializeInner {
    /// Full-copy deserialize a structure of this type from the given backend.
    fn deserialize_full_copy(backend: impl ReadNoStd) -> Result<Self>;
    /// ε-copy deserialize a structure of this type from the given backend.
    fn deserialize_eps_copy(backend: &'_ [u8]) -> Result<Self::DeserType<'_>>;

    fn load_full(&self, path: impl AsRef<Path>) -> Result<Self> {
        let mut file = std::fs::File::open(path).map_err(DeserializeError::FileOpenError)?;
        let mut buf_reader = BufReader::new(file);
        Self::deserialize_full_copy(&mut buf_reader)
    }
}

impl<T: DeserializeInner + TypeHash + Serialize> Deserialize for T {
    fn deserialize_full_copy(backend: impl ReadNoStd) -> Result<Self> {
        let mut backend = ReaderWithPos::new(backend);

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        Self::type_hash(&mut hasher);
        let self_hash = hasher.finish();

        backend = check_header(
            backend,
            self_hash,
            core::any::type_name::<Self>().to_string(),
        )?;
        let (res, _) = Self::_deserialize_full_copy_inner(backend)?;
        Ok(res)
    }
    fn deserialize_eps_copy(backend: &'_ [u8]) -> Result<Self::DeserType<'_>> {
        let mut backend = SliceWithPos::new(backend);

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        Self::type_hash(&mut hasher);
        let self_hash = hasher.finish();

        backend = check_header(
            backend,
            self_hash,
            core::any::type_name::<Self>().to_string(),
        )?;
        let (res, _) = Self::_deserialize_eps_copy_inner(backend)?;
        Ok(res)
    }
}

/// Common code for both full-copy and zero-copy deserialization
fn check_header<R: ReadWithPos>(backend: R, self_hash: u64, self_name: String) -> Result<R> {
    let (magic, backend) = u64::_deserialize_full_copy_inner(backend)?;
    match magic {
        MAGIC => Ok(()),
        MAGIC_REV => Err(DeserializeError::EndiannessError),
        magic => Err(DeserializeError::MagicNumberError(magic)),
    }?;

    let (major, backend) = u16::_deserialize_full_copy_inner(backend)?;
    if major != VERSION.0 {
        return Err(DeserializeError::MajorVersionMismatch(major));
    }
    let (minor, backend) = u16::_deserialize_full_copy_inner(backend)?;
    if minor > VERSION.1 {
        return Err(DeserializeError::MinorVersionMismatch(minor));
    };

    let (usize_size, backend) = u16::_deserialize_full_copy_inner(backend)?;
    let usize_size = usize_size as usize;
    let native_usize_size = core::mem::size_of::<usize>();
    if usize_size != native_usize_size {
        return Err(DeserializeError::UsizeSizeMismatch(usize_size));
    };

    let (type_hash, backend) = u64::_deserialize_full_copy_inner(backend)?;
    let (type_name, backend) = String::_deserialize_full_copy_inner(backend)?;

    if type_hash != self_hash {
        return Err(DeserializeError::WrongTypeHash {
            got_type_name: self_name,
            got: self_hash,
            expected_type_name: type_name,
            expected: type_hash,
        });
    }

    Ok(backend)
}

#[derive(Debug)]
/// Errors that can happen during deserialization
pub enum DeserializeError {
    /// [`Deserialize::load_full`] could not open the provided file.
    FileOpenError(std::io::Error),
    /// The underlying reader returned an error
    ReadError,
    /// The file is reasonable but the endianess is wrong.
    EndiannessError,
    /// Some field is not properly aligned. This can be either a serialization
    /// bug or a collision in the type hash.
    AlignmentError,
    /// The file was serialized with a version of epserde that is not compatible
    MajorVersionMismatch(u16),
    /// The file was serialized with a compatible, but too new version of epserde
    /// so we might be missing features.
    MinorVersionMismatch(u16),
    /// The the `pointer_width` of the serialized file is different from the
    /// `pointer_width` of the current architecture.
    /// E.g. the file was serialized on a 64-bit machine and we are trying to
    /// deserialize it on a 32-bit machine.
    /// We could check if the usizes are actually used, but currently we don't.
    UsizeSizeMismatch(usize),
    /// The magic number is wrong. The file is not an epserde file.
    MagicNumberError(u64),
    /// A tag is wrong (e.g., for [`Option`]).
    InvalidTag(u8),
    /// The type hash is wrong. Probabliy the user is trying to deserialize a
    /// file with the wrong type.
    WrongTypeHash {
        got_type_name: String,
        expected_type_name: String,
        expected: u64,
        got: u64,
    },
}

impl std::error::Error for DeserializeError {}

impl core::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::ReadError => write!(f, "Read error during ε-serde serialization"),
            Self::FileOpenError(error) => {
                write!(f, "Write error during ε-serde serialization: {}", error)
            }
            Self::EndiannessError => write!(
                f,
                "The current arch is {}-endian but the data is {}-endian.",
                if cfg!(target_endian = "little") {
                    "little"
                } else {
                    "big"
                },
                if cfg!(target_endian = "little") {
                    "big"
                } else {
                    "little"
                }
            ),
            Self::MagicNumberError(magic) => write!(
                f,
                "Wrong Magic Number Error. Got {:?} but the only two valids are {:?} and {:?}",
                magic,
                crate::MAGIC.to_le(),
                crate::MAGIC.to_be(),
            ),
            Self::MajorVersionMismatch(found_major) => write!(
                f,
                "Major Version Mismatch. Expected {} but got {}",
                VERSION.0, found_major,
            ),
            Self::MinorVersionMismatch(found_minor) => write!(
                f,
                "Minor Version Mismatch. Expected {} but got {}",
                VERSION.1, found_minor,
            ),
            Self::UsizeSizeMismatch(usize_size) => write!(
                f,
                "The file was serialized on a machine where an usize is {} bytes, but on the current machine it is {}.",
                usize_size,
                core::mem::size_of::<usize>()
            ),
            Self::AlignmentError => write!(f, "Alignment Error"),
            Self::InvalidTag(tag) => write!(f, "Invalid tag: 0x{:02x}", tag),
            Self::WrongTypeHash {
                got_type_name,
                expected_type_name,
                expected,
                got,
            } => {
                write!(
                    f,
                    concat!(
                        "Wrong type hash. Expected=0x{:016x}, Got=0x{:016x}.\n",
                        "The serialized type is '{}' but the deserialized type is '{}'",
                    ),
                    expected, got, expected_type_name, got_type_name,
                )
            }
        }
    }
}

/// [`std::io::Cursor`]-like trait for deserialization that does not
/// depend on [`std`].
#[derive(Debug)]
pub struct SliceWithPos<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> SliceWithPos<'a> {
    pub fn new(backend: &'a [u8]) -> Self {
        Self {
            data: backend,
            pos: 0,
        }
    }

    pub fn skip(&self, bytes: usize) -> Self {
        Self {
            data: &self.data[bytes..],
            pos: self.pos + bytes,
        }
    }
}

impl<'a> ReadNoStd for SliceWithPos<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = buf.len();
        if len > self.data.len() {
            return Err(DeserializeError::ReadError);
        }
        buf.copy_from_slice(&self.data[..len]);
        self.data = &self.data[len..];
        self.pos += len;
        Ok(len)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.read(buf).map(|_| ())
    }
}

impl<'a> ReadWithPos for SliceWithPos<'a> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn pad_align_and_check<T>(mut self) -> Result<Self> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, core::mem::align_of::<Self>());
        self = self.skip(padding);
        // Check that the ptr is indeed aligned
        if self.data.as_ptr() as usize % std::mem::align_of::<Self>() != 0 {
            Err(DeserializeError::AlignmentError)
        } else {
            Ok(self)
        }
    }
}

/// [`std::io::Read`]-like trait for serialization that does not
/// depend on [`std`].
///
/// In an [`std`] context, the user does not need to use directly
/// this trait as we provide a blanket
/// implementation that implements [`ReadNoStd`] for all types that implement
/// [`std::io::Read`]. In particular, in such a context you can use [`std::io::Cursor`]
/// for in-memory deserialization.
pub trait ReadNoStd {
    /// Read some bytes and return the number of bytes read
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
}

#[cfg(feature = "std")]
use std::io::Read;
#[cfg(feature = "std")]
impl<W: Read> ReadNoStd for W {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Read::read(self, buf).map_err(|_| DeserializeError::ReadError)
    }

    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        Read::read_exact(self, buf).map_err(|_| DeserializeError::ReadError)
    }
}

/// A trait for [`ReadNoStd`] that also keeps track of the current position.
///
/// This is needed because the [`Read`] trait doesn't have a `seek` method and
/// [`std::io::Seek`] would be a requirement much stronger than needed.
pub trait ReadWithPos: ReadNoStd + Sized {
    fn pos(&self) -> usize;

    /// Pad the cursor to the correct alignment and check that the resulting
    /// pointer is aligned correctly.
    fn pad_align_and_check<T>(self) -> Result<Self>;
}

/// A wrapper for a [`ReadNoStd`] that implements [`ReadWithPos`]
/// by keeping track of the current position.
pub struct ReaderWithPos<F: ReadNoStd> {
    /// What we actually readfrom
    backend: F,
    /// How many bytes we have read from the start
    pos: usize,
}

impl<F: ReadNoStd> ReaderWithPos<F> {
    #[inline(always)]
    /// Create a new [`ReadWithPos`] on top of a generic Reader `F`
    pub fn new(backend: F) -> Self {
        Self { backend, pos: 0 }
    }
}

impl<F: ReadNoStd> ReadNoStd for ReaderWithPos<F> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let res = self.backend.read(buf)?;
        self.pos += res;
        Ok(res)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.backend.read_exact(buf)?;
        self.pos += buf.len();
        Ok(())
    }
}

impl<F: ReadNoStd> ReadWithPos for ReaderWithPos<F> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn pad_align_and_check<T>(mut self) -> Result<Self> {
        // Skip bytes as needed
        let padding = crate::pad_align_to(self.pos, core::mem::align_of::<Self>());
        self.read_exact(&mut vec![0; padding]);
        // No alignment check, we are fully deserializing
        Ok(self)
    }
}
