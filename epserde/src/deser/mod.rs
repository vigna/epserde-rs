/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Deserialization traits and types

[`Deserialize`] is the main deserialization trait, providing methods
[`Deserialize::deserialize_eps`] and [`Deserialize::deserialize_full`]
which implement ε-copy and full-copy deserialization, respectively.
The implementation of this trait is based on [`DeserializeInner`],
which is automatically derived with `#[derive(Deserialize)]`.

*/

use crate::traits::*;
use crate::{MAGIC, MAGIC_REV, VERSION};
use core::mem::align_of;
use core::ptr::addr_of_mut;
use core::{hash::Hasher, mem::MaybeUninit};
use std::{io::BufReader, path::Path};

pub mod helpers;
pub use helpers::*;
pub mod mem_case;
pub use mem_case::*;
pub mod read;
pub use read::*;
pub mod reader_with_pos;
pub use reader_with_pos::*;
pub mod slice_with_pos;
pub use slice_with_pos::*;

pub type Result<T> = core::result::Result<T, Error>;

/// A shorthand for the [deserialized type associated with a type](DeserializeInner::DeserType).
pub type DeserType<'a, T> = <T as DeserializeInner>::DeserType<'a>;

/// Main deserialization trait. It is separated from [`DeserializeInner`] to
/// avoid that the user modify its behavior, and hide internal serialization
/// methods.
///
/// It provides several convenience methods to load or map into memory
/// structures that have been previously serialized. See, for example,
/// [`Deserialize::load_full`], [`Deserialize::load_mem`], and
/// [`Deserialize::mmap`].
///
/// # Safety
///
/// All deserialization methods are unsafe.
///
/// - No validation is performed on zero-copy types. For example, by altering a
///   serialized form you can deserialize a vector of
///   [`NonZeroUsize`](core::num::NonZeroUsize) containing zeros.
/// - The code assume that the [`read_exact`](ReadNoStd::read_exact) method of
///   the backend does not read the buffer. If the method reads the buffer, it
///   will cause undefined behavior. This is a general issue with Rust as the
///   I/O traits were written before [`MaybeUninit`] was stabilized.
/// - Malicious [`TypeHash`]/[`AlignHash`] implementations maybe lead to read
///   incompatible structures using the same code, or cause undefined behavior
///   by loading data with an incorrect alignment.
/// - Memory-mapped files might be modified externally.
pub trait Deserialize: DeserializeInner {
    /// Fully deserialize a structure of this type from the given backend.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    unsafe fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self>;
    /// ε-copy deserialize a structure of this type from the given backend.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    unsafe fn deserialize_eps(backend: &'_ [u8]) -> Result<Self::DeserType<'_>>;

    /// Convenience method to fully deserialize from a file.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    unsafe fn load_full(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;
        let mut buf_reader = BufReader::new(file);
        Self::deserialize_full(&mut buf_reader).map_err(|e| e.into())
    }

    /// Load a file into heap-allocated memory and ε-deserialize a data structure from it,
    /// returning a [`MemCase`] containing the data structure and the
    /// memory. Excess bytes are zeroed out.
    ///
    /// The allocated memory will have [`MemoryAlignment`] as alignment: types with
    /// a higher alignment requirement will cause an [alignment error](`Error::AlignmentError`).
    ///
    /// # Safety
    ///
    /// See the [trait documentation](DeserializeInner).
    unsafe fn load_mem<'a>(
        path: impl AsRef<Path>,
    ) -> anyhow::Result<MemCase<<Self as DeserializeInner>::DeserType<'a>>> {
        let align_to = align_of::<MemoryAlignment>();
        if align_of::<Self>() > align_to {
            return Err(Error::AlignmentError.into());
        }
        let file_len = path.as_ref().metadata()?.len() as usize;
        let mut file = std::fs::File::open(path)?;
        // Round up to u128 size
        let capacity = file_len + crate::pad_align_to(file_len, align_to);

        let mut uninit: MaybeUninit<MemCase<<Self as DeserializeInner>::DeserType<'_>>> =
            MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        // SAFETY: the entire vector will be filled with data read from the file,
        // or with zeroes if the file is shorter than the vector.
        #[allow(invalid_value)]
        let mut aligned_vec = unsafe {
            <Vec<MemoryAlignment>>::from_raw_parts(
                std::alloc::alloc(std::alloc::Layout::from_size_align(capacity, align_to)?)
                    as *mut MemoryAlignment,
                capacity / align_to,
                capacity / align_to,
            )
        };

        let bytes = unsafe {
            core::slice::from_raw_parts_mut(aligned_vec.as_mut_ptr() as *mut u8, capacity)
        };

        file.read_exact(&mut bytes[..file_len])?;
        // Fixes the last few bytes to guarantee zero-extension semantics
        // for bit vectors and full-vector initialization.
        bytes[file_len..].fill(0);

        // SAFETY: the vector is aligned to 16 bytes.
        let backend = MemBackend::Memory(aligned_vec.into_boxed_slice());

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(backend);
        }
        // deserialize the data structure
        let mem = unsafe { (*ptr).1.as_ref().unwrap() };
        let s = Self::deserialize_eps(mem)?;
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(s);
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }

    /// Load a file into `mmap()`-allocated memory and ε-deserialize a data structure from it,
    /// returning a [`MemCase`] containing the data structure and the
    /// memory. Excess bytes are zeroed out.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
    /// just pass `Flags::empty()`.
    ///
    /// Requires the `mmap` feature.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    #[cfg(feature = "mmap")]
    unsafe fn load_mmap<'a>(
        path: impl AsRef<Path>,
        flags: Flags,
    ) -> anyhow::Result<MemCase<<Self as DeserializeInner>::DeserType<'a>>> {
        let file_len = path.as_ref().metadata()?.len() as usize;
        let mut file = std::fs::File::open(path)?;
        let capacity = file_len + crate::pad_align_to(file_len, 16);

        let mut uninit: MaybeUninit<MemCase<<Self as DeserializeInner>::DeserType<'_>>> =
            MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        let mut mmap = mmap_rs::MmapOptions::new(capacity)?
            .with_flags(flags.mmap_flags())
            .map_mut()?;
        file.read_exact(&mut mmap[..file_len])?;
        // Fixes the last few bytes to guarantee zero-extension semantics
        // for bit vectors.
        mmap[file_len..].fill(0);

        let backend = MemBackend::Mmap(mmap.make_read_only().map_err(|(_, err)| err)?);

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(backend);
        }
        // deserialize the data structure
        let mem = unsafe { (*ptr).1.as_ref().unwrap() };
        let s = Self::deserialize_eps(mem)?;
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(s);
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }

    /// Memory map a file and ε-deserialize a data structure from it,
    /// returning a [`MemCase`] containing the data structure and the
    /// memory mapping.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
    /// just pass `Flags::empty()`.
    ///
    /// Requires the `mmap` feature.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    #[cfg(feature = "mmap")]
    unsafe fn mmap<'a>(
        path: impl AsRef<Path>,
        flags: Flags,
    ) -> anyhow::Result<MemCase<<Self as DeserializeInner>::DeserType<'a>>> {
        let file_len = path.as_ref().metadata()?.len();
        let file = std::fs::File::open(path)?;

        let mut uninit: MaybeUninit<MemCase<<Self as DeserializeInner>::DeserType<'_>>> =
            MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        let mmap = unsafe {
            mmap_rs::MmapOptions::new(file_len as _)?
                .with_flags(flags.mmap_flags())
                .with_file(&file, 0)
                .map()?
        };

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(MemBackend::Mmap(mmap));
        }

        let mmap = unsafe { (*ptr).1.as_ref().unwrap() };
        // deserialize the data structure
        let s = Self::deserialize_eps(mmap)?;
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(s);
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }
}

/// Inner trait to implement deserialization of a type. This trait exists
/// to separate the user-facing [`Deserialize`] trait from the low-level
/// deserialization mechanisms of [`DeserializeInner::_deserialize_full_inner`]
/// and [`DeserializeInner::_deserialize_eps_inner`]. Moreover,
/// it makes it possible to behave slightly differently at the top
/// of the recursion tree (e.g., to check the endianness marker), and to prevent
/// the user from modifying the methods in [`Deserialize`].
///
/// The user should not implement this trait directly, but rather derive it.
pub trait DeserializeInner: Sized {
    /// The deserialization type associated with this type. It can be
    /// retrieved conveniently with the alias [`DeserType`].
    type DeserType<'a>;
    fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> Result<Self>;

    fn _deserialize_eps_inner<'a>(backend: &mut SliceWithPos<'a>) -> Result<Self::DeserType<'a>>;
}

/// Blanket implementation that prevents the user from overwriting the
/// methods in [`Deserialize`].
///
/// This implementation [checks the header](`check_header`) written
/// by the blanket implementation of [`crate::ser::Serialize`] and then delegates to
/// [`DeserializeInner::_deserialize_full_inner`] or
/// [`DeserializeInner::_deserialize_eps_inner`].
impl<T: TypeHash + AlignHash + DeserializeInner> Deserialize for T {
    unsafe fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self> {
        let mut backend = ReaderWithPos::new(backend);
        check_header::<Self>(&mut backend)?;
        Self::_deserialize_full_inner(&mut backend)
    }

    unsafe fn deserialize_eps(backend: &'_ [u8]) -> Result<Self::DeserType<'_>> {
        let mut backend = SliceWithPos::new(backend);
        check_header::<Self>(&mut backend)?;
        Self::_deserialize_eps_inner(&mut backend)
    }
}

/// Common header check code for both ε-copy and full-copy deserialization.
///
/// Must be kept in sync with [`crate::ser::write_header`].
pub fn check_header<T: Deserialize + TypeHash + AlignHash>(
    backend: &mut impl ReadWithPos,
) -> Result<()> {
    let self_type_name = core::any::type_name::<T>().to_string();
    let mut type_hasher = xxhash_rust::xxh3::Xxh3::new();
    T::type_hash(&mut type_hasher);
    let self_type_hash = type_hasher.finish();

    let mut align_hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut offset_of = 0;
    T::align_hash(&mut align_hasher, &mut offset_of);
    let self_align_hash = align_hasher.finish();

    let magic = u64::_deserialize_full_inner(backend)?;
    match magic {
        MAGIC => Ok(()),
        MAGIC_REV => Err(Error::EndiannessError),
        magic => Err(Error::MagicCookieError(magic)),
    }?;

    let major = u16::_deserialize_full_inner(backend)?;
    if major != VERSION.0 {
        return Err(Error::MajorVersionMismatch(major));
    }
    let minor = u16::_deserialize_full_inner(backend)?;
    if minor > VERSION.1 {
        return Err(Error::MinorVersionMismatch(minor));
    };

    let usize_size = u8::_deserialize_full_inner(backend)?;
    let usize_size = usize_size as usize;
    let native_usize_size = core::mem::size_of::<usize>();
    if usize_size != native_usize_size {
        return Err(Error::UsizeSizeMismatch(usize_size));
    };

    let ser_type_hash = u64::_deserialize_full_inner(backend)?;
    let ser_align_hash = u64::_deserialize_full_inner(backend)?;
    let ser_type_name = String::_deserialize_full_inner(backend)?;

    if ser_type_hash != self_type_hash {
        return Err(Error::WrongTypeHash {
            self_type_name,
            self_type_hash,
            ser_type_name,
            ser_type_hash,
        });
    }
    if ser_align_hash != self_align_hash {
        return Err(Error::WrongAlignHash {
            self_type_name,
            self_align_hash,
            ser_type_name,
            ser_align_hash,
        });
    }

    Ok(())
}

/// A helper trait that makes it possible to implement differently
/// deserialization for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types.
/// See [`crate::traits::CopyType`] for more information.
pub trait DeserializeHelper<T: CopySelector> {
    type FullType;
    type DeserType<'a>;

    fn _deserialize_full_inner_impl(backend: &mut impl ReadWithPos) -> Result<Self::FullType>;

    fn _deserialize_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> Result<Self::DeserType<'a>>;
}

#[derive(thiserror::Error, Debug)]
/// Errors that can happen during deserialization.
pub enum Error {
    #[error("Error reading stats for file during ε-serde deserialization: {0}")]
    /// [`Deserialize::load_full`] could not open the provided file.
    FileOpenError(std::io::Error),
    #[error("Read error during ε-serde deserialization")]
    /// The underlying reader returned an error.
    ReadError,
    /// The file is from ε-serde but the endianness is wrong.
    #[cfg_attr(
        target_endian = "big",
        error("The current arch is big-endian but the data is little-endian.")
    )]
    #[cfg_attr(
        target_endian = "little",
        error("The current arch is little-endian but the data is big-endian.")
    )]
    EndiannessError,
    #[error("Alignment error. Most likely you are deserializing from a memory region with insufficient alignment.")]
    /// Some fields are not properly aligned.
    AlignmentError,
    #[error("Major version mismatch. Expected {major} but got {0}.", major = VERSION.0)]
    /// The file was serialized with a version of ε-serde that is not compatible.
    MajorVersionMismatch(u16),
    #[error("Minor version mismatch. Expected {minor} but got {0}.", minor = VERSION.1)]
    /// The file was serialized with a compatible, but too new version of ε-serde
    /// so we might be missing features.
    MinorVersionMismatch(u16),
    #[error("The file was serialized on an architecture where a usize has size {0}, but on the current architecture it has size {size}.", size = core::mem::size_of::<usize>())]
    /// The pointer width of the serialized file is different from the pointer
    /// width of the current architecture. For example, the file was serialized
    /// on a 64-bit machine and we are trying to deserialize it on a 32-bit
    /// machine.
    UsizeSizeMismatch(usize),
    #[error("Wrong magic cookie 0x{0:016x}. The byte stream does not come from ε-serde.")]
    /// The magic cookie is wrong. The byte sequence does not come from ε-serde.
    MagicCookieError(u64),
    #[error("Invalid tag: 0x{0:02x}")]
    /// A tag is wrong (e.g., for [`Option`]).
    InvalidTag(usize),
    #[error(
        r#"Wrong type hash: actual = 0x{ser_type_hash:016x}, expected = 0x{self_type_hash:016x}.
You are trying to deserialize a file with the wrong type. You might also be trying to deserialize
a structure containing tuples that was serialized before 0.9.0. The serialized type is '{ser_type_name}', 
but the type on which the deserialization method was invoked is '{self_type_name}'."#
    )]
    /// The type hash is wrong. Probably the user is trying to deserialize a
    /// file with the wrong type.
    WrongTypeHash {
        // The name of the type that was serialized.
        ser_type_name: String,
        // The [`TypeHash`] of the type that was serialized.
        ser_type_hash: u64,
        // The name of the type on which the deserialization method was called.
        self_type_name: String,
        // The [`TypeHash`] of the type on which the deserialization method was called.
        self_type_hash: u64,
    },
    #[error(
r#"Wrong alignment hash: actual = 0x{ser_align_hash:016x}, expected = 0x{self_align_hash:016x}.
You might be trying to deserialize a file that was serialized on an architecture 
with different alignment requirements, or some of the fields of the type 
might have changed their copy type (zero or deep). You might also be trying to deserialize a 
tuple of mixed zero-copy types, which is no longer supported since 0.8.0, or to 
deserialize an array, whose alignment hash has been fixed in 0.8.0. 
The serialized type is '{ser_type_name}',  but the type on which the the deserialization
method was invoked is '{self_type_name}'."#
    )]
    /// The type representation hash is wrong. Probably the user is trying to
    /// deserialize a file with some zero-copy type that has different
    /// in-memory representations on the serialization arch and on the current one,
    /// usually because of alignment issues. There are also some backward-compatibility
    /// issues discussed in the error message.
    WrongAlignHash {
        // The name of the type that was serialized.
        ser_type_name: String,
        // The [`AlignHash`] of the type that was serialized.
        ser_align_hash: u64,
        // The name of the type on which the deserialization method was called.
        self_type_name: String,
        // The [`AlignHash`] of the type on which the deserialization method was called.
        self_align_hash: u64,
    },
}
