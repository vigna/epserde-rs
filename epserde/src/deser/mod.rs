/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Deserialization traits and types
//!
//! [`Deserialize`] is the main deserialization trait, providing methods
//! [`Deserialize::deserialize_eps`] and [`Deserialize::deserialize_full`] which
//! implement ε-copy and full-copy deserialization, respectively. The
//! implementation of this trait is based on [`DeserInner`], which is
//! automatically derived with `#[derive(Epserde)]`.

use crate::ser::SerInner;
use crate::traits::*;
use crate::{MAGIC, MAGIC_REV, VERSION};
use core::hash::Hasher;
use core::{mem::MaybeUninit, ptr::addr_of_mut};

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

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{io::BufReader, path::Path};

pub type Result<T> = core::result::Result<T, Error>;

/// A shorthand for the [deserialization type associated with a deserializable
/// type](DeserInner::DeserType).
pub type DeserType<'a, T> = <T as DeserInner>::DeserType<'a>;

/// A covariance witness type used by
/// [`DeserInner::__check_covariance`].
///
/// This newtype wraps a [`PhantomData<fn() -> T>`](core::marker::PhantomData),
/// making it covariant in `T` and zero-sized. Its field is private, so it
/// cannot be constructed outside of the `epserde` crate. This ensures that any
/// returning bypass of the covariance check requires `unsafe` code.
#[doc(hidden)]
pub struct CovariantProof<T>(core::marker::PhantomData<fn() -> T>);

impl<T> CovariantProof<T> {
    /// Constructs a new [`CovariantProof`].
    ///
    /// This constructor is `pub(crate)` so that only code within `epserde`
    /// can create instances (e.g., in [`MemCase::uncase`](super::mem_case::MemCase::uncase)).
    #[doc(hidden)]
    pub(crate) fn new() -> Self {
        CovariantProof(core::marker::PhantomData)
    }
}

/// Calls [`DeserInner::__check_covariance`] on `T`, detecting non-returning
/// implementations at runtime.
///
/// This function is used by the derive macro to verify field-level covariance
/// from generated code without exposing [`CovariantProof::new`].
#[doc(hidden)]
pub fn __check_field_covariance<T: DeserInner>() {
    let _ = T::__check_covariance(CovariantProof::<T::DeserType<'static>>::new());
}

/// Main deserialization trait. It is separated from [`DeserInner`] to
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
///
/// The first problem can be solved by traits like
/// [`FromByte`](https://docs.rs/zerocopy/latest/zerocopy/trait.FromBytes.html).
/// The second issue is a non-problem with the standard library.
///
/// The last two issues are more an issue of security than undefined behavior,
/// but that is in the eye of the beholder.
pub trait Deserialize: DeserInner {
    /// Fully deserializes a structure of this type from the given backend.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    unsafe fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self>;
    /// ε-copy deserializes a structure of this type from the given backend.
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
    #[cfg(feature = "std")]
    unsafe fn load_full(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;
        let mut buf_reader = BufReader::new(file);
        unsafe { Self::deserialize_full(&mut buf_reader).map_err(|e| e.into()) }
    }

    /// Reads data from a reader into heap-allocated memory and ε-deserialize a
    /// data structure from it, returning a [`MemCase`] containing the data
    /// structure and the memory. Excess bytes are zeroed out.
    ///
    /// The allocated memory will have [`MemoryAlignment`] as alignment: types
    /// with a higher alignment requirement will cause an [alignment
    /// error](`Error::AlignmentError`).
    ///
    /// For a version using a file path, see [`load_mem`](Self::load_mem).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use epserde::prelude::*;
    /// let data = vec![1, 2, 3, 4, 5];
    /// let mut buffer = Vec::new();
    /// unsafe { data.serialize(&mut buffer)? };
    ///
    /// let cursor = <AlignedCursor>::from_slice(&buffer);
    /// let mem_case = unsafe { <Vec<i32>>::read_mem(cursor, buffer.len())? };
    /// assert_eq!(data, **mem_case.uncase());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    unsafe fn read_mem(mut read: impl ReadNoStd, size: usize) -> anyhow::Result<MemCase<Self>> {
        let align_to = align_of::<MemoryAlignment>();
        if align_of::<Self>() > align_to {
            return Err(Error::AlignmentError.into());
        }
        // Round up to u128 size
        let capacity = size + crate::pad_align_to(size, align_to);

        let mut uninit: MaybeUninit<MemCase<Self>> = MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        // SAFETY: the entire vector will be filled with data read from the reader,
        // or with zeroes if the reader provides less data than expected.
        #[allow(invalid_value)]
        let mut aligned_vec = unsafe {
            #[cfg(not(feature = "std"))]
            let alloc_func = alloc::alloc::alloc;
            #[cfg(feature = "std")]
            let alloc_func = std::alloc::alloc;

            <Vec<MemoryAlignment>>::from_raw_parts(
                alloc_func(core::alloc::Layout::from_size_align(capacity, align_to)?)
                    as *mut MemoryAlignment,
                capacity / align_to,
                capacity / align_to,
            )
        };

        let bytes = unsafe {
            core::slice::from_raw_parts_mut(aligned_vec.as_mut_ptr() as *mut u8, capacity)
        };

        read.read_exact(&mut bytes[..size])?;
        // Fixes the last few bytes to guarantee zero-extension semantics
        // for bit vectors and full-vector initialization.
        bytes[size..].fill(0);

        // SAFETY: the vector is aligned to 64 bytes.
        let backend = MemBackend::Memory(aligned_vec.into_boxed_slice());

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(backend);
        }
        // deserialize the data structure
        let mem = unsafe { (*ptr).1.as_ref().unwrap() };
        let s = unsafe { Self::deserialize_eps(mem) }?;
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(s);
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }

    /// Loads a file into heap-allocated memory and ε-deserialize a data
    /// structure from it, returning a [`MemCase`] containing the data structure
    /// and the memory. Excess bytes are zeroed out.
    ///
    /// The allocated memory will have [`MemoryAlignment`] as alignment: types
    /// with a higher alignment requirement will cause an [alignment
    /// error](`Error::AlignmentError`).
    ///
    /// For a version using a generic [`std::io::Read`], see
    /// [`read_mem`](Self::read_mem).
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    #[cfg(feature = "std")]
    unsafe fn load_mem(path: impl AsRef<Path>) -> anyhow::Result<MemCase<Self>> {
        let file_len = path.as_ref().metadata()?.len() as usize;
        let file = std::fs::File::open(path)?;
        unsafe { Self::read_mem(file, file_len) }
    }

    /// Reads data from a reader into `mmap()`-allocated memory and ε-deserialize
    /// a data structure from it, returning a [`MemCase`] containing the data
    /// structure and the memory. Excess bytes are zeroed out.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`];
    /// otherwise, just pass `Flags::empty()`.
    ///
    /// For a version using a file path, see [`load_mmap`](Self::load_mmap).
    ///
    /// Requires the `mmap` feature.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "mmap")]
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use epserde::prelude::*;
    /// # use std::io::Cursor;
    /// let data = vec![1, 2, 3, 4, 5];
    /// let mut buffer = Vec::new();
    /// unsafe { data.serialize(&mut buffer)? };
    ///
    /// let cursor = Cursor::new(&buffer);
    /// let mmap_case = unsafe { <Vec<i32>>::read_mmap(cursor, buffer.len(), Flags::empty())? };
    /// assert_eq!(data, **mmap_case.uncase());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize).
    #[cfg(feature = "mmap")]
    unsafe fn read_mmap(
        mut read: impl ReadNoStd,
        size: usize,
        flags: Flags,
    ) -> anyhow::Result<MemCase<Self>> {
        let capacity = size + crate::pad_align_to(size, 16);

        let mut uninit: MaybeUninit<MemCase<Self>> = MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        let mut mmap = mmap_rs::MmapOptions::new(capacity)?
            .with_flags(flags.mmap_flags())
            .map_mut()?;
        read.read_exact(&mut mmap[..size])?;
        // Fixes the last few bytes to guarantee zero-extension semantics
        // for bit vectors.
        mmap[size..].fill(0);

        let backend = MemBackend::Mmap(mmap.make_read_only().map_err(|(_, err)| err)?);

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(backend);
        }
        // deserialize the data structure
        let mem = unsafe { (*ptr).1.as_ref().unwrap() };
        let s = unsafe { Self::deserialize_eps(mem) }?;
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(s);
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }

    /// Loads a file into `mmap()`-allocated memory and ε-deserialize a data
    /// structure from it, returning a [`MemCase`] containing the data structure
    /// and the memory. Excess bytes are zeroed out.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`];
    /// otherwise, just pass `Flags::empty()`.
    ///
    /// For a version using a generic [`std::io::Read`], see
    /// [`read_mmap`](Self::read_mmap).
    ///
    /// Requires the `mmap` feature.
    ///
    /// # Safety
    ///
    /// See the [trait documentation](Deserialize) and [mmap's `with_file`'s
    /// documentation](mmap_rs::MmapOptions::with_file).
    #[cfg(all(feature = "mmap", feature = "std"))]
    unsafe fn load_mmap(path: impl AsRef<Path>, flags: Flags) -> anyhow::Result<MemCase<Self>> {
        let file_len = path.as_ref().metadata()?.len() as usize;
        let file = std::fs::File::open(path)?;
        unsafe { Self::read_mmap(file, file_len, flags) }
    }

    /// Memory maps a file and ε-deserializes a data structure from it,
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
    /// See the [trait documentation](Deserialize) and [mmap's `with_file`'s documentation](mmap_rs::MmapOptions::with_file).
    #[cfg(all(feature = "mmap", feature = "std"))]
    unsafe fn mmap(path: impl AsRef<Path>, flags: Flags) -> anyhow::Result<MemCase<Self>> {
        let file_len = path.as_ref().metadata()?.len();
        let file = std::fs::File::open(path)?;

        let mut uninit: MaybeUninit<MemCase<Self>> = MaybeUninit::uninit();
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
        let s = unsafe { Self::deserialize_eps(mmap) }?;
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(s);
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }
}

#[allow(clippy::missing_safety_doc)] // Clippy bug
/// Inner trait to implement deserialization of a type. This trait exists to
/// separate the user-facing [`Deserialize`] trait from the low-level
/// deserialization mechanisms of [`DeserInner::_deser_full_inner`]
/// and [`DeserInner::_deser_eps_inner`]. Moreover, it makes it
/// possible to behave slightly differently at the top of the recursion tree
/// (e.g., to check the endianness marker), and to prevent the user from
/// modifying the methods in [`Deserialize`].
///
/// The user should not implement this trait directly, but rather derive it.
///
/// # Safety
///
/// See [`Deserialize`].
pub trait DeserInner: Sized {
    /// The deserialization type associated with this type. It can be retrieved
    /// conveniently with the alias [`DeserType`].
    type DeserType<'a>;

    /// Compile-time covariance check for [`DeserType`].
    ///
    /// [`MemCase::uncase`](super::mem_case::MemCase::uncase) transmutes
    /// `DeserType<'static, S>` to `DeserType<'a, S>`, which is only sound if
    /// `DeserType` is covariant in its lifetime parameter.
    ///
    /// This method enforces that invariant: the only safe implementation is
    /// `{ p }`, which compiles only when `DeserType` is covariant. For generic
    /// container types where the compiler cannot see through associated-type
    /// projections, the body must be `unsafe { core::mem::transmute(p) }` with
    /// a SAFETY comment justifying covariance compositionally.
    ///
    /// The parameter and return types use [`CovariantProof`], a newtype with a
    /// private field that cannot be constructed outside of this crate. This
    /// ensures that any returning bypass of the covariance check requires
    /// `unsafe` code. Moreover,
    /// [`MemCase::uncase`](super::mem_case::MemCase::uncase) actually calls
    /// this method, so non-returning implementations (`todo!()`, `panic!()`,
    /// `unimplemented!()|, `loop {}`, etc.) are detected at runtime.
    #[doc(hidden)]
    fn __check_covariance<'__long: '__short, '__short>(
        p: CovariantProof<Self::DeserType<'__long>>,
    ) -> CovariantProof<Self::DeserType<'__short>>;

    /// # Safety
    ///
    /// See the documentation of [`Deserialize`].
    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> Result<Self>;

    /// # Safety
    ///
    /// See the documentation of [`Deserialize`].
    unsafe fn _deser_eps_inner<'a>(backend: &mut SliceWithPos<'a>) -> Result<Self::DeserType<'a>>;
}

/// Blanket implementation that prevents the user from overwriting the
/// methods in [`Deserialize`].
///
/// This implementation [checks the header](`check_header`) written
/// by the blanket implementation of [`crate::ser::Serialize`] and then delegates to
/// [`DeserInner::_deser_full_inner`] or
/// [`DeserInner::_deser_eps_inner`].
impl<T: SerInner<SerType: TypeHash + AlignHash> + DeserInner> Deserialize for T {
    /// # Safety
    ///
    /// See the documentation of [`Deserialize`].
    unsafe fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self> {
        let mut backend = ReaderWithPos::new(backend);
        check_header::<Self>(&mut backend)?;
        unsafe { Self::_deser_full_inner(&mut backend) }
    }

    /// # Safety
    ///
    /// See the documentation of [`Deserialize`].
    unsafe fn deserialize_eps(backend: &'_ [u8]) -> Result<Self::DeserType<'_>> {
        let mut backend = SliceWithPos::new(backend);
        check_header::<Self>(&mut backend)?;
        unsafe { Self::_deser_eps_inner(&mut backend) }
    }
}

/// Common header check code for both ε-copy and full-copy deserialization.
///
/// Must be kept in sync with [`crate::ser::write_header`].
pub fn check_header<T: SerInner<SerType: TypeHash + AlignHash>>(
    backend: &mut impl ReadWithPos,
) -> Result<()> {
    let self_type_name = core::any::type_name::<T>().to_string();
    let self_ser_type_name = core::any::type_name::<T::SerType>().to_string();
    let mut type_hasher = xxhash_rust::xxh3::Xxh3::new();
    T::SerType::type_hash(&mut type_hasher);
    let self_type_hash = type_hasher.finish();

    let mut align_hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut offset_of = 0;
    T::SerType::align_hash(&mut align_hasher, &mut offset_of);
    let self_align_hash = align_hasher.finish();

    let magic = unsafe { u64::_deser_full_inner(backend)? };
    match magic {
        MAGIC => Ok(()),
        MAGIC_REV => Err(Error::EndiannessError),
        magic => Err(Error::MagicCookieError(magic)),
    }?;

    let major = unsafe { u16::_deser_full_inner(backend)? };
    if major != VERSION.0 {
        return Err(Error::MajorVersionMismatch(major));
    }
    let minor = unsafe { u16::_deser_full_inner(backend)? };
    if minor > VERSION.1 {
        return Err(Error::MinorVersionMismatch(minor));
    };

    let usize_size = unsafe { u8::_deser_full_inner(backend)? };
    let usize_size = usize_size as usize;
    let native_usize_size = core::mem::size_of::<usize>();
    if usize_size != native_usize_size {
        return Err(Error::UsizeSizeMismatch(usize_size));
    };

    let ser_type_hash = unsafe { u64::_deser_full_inner(backend)? };
    let ser_align_hash = unsafe { u64::_deser_full_inner(backend)? };
    let ser_type_name = unsafe { String::_deser_full_inner(backend)? }.to_string();

    if ser_type_hash != self_type_hash {
        return Err(Error::WrongTypeHash {
            ser_type_name,
            ser_type_hash,
            self_type_name,
            self_ser_type_name,
            self_type_hash,
        });
    }
    if ser_align_hash != self_align_hash {
        return Err(Error::WrongAlignHash {
            ser_type_name,
            ser_align_hash,
            self_type_name,
            self_ser_type_name,
            self_align_hash,
        });
    }

    Ok(())
}

/// A helper trait that makes it possible to implement differently
/// deserialization for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types.
/// See [`crate::traits::CopyType`] for more information.
pub trait DeserHelper<T: CopySelector> {
    type FullType;
    type DeserType<'a>;

    /// # Safety
    ///
    /// See the documentation of [`Deserialize`].
    unsafe fn _deser_full_inner_impl(backend: &mut impl ReadWithPos) -> Result<Self::FullType>;

    /// # Safety
    ///
    /// See the documentation of [`Deserialize`].
    unsafe fn _deser_eps_inner_impl<'a>(
        backend: &mut SliceWithPos<'a>,
    ) -> Result<Self::DeserType<'a>>;
}

#[derive(thiserror::Error, Debug)]
/// Errors that can happen during deserialization.
pub enum Error {
    #[error("Error reading stats for file during ε-serde deserialization: {0}")]
    /// [`Deserialize::load_full`] could not open the provided file.
    #[cfg(feature = "std")]
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
    #[error(
        "Alignment error. Most likely you are deserializing from a memory region with insufficient alignment."
    )]
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
        r#"Wrong type hash
Actual: 0x{ser_type_hash:016x}; expected: 0x{self_type_hash:016x}.

The serialized type is
    '{ser_type_name}',
but the deserializable type on which the deserialization method was invoked is
    '{self_type_name}',
which has serialization type
    {self_ser_type_name}.

You are trying to deserialize a file with the wrong type. You might also be
trying to deserialize a tuple of mixed zero-copy types, which is no longer
supported since 0.8.0, an instance containing tuples, whose type hash was fixed
in 0.9.0, or an instance containing a vector or a string that was serialized
before 0.10.0."#
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
        // The name of the serialization type of `self_type_name`.
        self_ser_type_name: String,
        // The [`TypeHash`] of the type on which the deserialization method was called.
        self_type_hash: u64,
    },
    #[error(
        r#"Wrong alignment hash
Actual: 0x{ser_align_hash:016x}; expected: 0x{self_align_hash:016x}.

The serialized type is
    '{ser_type_name}',
but the deserializable type on which the deserialization method was invoked is
    '{self_type_name}',
which has serialization type
    {self_ser_type_name}.

You might be trying to deserialize a file that was serialized on an
architecture with different alignment requirements, or some of the fields of
the type might have changed their copy type (zero or deep). You might also be
trying to deserialize an array, whose alignment hash has been fixed in 0.8.0.
It is also possible that you are trying to deserialize a file serialized before
version 0.10.0 in which repr attributes were not sorted lexicographically, or
a range in a file serialized before version 0.12.0."#
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
        // The name of the serialization type of `self_type_name`.
        self_ser_type_name: String,
        // The [`AlignHash`] of the type on which the deserialization method was called.
        self_align_hash: u64,
    },
}
