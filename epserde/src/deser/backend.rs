/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use bitflags::bitflags;
use core::mem::size_of;
use maligned::A64;
use mem_dbg::{MemDbg, MemSize};
use std::ops::Deref;

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
    #[cfg(feature = "mmap")]
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

/// The [alignment](maligned::Alignment) by the [`Memory`](MemBackend::Memory) variant of [`MemBackend`].
pub type MemoryAlignment = A64;

/// Possible backends of a [`Yoke`]. The `None` variant is used when the data structure is
/// created in memory; the `Memory` variant is used when the data structure is deserialized
/// from a file loaded into a heap-allocated memory region; the `Mmap` variant is used when
/// the data structure is deserialized from a `mmap()`-based region, either coming from
/// an allocation or a from mapping a file.
#[derive(Debug, MemDbg, MemSize)]
pub enum MemBackend {
    /// No backend. The data structure is a standard Rust data structure.
    None,
    /// The backend is a heap-allocated in a memory region aligned to 16 bytes.
    Memory(Box<[MemoryAlignment]>),
    /// The backend is the result to a call to `mmap()`.
    #[cfg(feature = "mmap")]
    Mmap(mmap_rs::Mmap),
}

impl Deref for MemBackend {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            MemBackend::None => &[],
            MemBackend::Memory(mem) => unsafe {
                core::slice::from_raw_parts(
                    mem.as_ptr() as *const u8,
                    mem.len() * size_of::<MemoryAlignment>(),
                )
            },
            #[cfg(feature = "mmap")]
            MemBackend::Mmap(mmap) => mmap,
        }
    }
}

// This is safe because the only method on `StableDeref` is `deref`, which
// is already implemented for `MemBackend` and is stable.
unsafe impl stable_deref_trait::StableDeref for MemBackend {}
