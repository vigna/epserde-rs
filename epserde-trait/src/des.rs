use crate::{MAGIC, MAGIC_REV, VERSION};

pub trait DeserializeNature: Sized {
    const ZC_TYPE: bool;
    const ZC_SUB_TYPE: bool;

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

fn check_header<'a>(backend: &'a [u8]) -> Result<&'a [u8], DeserializeError> {
    let (magic, backend) = u64::deserialize_full_copy_inner(backend)?;
    match magic {
        MAGIC => Ok(backend),
        MAGIC_REV => Err(DeserializeError::EndiannessError),
        magic => Err(DeserializeError::MagicNumberError(magic)),
    }?;
    let (major, backend) = u32::deserialize_full_copy_inner(backend)?;
    let (minor, backend) = u32::deserialize_full_copy_inner(backend)?;
    if major != VERSION.0 {
        return Err(DeserializeError::MajorVersionMismatch(major));
    }
    if minor > VERSION.1 {
        return Err(DeserializeError::MinorVersionMismatch(minor));
    };
    Ok(backend)
}

/// The inner trait to implement ZeroCopy Deserialization
///
/// STN is the SubTypeNature this only applies to compund types and is used
/// to have different implementations for different subtypes
/// (e.g., Vec<u8> and Vec<Vec<u8>>).
///
/// This shouldn't be needed as we can do `impl<T: DeserializeNatrue<FullCopy>>`
/// and `impl<T: DeserializeNatrue<ZeroCopy>>`
/// but rust is not smart enough to figure out that the impl is not in conflict
pub trait DeserializeZeroCopy<const ZC_SUB_TYPE: bool>: DeserializeNature {
    type DesType<'b>;

    fn deserialize_zero_copy<'a>(
        mut backend: &'a [u8],
    ) -> Result<Self::DesType<'a>, DeserializeError> {
        backend = check_header(backend)?;
        let (res, _) = Self::deserialize_zero_copy_inner(backend)?;
        Ok(res)
    }

    fn deserialize_zero_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self::DesType<'a>, &'a [u8]), DeserializeError>;
}

/// The inner trait to implement FullCopy Deserialization
pub trait DeserializeFullCopy: DeserializeNature {
    fn deserialize_full_copy<'a>(mut backend: &'a [u8]) -> Result<Self, DeserializeError> {
        backend = check_header(backend)?;
        let (res, _) = Self::deserialize_full_copy_inner(backend)?;
        Ok(res)
    }

    fn deserialize_full_copy_inner<'a>(
        backend: &'a [u8],
    ) -> Result<(Self, &'a [u8]), DeserializeError>;
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
