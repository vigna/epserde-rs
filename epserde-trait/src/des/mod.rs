use crate::{MAGIC, MAGIC_REV, VERSION};

pub mod des_impl;
pub use des_impl::*;

/// <https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html>
pub struct DesWrap<T: ?Sized> {
    _phantom: core::marker::PhantomData<*mut T>,
}
/// A dispatcher that hides the bound on the lifetimes
pub struct DesDis<T: ?Sized> {
    _phantom: core::marker::PhantomData<*mut T>,
}

impl<T: 'static> DeserializeZeroCopyInner for DesDis<T>
where
    for<'a, 'b, 'c> &'a &'b &'c DesWrap<T>: DeserializeZeroCopyInner,
{
    type DesType<'b> = <&'b &'b &'b DesWrap<T> as DeserializeZeroCopyInner>::DesType<'b>;

    fn deserialize_zc_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        <&'a &'a &'a DesWrap<T>>::deserialize_zc_inner(backend)
    }
}

pub trait Deserialize: Sized {
    fn deserialize(backend: &[u8]) -> Result<Self, DeserializeError>;
}

impl<T: DeserializeInner> Deserialize for T {
    fn deserialize(mut backend: &[u8]) -> Result<Self, DeserializeError> {
        backend = check_header(backend)?;
        let (res, _) = Self::deserialize_inner(backend)?;
        Ok(res)
    }
}

pub trait DeserializeInner: Sized {
    fn deserialize_inner<'a>(backend: &'a [u8]) -> Result<(Self, &'a [u8]), DeserializeError>;
}

pub trait DeserializeZeroCopy
where
    DesDis<Self>: DeserializeZeroCopyInner,
{
    type DesType<'b>;
    fn deserialize_zero_copy<'a>(backend: &'a [u8]) -> Result<Self::DesType<'a>, DeserializeError>;
}

impl<T: 'static> DeserializeZeroCopy for T
where
    DesDis<Self>: DeserializeZeroCopyInner,
{
    type DesType<'b> = <DesDis<Self> as DeserializeZeroCopyInner>::DesType<'b>;

    fn deserialize_zero_copy<'a>(
        mut backend: &'a [u8],
    ) -> Result<Self::DesType<'a>, DeserializeError> {
        backend = check_header(backend)?;
        let (res, _) = <DesDis<Self>>::deserialize_zc_inner(backend)?;
        Ok(res)
    }
}

/// The inner trait to implement ZeroCopy Deserialization
pub trait DeserializeZeroCopyInner {
    type DesType<'b>;

    fn deserialize_zc_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError>;
}

/// The lowest priority (3) of zero copy, is always full copy
impl<T: DeserializeInner> DeserializeZeroCopyInner for &&DesWrap<T> {
    type DesType<'b> = T;
    fn deserialize_zc_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError> {
        <T as DeserializeInner>::deserialize_inner(backend)
    }
}

fn check_header<'a>(backend: &'a [u8]) -> Result<&'a [u8], DeserializeError> {
    let (magic, backend) = u64::deserialize_inner(backend)?;
    match magic {
        MAGIC => Ok(backend),
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

pub trait CheckAlignement: Sized {
    /// Inner function used to check that the given slice is aligned to
    /// deserialize the current type
    fn check_alignement<'a>(backend: &'a [u8]) -> Result<(), DeserializeError> {
        if backend.as_ptr() as usize % std::mem::align_of::<Self>() != 0 {
            Err(DeserializeError::AlignementError)
        } else {
            Ok(())
        }
    }
}
impl<T: Sized> CheckAlignement for T {}
