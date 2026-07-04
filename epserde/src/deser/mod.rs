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
use aliasable::boxed::AliasableBox;
use core::hash::Hasher;
use core::{mem::MaybeUninit, ptr::addr_of_mut};
use maybe_dangling::MaybeDangling;

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

/// The result type for deserialization, using the deserialization [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// A shorthand for the [deserialization associated type].
///
/// [deserialization associated type]: DeserInner::DeserType
pub type DeserType<'a, T> = <T as DeserInner>::DeserType<'a>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// Errors that can happen during deserialization.
pub enum Error {
    /// A convenience method (e.g., [`Deserialize::load_full`]) could not open
    /// the provided file or read its metadata.
    #[cfg(feature = "std")]
    #[error("Error opening or inspecting file during ε-serde deserialization: {0}")]
    FileOpenError(#[source] std::io::Error),
    /// The underlying reader returned an error.
    #[cfg(feature = "std")]
    #[error("I/O error during ε-serde deserialization: {0}")]
    IoError(#[source] std::io::Error),
    /// There is not enough data (e.g., the serialized data is truncated).
    ///
    /// When the `std` feature is enabled, standard-library readers report
    /// end-of-file conditions through this variant and every other failure
    /// through [`IoError`].
    ///
    /// [`IoError`]: https://docs.rs/epserde/latest/epserde/deser/enum.Error.html#variant.IoError
    #[error("Read error during ε-serde deserialization")]
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
    EndiannessMismatch,
    /// Some fields are not properly aligned.
    #[error(
        "Alignment error. Most likely you are deserializing from a memory region with insufficient alignment."
    )]
    AlignmentError,
    /// The file was serialized with a version of ε-serde that is not compatible.
    #[error("Major version mismatch. Expected {major} but got {0}.", major = VERSION.0)]
    MajorVersionMismatch(u16),
    /// The file was serialized with a compatible, but too new version of ε-serde
    /// so we might be missing features.
    #[error("Minor version mismatch. Expected {minor} but got {0}.", minor = VERSION.1)]
    MinorVersionMismatch(u16),
    /// The pointer width of the serialized file is different from the pointer
    /// width of the current architecture. For example, the file was serialized
    /// on a 64-bit machine and we are trying to deserialize it on a 32-bit
    /// machine.
    #[error("The file was serialized on an architecture where a usize has size {0}, but on the current architecture it has size {size}.", size = core::mem::size_of::<usize>())]
    UsizeSizeMismatch(usize),
    /// The magic cookie is wrong. The byte sequence does not come from ε-serde.
    #[error("Wrong magic cookie 0x{0:016x}. The byte stream does not come from ε-serde.")]
    InvalidMagicCookie(u64),
    /// A tag is wrong (e.g., for [`Option`]).
    #[error("Invalid tag: 0x{0:02x}")]
    InvalidTag(usize),
    /// The type hash is wrong. Probably the user is trying to deserialize a
    /// file with the wrong type.
    #[error(
        r#"Wrong type hash
Actual: 0x{ser_type_hash:016x}; expected: 0x{self_type_hash:016x}.

The serialized type is
    {ser_type_name},
but the deserializing type on which the deserialization method was invoked is
    {self_type_name},
which has serialization type
    {self_ser_type_name}.

You are trying to deserialize a file with the wrong type."#
    )]
    TypeHashMismatch {
        /// The name of the type that was serialized.
        ser_type_name: String,
        /// The [`TypeHash`] of the type that was serialized.
        ser_type_hash: u64,
        /// The name of the type on which the deserialization method was called.
        self_type_name: String,
        /// The name of the serialization type of `self_type_name`.
        self_ser_type_name: String,
        /// The [`TypeHash`] of the type on which the deserialization method was called.
        self_type_hash: u64,
    },
    /// The alignment hash is wrong. Probably the user is trying to deserialize
    /// a file with some zero-copy type that has different in-memory
    /// representations on the serialization architecture and on the current
    /// one, usually because of alignment issues.
    #[error(
        r#"Wrong alignment hash
Actual: 0x{ser_align_hash:016x}; expected: 0x{self_align_hash:016x}.

The serialized type is
    {ser_type_name},
but the deserializing type on which the deserialization method was invoked is
    {self_type_name},
which has serialization type
    {self_ser_type_name}.

You are trying to deserialize a file that was serialized on an
architecture with incompatible alignment requirements."#
    )]
    AlignHashMismatch {
        /// The name of the type that was serialized.
        ser_type_name: String,
        /// The [`AlignHash`] of the type that was serialized.
        ser_align_hash: u64,
        /// The name of the type on which the deserialization method was called.
        self_type_name: String,
        /// The name of the serialization type of `self_type_name`.
        self_ser_type_name: String,
        /// The [`AlignHash`] of the type on which the deserialization method was called.
        self_align_hash: u64,
    },
}

/// A zero-sized type covariant in `T` whose private field prevents construction
/// outside this crate, making it impossible to implement
/// [`DeserInner::__check_covariance`] with a returning body without `unsafe`.
#[doc(hidden)]
pub struct CovariantProof<T>(core::marker::PhantomData<fn() -> T>);

impl<T> CovariantProof<T> {
    #[doc(hidden)]
    pub(crate) const fn new() -> Self {
        CovariantProof(core::marker::PhantomData)
    }
}

/// Calls [`DeserInner::__check_covariance`] on `T`, detecting non-returning
/// implementations at runtime.
///
/// This function can be used in custom implementations to verify field-level
/// covariance without accessing [`CovariantProof`]'s constructor. In
/// particular, it is used by the derive macro.
#[inline(always)]
pub fn __check_type_covariance<T: DeserInner>() {
    let _ = T::__check_covariance(CovariantProof::<T::DeserType<'static>>::new());
}

/// Implements [`DeserInner::__check_covariance`] for types whose [`DeserType`]
/// is directly covariant in its lifetime parameter (i.e., `{ proof }`
/// compiles).
///
/// Use this when [`DeserType<'a>`] does not involve any associated-type
/// projection, so the compiler can verify covariance directly. Typical cases
/// include types where `DeserType<'a>` is `Self`, `&'a Self`, or any other
/// concrete covariant type.
///
/// [`DeserType<'a>`]: DeserInner::DeserType
#[macro_export]
macro_rules! check_covariance {
    () => {
        #[inline(always)]
        fn __check_covariance<'__long: '__short, '__short>(
            proof: $crate::deser::CovariantProof<Self::DeserType<'__long>>,
        ) -> $crate::deser::CovariantProof<Self::DeserType<'__short>> {
            proof
        }
    };
}

/// Implements [`DeserInner::__check_covariance`] with an `unsafe` transmute for
/// types in which the compiler cannot see through associated-type projections.
///
/// The macro accepts a list of types whose covariance is verified by calling
/// [`__check_type_covariance`] on each, mirroring what the derive macro does
/// for structs and enums.
///
/// # Safety
///
/// The caller must ensure that the type itself is covariant in its type
/// parameters. The macro verifies that the [`DeserType`] of each listed type is
/// covariant (via its own [`__check_covariance`]), but the type's own
/// covariance (e.g., `Vec`, `Box` being covariant) must be known from its
/// definition.
///
/// [`DeserType`]: DeserInner::DeserType
/// [`__check_covariance`]: crate::deser::DeserInner::__check_covariance
#[macro_export]
macro_rules! unsafe_assume_covariance {
    ($($type:ty),* $(,)?) => {
        #[allow(clippy::useless_transmute)]
        #[inline(always)]
        fn __check_covariance<'__long: '__short, '__short>(
            proof: $crate::deser::CovariantProof<Self::DeserType<'__long>>,
        ) -> $crate::deser::CovariantProof<Self::DeserType<'__short>> {
            $(
                $crate::deser::__check_type_covariance::<$type>();
            )*
            // SAFETY: see the safety documentation of this macro.
            unsafe { ::core::mem::transmute(proof) }
        }
    };
}

/// Drops the backend field of a partially initialized [`MemCase`] unless
/// defused, preventing a leak if ε-copy deserialization errors or panics
/// after the backend has been installed.
struct BackendGuard<S: DeserInner>(*mut MemCase<S>);

impl<S: DeserInner> Drop for BackendGuard<S> {
    fn drop(&mut self) {
        // SAFETY: the backend field has been initialized, and the value field
        // has not (the guard is defused before the value is written).
        unsafe { addr_of_mut!((*self.0).1).drop_in_place() };
    }
}

/// Main deserialization trait. It is separated from [`DeserInner`] to avoid
/// that the user modify its behavior, and hide internal serialization methods.
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
/// - No validation is performed on deserialized values, whether the
///   deserialization is full-copy or ε-copy. Types with a validity invariant
///   are reinterpreted or converted without checking it, so an altered
///   serialized form can produce a value the invariant forbids, which is
///   undefined behavior. For example, this can yield an invalid `bool` (not
///   `0`/`1`), an invalid `char` (a surrogate or a value above `0x10FFFF`), a
///   zero [`NonZeroUsize`], or an ill-formed `str`/[`String`]. The data passed
///   to a deserialization method must come from a correct serialization.
///   Structural metadata is still checked: besides the header hashes, tags of
///   types such as [`Option`] are validated and yield [`Error::InvalidTag`] on
///   unknown values.
///
/// - The code assumes that the [`read_exact`] method of the backend does not
///   read the buffer. If the method reads the buffer, it will cause undefined
///   behavior. This is a general issue with Rust as the I/O traits were written
///   before [`MaybeUninit`] was stabilized.
///
/// - Malicious [`TypeHash`]/[`AlignHash`] implementations may lead to reading
///   incompatible structures using the same code, or cause undefined behavior
///   by loading data with an incorrect alignment. A wrong [`PadTo`]
///   implementation similarly desynchronizes the stream with respect to
///   serialization, causing wrong values or errors.
///
/// - Memory-mapped files might be modified externally.
///
/// The first problem can be solved by traits like [`FromBytes`]. The second
/// issue is a non-problem with the standard library.
///
/// The last two issues are more an issue of security than undefined behavior,
/// but that is in the eye of the beholder.
///
/// [`NonZeroUsize`]: core::num::NonZeroUsize
/// [`read_exact`]: ReadNoStd::read_exact
/// [`FromBytes`]: https://docs.rs/zerocopy/latest/zerocopy/trait.FromBytes.html
/// [`Deserialize::load_full`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_full
/// [`Deserialize::load_mem`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mem
/// [`Deserialize::mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.mmap
pub trait Deserialize: DeserInner {
    /// Fully deserializes a structure of this type from the given backend.
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [trait documentation]: Deserialize
    unsafe fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self>;
    /// ε-copy deserializes a structure of this type from the given backend.
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [trait documentation]: Deserialize
    unsafe fn deserialize_eps(backend: &'_ [u8]) -> Result<Self::DeserType<'_>>;

    /// Convenience method to fully deserialize from a file.
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [trait documentation]: Deserialize
    #[cfg(feature = "std")]
    unsafe fn load_full(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;
        let mut buf_reader = BufReader::new(file);
        unsafe { Self::deserialize_full(&mut buf_reader).map_err(|e| e.into()) }
    }

    /// Reads data from a reader into heap-allocated memory and ε-copy
    /// deserializes a data structure from it, returning a [`MemCase`]
    /// containing the data structure and the memory. Excess bytes are zeroed
    /// out.
    ///
    /// The allocated memory will have [`MemoryAlignment`] as alignment: types
    /// with a higher alignment requirement will cause an [alignment error].
    ///
    /// For a version using a file path, see [`load_mem`].
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
    /// See the [trait documentation].
    ///
    /// [alignment error]: Error::AlignmentError
    /// [`load_mem`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mem
    /// [trait documentation]: Deserialize
    unsafe fn read_mem(mut read: impl ReadNoStd, size: usize) -> anyhow::Result<MemCase<Self>> {
        let pad_to = align_of::<MemoryAlignment>();
        if align_of::<Self>() > pad_to {
            return Err(Error::AlignmentError.into());
        }
        // Round up to MemoryAlignment size; the maximum with pad_to
        // guarantees a nonzero allocation size even when size is zero.
        let capacity = size
            .checked_add(crate::pad_align_to(size, pad_to))
            .ok_or_else(|| {
                anyhow::anyhow!("Size too large: adding alignment padding overflows usize")
            })?
            .max(pad_to);
        let layout = core::alloc::Layout::from_size_align(capacity, pad_to)?;

        let mut uninit: MaybeUninit<MemCase<Self>> = MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        // SAFETY: the first size bytes of the vector will be filled by
        // read_exact (which errors out on short reads), and the remaining
        // bytes are zeroed explicitly below.
        #[allow(invalid_value)]
        let mut aligned_vec = unsafe {
            #[cfg(not(feature = "std"))]
            let alloc_func = alloc::alloc::alloc;
            #[cfg(feature = "std")]
            let alloc_func = std::alloc::alloc;
            #[cfg(not(feature = "std"))]
            let handle_alloc_error_func = alloc::alloc::handle_alloc_error;
            #[cfg(feature = "std")]
            let handle_alloc_error_func = std::alloc::handle_alloc_error;

            let raw = alloc_func(layout);
            if raw.is_null() {
                handle_alloc_error_func(layout);
            }

            <Vec<MemoryAlignment>>::from_raw_parts(
                raw as *mut MemoryAlignment,
                capacity / pad_to,
                capacity / pad_to,
            )
        };

        let bytes = unsafe {
            core::slice::from_raw_parts_mut(aligned_vec.as_mut_ptr() as *mut u8, capacity)
        };

        read.read_exact(&mut bytes[..size])?;
        // Fixes the last few bytes to guarantee zero-extension semantics
        // for bit vectors and full-vector initialization.
        bytes[size..].fill(0);

        let backend = MemBackend::Memory(AliasableBox::from(aligned_vec.into_boxed_slice()));

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(backend);
        }
        // From here on the backend is dropped if deserialization errors or
        // panics, so it cannot leak.
        let guard = BackendGuard(ptr);
        // deserialize the data structure
        let mem = unsafe { (*ptr).1.as_bytes().unwrap() };
        let s = unsafe { Self::deserialize_eps(mem) }?;
        core::mem::forget(guard);
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(MaybeDangling::new(s));
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }

    /// Loads a file into heap-allocated memory and ε-copy deserializes a data
    /// structure from it, returning a [`MemCase`] containing the data structure
    /// and the memory. Excess bytes are zeroed out.
    ///
    /// The allocated memory will have [`MemoryAlignment`] as alignment: types
    /// with a higher alignment requirement will cause an [alignment error].
    ///
    /// For a version using a generic [`std::io::Read`], see [`read_mem`].
    ///
    /// # Safety
    ///
    /// See the [trait documentation].
    ///
    /// [alignment error]: Error::AlignmentError
    /// [`read_mem`]: Self::read_mem
    /// [trait documentation]: Deserialize
    #[cfg(feature = "std")]
    unsafe fn load_mem(path: impl AsRef<Path>) -> anyhow::Result<MemCase<Self>> {
        let file_len = path
            .as_ref()
            .metadata()
            .map_err(Error::FileOpenError)?
            .len();
        anyhow::ensure!(
            file_len <= isize::MAX as u64,
            "File too large for the current architecture (longer than isize::MAX)"
        );
        let file_len = file_len as usize;
        let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;
        unsafe { Self::read_mem(file, file_len) }
    }

    /// Reads data from a reader into `mmap()`-allocated memory and ε-copy
    /// deserializes a data structure from it, returning a [`MemCase`]
    /// containing the data structure and the memory. Excess bytes are zeroed
    /// out.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`];
    /// otherwise, just pass `Flags::empty()`.
    ///
    /// For a version using a file path, see [`load_mmap`].
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
    /// See the [trait documentation].
    ///
    /// [`load_mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mmap
    /// [trait documentation]: Deserialize
    #[cfg(feature = "mmap")]
    unsafe fn read_mmap(
        mut read: impl ReadNoStd,
        size: usize,
        flags: Flags,
    ) -> anyhow::Result<MemCase<Self>> {
        // The maximum with 16 guarantees a nonzero mapping size even when
        // size is zero.
        let capacity = size
            .checked_add(crate::pad_align_to(size, 16))
            .ok_or_else(|| {
                anyhow::anyhow!("Size too large: adding alignment padding overflows usize")
            })?
            .max(16);

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
        // From here on the backend is dropped if deserialization errors or
        // panics, so it cannot leak.
        let guard = BackendGuard(ptr);
        // deserialize the data structure
        let mem = unsafe { (*ptr).1.as_bytes().unwrap() };
        let s = unsafe { Self::deserialize_eps(mem) }?;
        core::mem::forget(guard);
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(MaybeDangling::new(s));
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }

    /// Loads a file into `mmap()`-allocated memory and ε-copy deserializes a
    /// data structure from it, returning a [`MemCase`] containing the data
    /// structure and the memory. Excess bytes are zeroed out.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`];
    /// otherwise, just pass `Flags::empty()`.
    ///
    /// For a version using a generic [`std::io::Read`], see [`read_mmap`].
    ///
    /// Requires the `mmap` feature.
    ///
    /// # Safety
    ///
    /// See the [trait documentation] and [mmap's `with_file`'s documentation].
    ///
    /// [`read_mmap`]: Self::read_mmap
    /// [trait documentation]: Deserialize
    /// [mmap's `with_file`'s documentation]: mmap_rs::MmapOptions::with_file
    #[cfg(all(feature = "mmap", feature = "std"))]
    unsafe fn load_mmap(path: impl AsRef<Path>, flags: Flags) -> anyhow::Result<MemCase<Self>> {
        let file_len = path
            .as_ref()
            .metadata()
            .map_err(Error::FileOpenError)?
            .len();
        anyhow::ensure!(
            file_len <= isize::MAX as u64,
            "File too large for the current architecture (longer than isize::MAX)"
        );
        let file_len = file_len as usize;
        let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;
        unsafe { Self::read_mmap(file, file_len, flags) }
    }

    /// Memory maps a file and ε-copy deserializes a data structure from it,
    /// returning a [`MemCase`] containing the data structure and the memory
    /// mapping.
    ///
    /// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
    /// just pass `Flags::empty()`.
    ///
    /// Requires the `mmap` feature.
    ///
    /// # Safety
    ///
    /// See the [trait documentation] and [mmap's `with_file`'s documentation].
    ///
    /// [trait documentation]: Deserialize
    /// [mmap's `with_file`'s documentation]: mmap_rs::MmapOptions::with_file
    #[cfg(all(feature = "mmap", feature = "std"))]
    unsafe fn mmap(path: impl AsRef<Path>, flags: Flags) -> anyhow::Result<MemCase<Self>> {
        let file_len = path
            .as_ref()
            .metadata()
            .map_err(Error::FileOpenError)?
            .len();
        anyhow::ensure!(
            file_len <= isize::MAX as u64,
            "File too large for the current architecture (longer than isize::MAX)"
        );
        let file_len = file_len as usize;
        let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;

        let mut uninit: MaybeUninit<MemCase<Self>> = MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        let mmap = unsafe {
            mmap_rs::MmapOptions::new(file_len)?
                .with_flags(flags.mmap_flags())
                .with_file(&file, 0)
                .map()?
        };

        // store the backend inside the MemCase
        unsafe {
            addr_of_mut!((*ptr).1).write(MemBackend::Mmap(mmap));
        }
        // From here on the backend is dropped if deserialization errors or
        // panics, so it cannot leak.
        let guard = BackendGuard(ptr);
        let mmap = unsafe { (*ptr).1.as_bytes().unwrap() };
        // deserialize the data structure
        let s = unsafe { Self::deserialize_eps(mmap) }?;
        core::mem::forget(guard);
        // write the deserialized struct in the MemCase
        unsafe {
            addr_of_mut!((*ptr).0).write(MaybeDangling::new(s));
        }
        // finish init
        Ok(unsafe { uninit.assume_init() })
    }
}

/// Inner trait to implement deserialization of a type.
///
/// This trait exists to separate the user-facing [`Deserialize`] trait from the
/// low-level deserialization mechanisms of [`DeserInner::_deser_full_inner`]
/// and [`DeserInner::_deser_eps_inner`]. Moreover, it makes it possible to
/// behave slightly differently at the top of the recursion tree (e.g., to check
/// the endianness marker), and to prevent the user from modifying the methods
/// in [`Deserialize`].
///
/// The [`__check_covariance`] method guarantees that the deserialization type
/// associated with this type is covariant in its lifetime parameter, which is
/// necessary for the safety of the inner workings of [`MemCase`].
///
/// The user should not implement this trait directly, but rather derive it.
///
/// # Safety
///
/// See [`Deserialize`].
///
/// [`__check_covariance`]: Self::__check_covariance
pub trait DeserInner: Sized {
    /// The deserialization type associated with this type. It can be retrieved
    /// conveniently with the alias [`DeserType`].
    type DeserType<'a>;

    /// Internal method for checking the covariance of [`DeserType`].
    ///
    /// [`MemCase::uncase`] transmutes `DeserType<'static, S>` to
    /// `DeserType<'a, S>`, which is only sound if [`DeserType`] is covariant in
    /// its lifetime parameter.
    ///
    /// This method enforces that invariant: the only safe, returning
    /// implementation is `{ proof }`, which compiles only when [`DeserType`] is
    /// covariant. This happens because [`CovariantProof<T>`] is covariant in
    /// `T`, so an implicit coercion from `CovariantProof<DeserType<'long>>` to
    /// `CovariantProof<DeserType<'short>>` is only possible when
    /// `DeserType<'long>` is a subtype of `DeserType<'short>`, which is the
    /// definition of covariance.
    ///
    /// For structures where the compiler cannot see through associated-type
    /// projections, the body must be `unsafe { core::mem::transmute(proof) }`
    /// with a safety comment justifying covariance compositionally. The method
    /// [`__check_type_covariance`] should be called when relying on the
    /// covariance of field types, as the call will detect at runtime
    /// non-returning implementations (`todo!()`, `panic!()`,
    /// `unimplemented!()`, `loop {}`, etc.).
    ///
    /// All implementations must be `#[inline(always)]` to ensure that the
    /// covariance check has no cost.
    ///
    /// Two ready-made implementations are provided as macros:
    /// - [`check_covariance!()`]: the safe `{ proof }` body, for types whose
    ///   `DeserType` is a concrete covariant type;
    /// - [`unsafe_assume_covariance!()`]: the `unsafe` transmute body, for
    ///   generic containers that are compositionally covariant.
    ///
    /// [`MemCase::uncase`]: crate::deser::MemCase::uncase
    /// [`CovariantProof<T>`]: crate::deser::CovariantProof
    fn __check_covariance<'__long: '__short, '__short>(
        proof: CovariantProof<Self::DeserType<'__long>>,
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

/// Marker trait identifying types that are a fixed point of deserialization
/// (types `T` whose [`DeserType`] is `T` itself for every lifetime).
///
/// The derive emits an assertion against this trait when a type parameter is
/// both ε-copy and full-copy (it appears in a field marked
/// `#[epserde(force_full_copy)]` and in an unmarked field). The (de)serialization
/// code compiles only when `T` satisfies this trait, and the
/// `#[diagnostic::on_unimplemented]` attribute below surfaces an actionable
/// hint when the user has not supplied the required bound.
///
/// The derive emits a bound of the form `<T as DeserInner>::DeserType<'_>:
/// EitherFullOrEpsCopy<T>` for each conflicting parameter, so that when the
/// equality fails the blanket impl `impl<T> EitherFullOrEpsCopy<T> for T` does not
/// apply and rustc reports `EitherFullOrEpsCopy<T>` as unimplemented. The
/// `#[diagnostic::on_unimplemented]` attribute below then suggests the bound
/// that fixes the issue.
///
/// [`DeserType`]: DeserInner::DeserType
#[diagnostic::on_unimplemented(
    message = "type parameter `{T}` is both full-copy and ε-copy (it appears both in a field marked `#[epserde(force_full_copy)]` and in an unmarked field)",
    label = "this occurrence of `{T}` in an ε-copy field conflicts with its use in a full-copy field",
    note = "consider restricting type parameter `{T}` with `{T}: for<'a> DeserInner<DeserType<'a> = {T}>`",
    note = "alternatively, mark the ε-copy field with `#[epserde(force_full_copy)]`",
    note = "alternatively, pin the parameter to full-copy with `#[epserde(full_copy({T}))]` on the type"
)]
pub trait EitherFullOrEpsCopy<T: ?Sized> {}

impl<T: ?Sized> EitherFullOrEpsCopy<T> for T {}

/// Marker trait asserting that a type parameter used as a (possibly nested)
/// element of a literal vector, boxed slice, or array in an ε-copy field is
/// deep-copy, as required for ε-copy stability.
///
/// The derive emits an assertion against this trait for every type parameter
/// that occurs as the direct element of a literal `Vec<…>`, `Box<[…]>`, or `[…;
/// N]` inside an unmarked field. Were such a parameter zero-copy, the
/// containing sequence would ε-copy deserialize to a slice reference (`&[…]`),
/// a type not expressible as the original sequence; the parameter is therefore
/// forced to be deep-copy.
///
/// The blanket impl applies to every [`DeepCopy`] type, so the assertion holds
/// as soon as the user supplies the required bound. Alternatively, the
/// requirement is lifted by full-copy deserializing the offending field, either
/// with the field-level `#[epserde(force_full_copy)]` marker or, when the parameter
/// is that field's only ε-copy occurrence, by pinning the parameter with the
/// type-level `#[epserde(full_copy(...))]` attribute. The
/// `#[diagnostic::do_not_recommend]` attribute keeps the compiler from
/// reporting the missing `DeepCopy` bound as the root cause, so the
/// `#[diagnostic::on_unimplemented]` message below is what surfaces.
#[diagnostic::on_unimplemented(
    message = "type parameter `{Self}` must be deep-copy: it occurs as an element of a vector, boxed slice, or array in an ε-copy field",
    label = "if `{Self}` were zero-copy, this field would ε-copy deserialize to a slice reference, a type not expressible in the source",
    note = "consider restricting type parameter `{Self}` with trait `DeepCopy` (more targeted)",
    note = "alternatively, mark the field with `#[epserde(force_full_copy)]`",
    note = "alternatively, pin `{Self}` to full-copy with `#[epserde(full_copy({Self}))]` on the type, which makes `{Self}` full-copy in every field"
)]
pub trait DeepCopyInSeq {}

#[diagnostic::do_not_recommend]
impl<T: crate::traits::DeepCopy> DeepCopyInSeq for T {}

/// Marker trait witnessing that a field actual deserialization type matches the
/// slot the derive places for it in [`DeserType`]. Used to diagnose a
/// `#[epserde(full_copy(...))]` parameter that a field ε-copy deserializes.
///
/// The type-level `#[epserde(full_copy(T))]` attribute removes `T` from the
/// [`DeserType`] substitution set, leaving it verbatim. This is sound only when
/// the field type that carries `T` actually deserializes it full-copy (so its
/// own `DeserType` keeps `T` verbatim). When instead the field type
/// deserializes `T` ε-copy the slot the derive emits disagrees with the field's
/// real deserialization type, producing a raw slot mismatch.
///
/// For every ε-copy field that contains a `full_copy(...)`-pinned parameter the
/// derive emits an assertion `<Field as DeserInner>::DeserType<'_>:
/// FullCopyConsistent<Slot>`, where `Slot` is the field's slot in
/// [`DeserType`]. The blanket impl `impl<T> FullCopyConsistent<T> for T` makes
/// the bound hold exactly when the two coincide (so a legitimately full-copy
/// field is silent); otherwise it does not apply and the
/// `#[diagnostic::on_unimplemented]` message below points at the fix instead of
/// leaving the user with rustc's raw slot mismatch.
///
/// [`DeserType`]: DeserInner::DeserType
#[diagnostic::on_unimplemented(
    message = "a field deserialization type is inconsistent with `#[epserde(full_copy(...))]`",
    label = "a parameter pinned by `#[epserde(full_copy(...))]` is ε-copy deserialized by this field",
    note = "the field ε-copy deserializes to `{Self}`, but `#[epserde(full_copy(...))]` requires `{Expected}`",
    note = "consider removing that parameter from `#[epserde(full_copy(...))]`",
    note = "alternatively, mark this field with `#[epserde(force_full_copy)]`"
)]
pub trait FullCopyConsistent<Expected: ?Sized> {}

impl<T: ?Sized> FullCopyConsistent<T> for T {}

/// Blanket implementation that prevents the user from overwriting the methods
/// in [`Deserialize`].
///
/// This implementation [checks the header] written by the blanket
/// implementation of [`crate::ser::Serialize`] and then delegates to
/// [`DeserInner::_deser_full_inner`] or [`DeserInner::_deser_eps_inner`].
///
/// [checks the header]: check_header
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
    // SAFETY (for all the unsafe blocks in this function): the header
    // contains only primitive values and a byte sequence, which are valid
    // for any bit pattern, so the validity hazard that makes deserialization
    // unsafe cannot arise; this is why this function can be safe.

    /// Reads the serialized type name, which is diagnostic data possibly
    /// coming from a foreign file, without assuming it is valid UTF-8 and
    /// without letting its length prefix drive an unbounded allocation:
    /// the stream is consumed entirely, but only a bounded prefix of the
    /// name is kept.
    fn read_type_name(backend: &mut impl ReadWithPos) -> Result<String> {
        const MAX_NAME_LEN: usize = 1024;
        let len = unsafe { usize::_deser_full_inner(backend) }?;
        let mut name = Vec::with_capacity(len.min(MAX_NAME_LEN));
        let mut buf = [0u8; 256];
        let mut remaining = len;
        while remaining > 0 {
            let chunk = remaining.min(buf.len());
            backend.read_exact(&mut buf[..chunk])?;
            let keep = chunk.min(MAX_NAME_LEN.saturating_sub(name.len()));
            name.extend_from_slice(&buf[..keep]);
            remaining -= chunk;
        }
        Ok(String::from_utf8_lossy(&name).into_owned())
    }

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
        MAGIC_REV => Err(Error::EndiannessMismatch),
        magic => Err(Error::InvalidMagicCookie(magic)),
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

    if ser_type_hash != self_type_hash {
        // Do not let a failing name read mask the mismatch diagnostic.
        let ser_type_name = read_type_name(backend).unwrap_or_else(|_| "<unreadable>".to_string());
        return Err(Error::TypeHashMismatch {
            ser_type_name,
            ser_type_hash,
            self_type_name: core::any::type_name::<T>().to_string(),
            self_ser_type_name: core::any::type_name::<T::SerType>().to_string(),
            self_type_hash,
        });
    }
    if ser_align_hash != self_align_hash {
        // Do not let a failing name read mask the mismatch diagnostic.
        let ser_type_name = read_type_name(backend).unwrap_or_else(|_| "<unreadable>".to_string());
        return Err(Error::AlignHashMismatch {
            ser_type_name,
            ser_align_hash,
            self_type_name: core::any::type_name::<T>().to_string(),
            self_ser_type_name: core::any::type_name::<T::SerType>().to_string(),
            self_align_hash,
        });
    }

    // Consume the type name to position the stream at the body.
    let _ = read_type_name(backend)?;

    Ok(())
}

/// A helper trait that makes it possible to implement differently
/// deserialization for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types.
/// See [`crate::traits::CopyType`] for more information.
pub trait DeserHelper<T: CopySelector> {
    /// The type returned by full-copy deserialization (an owned instance).
    type FullType;
    /// The type returned by ε-copy deserialization for lifetime `'a`.
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
