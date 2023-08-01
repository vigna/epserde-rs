use crate::{MAGIC, MAGIC_REV, VERSION};

pub mod des_impl;
pub use des_impl::*;

/// <https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html>
pub struct DesWrap<T: ?Sized> {
    _phantom: core::marker::PhantomData<T>,
}

pub trait Deserialize: Sized {
    fn deserialize(backend: &[u8]) -> Result<Self, DeserializeError>;
}

impl<T: DeserializeInner> Deserialize for T {
    fn deserialize(backend: &[u8]) -> Result<Self, DeserializeError> {
        let mut backend = Cursor::new(backend);
        backend = check_header(backend)?;
        let (res, _) = Self::deserialize_inner(backend)?;
        Ok(res)
    }
}

pub trait DeserializeInner: Sized {
    fn deserialize_inner<'a>(backend: Cursor<'a>) -> Result<(Self, Cursor<'a>), DeserializeError>;
}

pub trait DeserializeZeroCopy
where
    for<'a> &'a &'a &'a DesWrap<Self>: DeserializeZeroCopyInner,
{
    type DesType<'b>;
    fn deserialize_zero_copy<'a>(backend: &'a [u8]) -> Result<Self::DesType<'a>, DeserializeError>;
}

impl<T: 'static> DeserializeZeroCopy for T
where
    for<'a> &'a &'a &'a DesWrap<Self>: DeserializeZeroCopyInner,
{
    type DesType<'b> = <&'b &'b &'b DesWrap<Self> as DeserializeZeroCopyInner>::DesType<'b>;

    fn deserialize_zero_copy<'a>(backend: &'a [u8]) -> Result<Self::DesType<'a>, DeserializeError> {
        let mut backend = Cursor::new(backend);
        backend = check_header(backend)?;
        let (res, _) = <&&&DesWrap<Self>>::deserialize_zc_inner(backend)?;
        Ok(res)
    }
}

/// The inner trait to implement ZeroCopy Deserialization
pub trait DeserializeZeroCopyInner {
    type DesType<'b>;

    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DesType<'a>, Cursor<'a>), DeserializeError>;
}

/// The lowest priority (3) of zero copy, is always full copy
impl<T: DeserializeInner> DeserializeZeroCopyInner for &&DesWrap<T> {
    type DesType<'b> = T;
    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> Result<(Self::DesType<'a>, Cursor<'a>), DeserializeError> {
        <T as DeserializeInner>::deserialize_inner(backend)
    }
}

fn check_header<'a>(backend: Cursor<'a>) -> Result<Cursor<'a>, DeserializeError> {
    let (magic, backend) = u64::deserialize_inner(backend)?;
    match magic {
        MAGIC => Ok(()),
        MAGIC_REV => Err(DeserializeError::EndiannessError),
        magic => Err(DeserializeError::MagicNumberError(magic)),
    }?;
    let (major, backend) = u32::deserialize_inner(backend)?;
    let (minor, backend) = u32::deserialize_inner(backend)?;
    if major != VERSION.0 {
        return Err(DeserializeError::MajorVersionMismatch(major));
    }
    if minor > VERSION.1 {
        return Err(DeserializeError::MinorVersionMismatch(minor));
    };
    Ok(backend)
}

#[derive(Debug, Clone)]
pub enum DeserializeError {
    EndiannessError,
    AlignementError,
    MajorVersionMismatch(u32),
    MinorVersionMismatch(u32),
    MagicNumberError(u64),
    WrongTypeHash { expected: u64, got: u64 },
}

impl std::error::Error for DeserializeError {}

impl std::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
            Self::AlignementError => write!(f, "Alignement Error"),
            Self::WrongTypeHash { expected, got } => {
                write!(
                    f,
                    "Wrong type hash. Expected={:016x}, Got={:016x}",
                    expected, got
                )
            }
        }
    }
}

/// We have to know the offset from the start to compute the padding to skip
/// and then check that the pointer is aligned to the type
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
