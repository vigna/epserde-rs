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
        /// Suggest that the mapped region will be accessed sequentially.
        ///
        /// This flag is only a suggestion, and it is ignored if the kernel does
        /// not support it. It is mainly useful to support `madvise()` on Linux.
        const SEQUENTIAL = 1 << 1;
        /// Suggest that the mapped region will be accessed randomly.
        ///
        /// This flag is only a suggestion, and it is ignored if the kernel does
        /// not support it. It is mainly useful to support `madvise()` on Linux.
        const RANDOM_ACCESS = 1 << 2;
    }
}

/// Empty flags.
impl core::default::Default for Flags {
    fn default() -> Self {
        Flags::empty()
    }
}

impl Flags {
    /// Translates internal flags to `mmap_rs` flags.
    pub(crate) fn mmap_flags(&self) -> mmap_rs::MmapFlags {
        let mut flags: mmap_rs::MmapFlags = mmap_rs::MmapFlags::empty();
        if self.contains(Self::SEQUENTIAL) {
            flags |= mmap_rs::MmapFlags::SEQUENTIAL;
        }
        if self.contains(Self::RANDOM_ACCESS) {
            flags |= mmap_rs::MmapFlags::RANDOM_ACCESS;
        }
        if self.contains(Self::TRANSPARENT_HUGE_PAGES) {
            flags |= mmap_rs::MmapFlags::TRANSPARENT_HUGE_PAGES;
        }

        flags
    }
}

/// Possible backends of a [`MemCase`]. The `None` variant is used when the data structure is
/// created in memory; the `Memory` variant is used when the data structure is deserialized
/// from a file loaded into a heap-allocated memory region; the `Mmap` variant is used when
/// the data structure is deserialized from a `mmap()`-based region, either coming from
/// an allocation or a from mapping a file.
pub enum MemBackend {
    /// No backend. The data structure is a standard Rust data structure.
    /// This variant is returned by [`MemCase::encase`].
    None,
    /// The backend is a heap-allocated in a memory region aligned to 4096 bytes.
    /// This variant is returned by [`crate::deser::Deserialize::load_mem`].
    Memory(Vec<u8>),
    /// The backend is the result to a call to `mmap()`.
    /// This variant is returned by [`crate::deser::Deserialize::load_mmap`] and [`crate::deser::Deserialize::mmap`].
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
/// it was deserialized from. [`MemCase`] instances can not be cloned, but references
/// to such instances can be shared freely.
///
/// [`MemCase`] implements [`Deref`] and [`AsRef`] to the
/// wrapped type, so it can be used almost transparently and
/// with no performance cost. However,
/// if you need to use a memory-mapped structure as a field in
/// a struct and you want to avoid `dyn`, you will have
/// to use [`MemCase`] as the type of the field.
/// [`MemCase`] implements [`From`] for the
/// wrapped type, using the no-op [`None`](`MemBackend#variant.None`) variant
/// of [`MemBackend`], so a structure can be [encased](MemCase::encase)
/// almost transparently.

pub struct MemCase<S>(pub(crate) S, pub(crate) MemBackend);

impl<S> MemCase<S> {
    /// Encases a data structure in a [`MemCase`] with no backend.
    pub fn encase(s: S) -> MemCase<S> {
        MemCase(s, MemBackend::None)
    }
}

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

impl<S: Send + Sync> From<S> for MemCase<S> {
    fn from(s: S) -> Self {
        MemCase::encase(s)
    }
}
