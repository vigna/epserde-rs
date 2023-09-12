/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */



use bitflags::bitflags;
use core::ops::Deref;

bitflags! {
    /// Flags for [`map`] and [`load_mmap`].
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
            true => mmap_rs::MmapFlags::TRANSPARENT_HUGE_PAGES | mmap_rs::MmapFlags::COPY_ON_WRITE,
            false => mmap_rs::MmapFlags::empty(),
        }
    }
}

/// Possible backends of a [`MemCase`]. The `None` variant is used when the data structure is
/// created in memory; the `Memory` variant is used when the data structure is deserialized
/// from a file loaded into a heap-allocated memory region; the `Mmap` variant is used when
/// the data structure is deserialized from a `mmap()`-based region.
pub enum MemBackend {
    /// No backend. The data structure is a standard Rust data structure.
    /// This variant is returned by [`encase`].
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
/// of [`MemBackend`], so a data structure can be [encased](encase)
/// almost transparently.

pub struct MemCase<S>(pub(crate) S, pub(crate) MemBackend);

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
pub fn encase<S>(s: S) -> MemCase<S> {
    MemCase(s, MemBackend::None)
}

impl<S: Send + Sync> From<S> for MemCase<S> {
    fn from(s: S) -> Self {
        encase(s)
    }
}


