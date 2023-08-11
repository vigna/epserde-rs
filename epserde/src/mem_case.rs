/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::{Deserialize, DeserializeInner};
use anyhow::Result;
use bitflags::bitflags;
use core::ops::Deref;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Flags: u32 {
        const MMAP = 1 << 0;
        const TRANSPARENT_HUGE_PAGES = 1 << 1;
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
/// from a file loaded into an allocated memory region; the `Mmap` variant is used when
/// the data structure is deserialized from a memory-mapped region.
pub enum MemBackend {
    /// No backend. The data structure is a standard Rust data structure.
    None,
    /// The backend is an allocated in a memory region aligned to 64 bits.
    Memory(Vec<u64>),
    /// The backend is a memory-mapped region.
    Mmap(mmap_rs::Mmap),
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

/// Memory map a file and deserialize a data structure from it,
/// returning a [`MemCase`] containing the data structure and the
/// memory mapping.
#[allow(clippy::uninit_vec)]
pub fn map<S: Deserialize>(
    path: impl AsRef<Path>,
    flags: &Flags,
) -> Result<MemCase<<S as DeserializeInner>::DeserType<'_>>> {
    let file_len = path.as_ref().metadata()?.len();
    let file = std::fs::File::open(path)?;

    Ok({
        let mut uninit: MaybeUninit<MemCase<<S as DeserializeInner>::DeserType<'_>>> =
            MaybeUninit::uninit();
        let ptr = uninit.as_mut_ptr();

        let mmap = unsafe {
            mmap_rs::MmapOptions::new(file_len as _)?
                .with_flags(flags.mmap_flags())
                .with_file(file, 0)
                .map()?
        };

        unsafe {
            addr_of_mut!((*ptr).1).write(MemBackend::Mmap(mmap));
        }

        if let MemBackend::Mmap(mmap) = unsafe { &(*ptr).1 } {
            let s = S::deserialize_eps_copy(mmap)?;
            unsafe {
                addr_of_mut!((*ptr).0).write(s);
            }

            unsafe { uninit.assume_init() }
        } else {
            unreachable!()
        }
    })
}

/// Load a file into memory and deserialize a data structure from it,
/// returning a [`MemCase`] containing the data structure and the
/// memory. Excess bytes are zeroed out.
#[allow(clippy::uninit_vec)]
pub fn load<S: Deserialize>(
    path: impl AsRef<Path>,
    flags: &Flags,
) -> Result<MemCase<<S as DeserializeInner>::DeserType<'_>>> {
    let file_len = path.as_ref().metadata()?.len() as usize;
    let mut file = std::fs::File::open(path)?;
    let capacity = (file_len + 7) / 8;

    if flags.contains(Flags::MMAP) {
        let mut mmap = mmap_rs::MmapOptions::new(capacity * 8)?
            .with_flags(flags.mmap_flags())
            .map_mut()?;
        Ok({
            let mut uninit: MaybeUninit<MemCase<<S as DeserializeInner>::DeserType<'_>>> =
                MaybeUninit::uninit();
            let ptr = uninit.as_mut_ptr();

            file.read_exact(&mut mmap[..file_len])?;
            // Fixes the last few bytes to guarantee zero-extension semantics
            // for bit vectors.
            mmap[file_len..].fill(0);

            unsafe {
                if let Ok(mmap_ro) = mmap.make_read_only() {
                    addr_of_mut!((*ptr).1).write(MemBackend::Mmap(mmap_ro));
                } else {
                    unreachable!("make_read_only() failed")
                }
            }

            if let MemBackend::Mmap(mmap) = unsafe { &mut (*ptr).1 } {
                let s = S::deserialize_eps_copy(mmap)?;

                unsafe {
                    addr_of_mut!((*ptr).0).write(s);
                }

                unsafe { uninit.assume_init() }
            } else {
                unreachable!()
            }
        })
    } else {
        let mut mem = Vec::<u64>::with_capacity(capacity);
        unsafe {
            // This is safe because we are filling the vector
            // reading from a file.
            mem.set_len(capacity);
        }
        Ok({
            let mut uninit: MaybeUninit<MemCase<<S as DeserializeInner>::DeserType<'_>>> =
                MaybeUninit::uninit();
            let ptr = uninit.as_mut_ptr();

            let bytes: &mut [u8] = bytemuck::cast_slice_mut::<u64, u8>(mem.as_mut_slice());
            file.read_exact(&mut bytes[..file_len])?;
            // Fixes the last few bytes to guarantee zero-extension semantics
            // for bit vectors.
            bytes[file_len..].fill(0);

            unsafe {
                addr_of_mut!((*ptr).1).write(MemBackend::Memory(mem));
            }

            if let MemBackend::Memory(mem) = unsafe { &mut (*ptr).1 } {
                let s = S::deserialize_eps_copy(bytemuck::cast_slice::<u64, u8>(mem))?;

                unsafe {
                    addr_of_mut!((*ptr).0).write(s);
                }

                unsafe { uninit.assume_init() }
            } else {
                unreachable!()
            }
        })
    }
}
