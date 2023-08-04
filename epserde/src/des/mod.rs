//! Deserialization traits and types
//!
//! [`Deserialize`] is the main deserialization trait, providing methods
//! [`Deserialize::deserialize_eps_copy`] method that serializes the type into a
//! generic [`WriteNoStd`] backend. The implementation of this trait
//! is based on [`SerializeInner`], which is automatically derived
//! with `#[derive(Serialize)]`.
use crate::{Serialize, TypeName, MAGIC, MAGIC_REV, VERSION};
use core::hash::Hasher;

pub mod des_impl;
pub use des_impl::*;
pub mod des_zc_impl;
pub use des_zc_impl::*;

/// The inner trait to implement full-copy deserialization.
/// The user should implement this trait directly but rather derive it.
pub trait DeserializeInner: TypeName + Sized {
    type DeserType<'b>: TypeName;
    fn _deserialize_inner<'a>(backend: Cursor<'a>) -> Result<(Self, Cursor<'a>), DeserializeError>;
    fn _deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DeserType<'a>, Cursor<'a>), DeserializeError>;
}

/// User-facing trait for full-copy deserialization
/// The user should implement this trait directly but rather derive it.
pub trait Deserialize: DeserializeInner {
    fn deserialize_full_copy(backend: &[u8]) -> Result<Self, DeserializeError>;
    fn deserialize_eps_copy<'a>(backend: &'a [u8])
        -> Result<Self::DeserType<'a>, DeserializeError>;
}

impl<T: DeserializeInner + TypeName + Serialize> Deserialize for T {
    fn deserialize_full_copy(backend: &[u8]) -> Result<Self, DeserializeError> {
        let mut backend = Cursor::new(backend);

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        Self::type_hash(&mut hasher);
        let self_hash = hasher.finish();

        backend = check_header(backend, self_hash, Self::type_name())?;
        let (res, _) = Self::_deserialize_inner(backend)?;
        Ok(res)
    }
    fn deserialize_eps_copy<'a>(
        backend: &'a [u8],
    ) -> Result<Self::DeserType<'a>, DeserializeError> {
        let mut backend = Cursor::new(backend);

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        Self::type_hash(&mut hasher);
        let self_hash = hasher.finish();

        backend = check_header(backend, self_hash, Self::type_name())?;
        let (res, _) = Self::_deserialize_zc_inner(backend)?;
        Ok(res)
    }
}

/// Common code for both full-copy and zero-copy deserialization
fn check_header<'a>(
    backend: Cursor<'a>,
    self_hash: u64,
    self_name: String,
) -> Result<Cursor<'a>, DeserializeError> {
    let (magic, backend) = u64::_deserialize_inner(backend)?;
    match magic {
        MAGIC => Ok(()),
        MAGIC_REV => Err(DeserializeError::EndiannessError),
        magic => Err(DeserializeError::MagicNumberError(magic)),
    }?;

    let (major, backend) = u32::_deserialize_inner(backend)?;
    if major != VERSION.0 {
        return Err(DeserializeError::MajorVersionMismatch(major));
    }
    let (minor, backend) = u32::_deserialize_inner(backend)?;
    if minor > VERSION.1 {
        return Err(DeserializeError::MinorVersionMismatch(minor));
    };

    let (type_hash, backend) = u64::_deserialize_inner(backend)?;
    let (type_name, backend) = String::_deserialize_zc_inner(backend)?;

    if type_hash != self_hash {
        return Err(DeserializeError::WrongTypeHash {
            got_type_name: self_name,
            expected_type_name: type_name.to_string(),
            expected: self_hash,
            got: type_hash,
        });
    }

    Ok(backend)
}

#[derive(Debug, Clone)]
/// Errors that can happen during deserialization
pub enum DeserializeError {
    /// The file is reasonable but the endianess is wrong.
    EndiannessError,
    /// Some field is not properly aligned. This can be either a serialization
    /// bug or a collision in the type hash.
    AlignmentError,
    /// The file was serialized with a version of epserde that is not compatible
    MajorVersionMismatch(u32),
    /// The file was serialized with a compatible, but too new version of epserde
    /// so we might be missing features.
    MinorVersionMismatch(u32),
    /// The magic number is wrong. The file is not an epserde file.
    MagicNumberError(u64),
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
            Self::AlignmentError => write!(f, "Alignment Error"),
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

/// We have to know the offset from the start to compute the padding to skip
/// and then check that the pointer is aligned to the type.
///
/// This is not [`std::io::Cursor`] because there is no `no_std` equivalent.
#[derive(Debug)]
pub struct Cursor<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> Cursor<'a> {
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
