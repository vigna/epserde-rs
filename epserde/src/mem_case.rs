/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::{pad_align_to, Deserialize, DeserializeInner};
use anyhow::Result;
use bitflags::bitflags;
use core::ops::Deref;

bitflags! {
    /// Flags for [`map`] and [`load`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Flags: u32 {
        /// Suggest to map a region using transparent huge pages. This flag
        /// is only a suggestion, and it is ignored if the kernel does not
        /// support transparent huge pages. It is mainly useful to support
        /// `madvise()`-based huge pages on Linux. Note that at the time
        /// of this writing Linux does not support transparent huge pages
        /// in file-based memory mappings.
        const TRANSPARENT_HUGE_PAGES = 1 << 0;
    }
}

impl core::default::Default for Flags {
    fn default() -> Self {
        unsafe { core::mem::transmute::<u32, Flags>(0) }
    }
}

impl Flags {
    pub fn mmap_flags(&self) -> mmap_rs::MmapFlags {
        match self.contains(Flags::TRANSPARENT_HUGE_PAGES) {
            // By passing COPY_ON_WRITE we set the MAP_PRIVATE flag, which
            // in necessary for transparent huge pages to work.
            true => mmap_rs::MmapFlags::TRANSPARENT_HUGE_PAGES,
            false => mmap_rs::MmapFlags::empty(),
        }
    }
}

/// Possible backends of a [`MemCase`]. The `None` variant is used when the data structure is
/// created in memory; the `Memory` variant is used when the data structure is deserialized
/// from a file loaded into an allocated memory region; the `Mmap` variant is used when
/// the data structure is deserialized from a memory-mapped region.
pub enum MemBackend {
    /// No backend. The data structure is a standard Rust data structure.
    /// This variant is returned by [`encase_mem`].
    None,
    /// The backend is a heap-allocated in a memory region aligned to 4096 bytes.
    /// This variant is returned by [`load`].
    Memory(Vec<u8>),
    /// The backend is the result to a call to `mmap()`.
    /// This variant is returned by [`load_mmap`] and [`map`].
    Mmap(mmap_rs::Mmap),
}

impl MemBackend {
    pub fn as_ref(&self) -> Option<&[u8]> {
        match self {
            MemBackend::None => None,
            MemBackend::Memory(mem) => Some(mem),
            MemBackend::Mmap(mmap) => Some(mmap),
        }
    }
}

/// A wrapper keeping together an immutable structure and the memory
/// it was deserialized from. It is specifically designed for
/// the case of memory-mapped regions, where the mapping must
/// be kept alive for the whole lifetime of the data structure.
/// [`MemCase`] instances can not be cloned, but references
/// to such instances can be shared freely.
///
/// [`MemCase`] can also be used with data structures deserialized from
/// memory, although in that case it is not strictly necessary;
/// nonetheless, reading a single block of memory with [`Read::read_exact`] can be
/// very fast, and using [`load`] to create a [`MemCase`]
/// is a way to prevent cloning of the immutable
/// structure.
///
/// [`MemCase`] implements [`Deref`] and [`AsRef`] to the
/// wrapped type, so it can be used almost transparently and
/// with no performance cost. However,
/// if you need to use a memory-mapped structure as a field in
/// a struct and you want to avoid `dyn`, you will have
/// to use [`MemCase`] as the type of the field.
/// [`MemCase`] implements [`From`] for the
/// wrapped type, using the no-op [`None`](`MemBackend#variant.None`) variant
/// of [`MemBackend`], so a data structure can be [encased](encase_mem)
/// almost transparently.

pub struct MemCase<S>(pub S, MemBackend);

unsafe impl<S: Send> Send for MemCase<S> {}
unsafe impl<S: Sync> Sync for MemCase<S> {}

impl<S> Deref for MemCase<S> {
    type Target = S;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> AsRef<S> for MemCase<S> {
    #[inline(always)]
    fn as_ref(&self) -> &S {
        &self.0
    }
}

/// Encases a data structure in a [`MemCase`] with no backend.
pub fn encase_mem<S>(s: S) -> MemCase<S> {
    MemCase(s, MemBackend::None)
}

impl<S: Send + Sync> From<S> for MemCase<S> {
    fn from(s: S) -> Self {
        encase_mem(s)
    }
}

use std::{io::Read, mem::MaybeUninit, path::Path, ptr::addr_of_mut};

/// Load a file into heap-allocated memory and deserialize a data structure from it,
/// returning a [`MemCase`] containing the data structure and the
/// memory. Excess bytes are zeroed out.
pub fn load<'a, S: Deserialize>(
    path: impl AsRef<Path>,
) -> Result<MemCase<<S as DeserializeInner>::DeserType<'a>>> {
    let file_len = path.as_ref().metadata()?.len() as usize;
    let mut file = std::fs::File::open(path)?;
    // Round up to u128 size
    let len = file_len + pad_align_to(file_len, 16);

    let mut uninit: MaybeUninit<MemCase<<S as DeserializeInner>::DeserType<'_>>> =
        MaybeUninit::uninit();
    let ptr = uninit.as_mut_ptr();

    // SAFETY: the entire vector will be filled with data read from the file,
    // or with zeroes if the file is shorter than the vector.
    let mut bytes = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc(std::alloc::Layout::from_size_align(len, 4096)?),
            len,
            len,
        )
    };

    file.read_exact(&mut bytes[..file_len])?;
    // Fixes the last few bytes to guarantee zero-extension semantics
    // for bit vectors and full-vector initialization.
    bytes[file_len..].fill(0);
    let backend = MemBackend::Memory(bytes);

    // store the backend inside the MemCase
    unsafe {
        addr_of_mut!((*ptr).1).write(backend);
    }
    // deserialize the data structure
    let mem = unsafe { (*ptr).1.as_ref().unwrap() };
    let s = S::deserialize_eps_copy(mem)?;
    // write the deserialized struct in the memcase
    unsafe {
        addr_of_mut!((*ptr).0).write(s);
    }
    // finish init
    Ok(unsafe { uninit.assume_init() })
}

/// Load a file into `mmap()`-allocated memory and deserialize a data structure from it,
/// returning a [`MemCase`] containing the data structure and the
/// memory. Excess bytes are zeroed out.
///
/// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
/// just pass `&Flags::empty()`.
#[allow(clippy::uninit_vec)]
pub fn load_mmap<'a, S: Deserialize>(
    path: impl AsRef<Path>,
    flags: &Flags,
) -> Result<MemCase<<S as DeserializeInner>::DeserType<'a>>> {
    let file_len = path.as_ref().metadata()?.len() as usize;
    let mut file = std::fs::File::open(path)?;
    let capacity = (file_len + 7) / 8;

    let mut uninit: MaybeUninit<MemCase<<S as DeserializeInner>::DeserType<'_>>> =
        MaybeUninit::uninit();
    let ptr = uninit.as_mut_ptr();

    let mut mmap = mmap_rs::MmapOptions::new(capacity * 8)?
        .with_flags(flags.mmap_flags())
        .map_mut()?;
    file.read_exact(&mut mmap[..file_len])?;
    // Fixes the last few bytes to guarantee zero-extension semantics
    // for bit vectors.
    mmap[file_len..].fill(0);

    let backend = MemBackend::Mmap(mmap.make_read_only().map_err(|(_, err)| err).unwrap());

    // store the backend inside the MemCase
    unsafe {
        addr_of_mut!((*ptr).1).write(backend);
    }
    // deserialize the data structure
    let mem = unsafe { (*ptr).1.as_ref().unwrap() };
    let s = S::deserialize_eps_copy(mem)?;
    // write the deserialized struct in the MemCase
    unsafe {
        addr_of_mut!((*ptr).0).write(s);
    }
    // finish init
    Ok(unsafe { uninit.assume_init() })
}

/// Memory map a file and deserialize a data structure from it,
/// returning a [`MemCase`] containing the data structure and the
/// memory mapping.
///
/// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
/// just pass `&Flags::empty()`.
#[allow(clippy::uninit_vec)]
pub fn map<'a, S: Deserialize>(
    path: impl AsRef<Path>,
    flags: &Flags,
) -> Result<MemCase<<S as DeserializeInner>::DeserType<'a>>> {
    let file_len = path.as_ref().metadata()?.len();
    let file = std::fs::File::open(path)?;

    let mut uninit: MaybeUninit<MemCase<<S as DeserializeInner>::DeserType<'_>>> =
        MaybeUninit::uninit();
    let ptr = uninit.as_mut_ptr();

    let mmap = unsafe {
        mmap_rs::MmapOptions::new(file_len as _)?
            .with_flags(flags.mmap_flags() | mmap_rs::MmapFlags::COPY_ON_WRITE)
            .with_file(file, 0)
            .map()?
    };

    // store the backend inside the MemCase
    unsafe {
        addr_of_mut!((*ptr).1).write(MemBackend::Mmap(mmap));
    }

    let mmap = unsafe { (*ptr).1.as_ref().unwrap() };
    // deserialize the data structure
    let s = S::deserialize_eps_copy(mmap)?;
    // write the deserialized struct in the MemCase
    unsafe {
        addr_of_mut!((*ptr).0).write(s);
    }
    // finish init
    Ok(unsafe { uninit.assume_init() })
}
