/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::{DeserializeInner, deser::DeserType};
use bitflags::bitflags;
use core::{fmt, mem::size_of};
use maligned::A64;
use mem_dbg::{MemDbg, MemSize};

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

/// Possible backends of a [`MemCase`]. The [`None`](MemBackend::None) variant
/// is used when the data structure is created in memory; the
/// [`Memory`](MemBackend::Memory) variant is used when the data structure is
/// deserialized from a file loaded into a heap-allocated memory region; the
/// [`Mmap`](MemBackend::Mmap) variant is used when the data structure is
/// deserialized from a `mmap()`-based region, either coming from an allocation
/// or a from mapping a file.
#[derive(Debug, MemDbg, MemSize)]
pub enum MemBackend {
    /// No backend. The data structure is a standard Rust data structure.
    /// This variant is returned by [`MemCase::encase`].
    None,
    /// The backend is a heap-allocated in a memory region aligned to 16 bytes.
    /// This variant is returned by [`crate::deser::Deserialize::load_mem`].
    Memory(Box<[MemoryAlignment]>),
    /// The backend is the result to a call to `mmap()`.
    /// This variant is returned by [`crate::deser::Deserialize::load_mmap`] and [`crate::deser::Deserialize::mmap`].
    #[cfg(feature = "mmap")]
    Mmap(mmap_rs::Mmap),
}

impl MemBackend {
    pub fn as_ref(&self) -> Option<&[u8]> {
        match self {
            MemBackend::None => None,
            MemBackend::Memory(mem) => Some(unsafe {
                core::slice::from_raw_parts(
                    mem.as_ptr() as *const MemoryAlignment as *const u8,
                    mem.len() * size_of::<MemoryAlignment>(),
                )
            }),
            #[cfg(feature = "mmap")]
            MemBackend::Mmap(mmap) => Some(mmap),
        }
    }
}

/// A wrapper keeping together an immutable structure and the [`MemBackend`] it
/// was deserialized from. [`MemCase`] instances can not be cloned, but
/// references to such instances can be shared freely.
///
/// A [`MemCase`] is parameterized by a type `S`, but it stores an inner
/// structure of type `DeserType<'static, S>`. The references contained in the
/// latter point inside the [`MemBackend`] (i.e., a [`MemCase`] is in general
/// self-referential).
///
/// You must use [`uncase`](MemCase::uncase) to get a reference to the inner
/// structure. If you need to use [`MemCase`]s parameterized by `S` and
/// structures of type `S` interchangeably you need to implement the same traits
/// for both cases. Note that for delegation to work, the traits must be
/// implemented also on the deserialization type of `S`, but this usually not a
/// problem because traits that do not satisfy this property are unusable on
/// [ε-copy deserialized](crate::deser::Deserialize::deserialize_eps) structures.
///
/// We provide implementations for [`MemCase`] delegating basic traits from the
/// standard library, such as [`AsRef`], [`Deref`](std::ops::Deref) and
/// [`IntoIterator`] (the latter, implemented on a reference) to the inner
/// structure.
///
/// Packages that are ε-serde–aware are encouraged to provide such delegations
/// for their traits. Note that in case the traits contain associated types such
/// delegations require some lifetime juggling using
/// [`transmute`](core::mem::transmute), as [`uncase`](MemCase::uncase) returns
/// a `&'a DeserType<'a, S>`, where `'a` is the lifetime
/// of `self`, but associated types for the delegating implementation will be
/// necessarily written using `DeserType<'static, S>`. The
/// unsafe [`uncase_static`](MemCase::uncase_static) method might be handy.
/// Examples can be found in the delegations of the trait `IndexedDict` from the
/// [`sux`](https://crates.io/crates/sux) crate.
#[derive(MemDbg, MemSize)]
pub struct MemCase<S: DeserializeInner>(pub(crate) DeserType<'static, S>, pub(crate) MemBackend);

impl<S: DeserializeInner> fmt::Debug for MemCase<S>
where
    DeserType<'static, S>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MemBackend")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<S: DeserializeInner> MemCase<S> {
    /// Encases a data structure in a [`MemCase`] with no backend.
    ///
    /// Note that since a [`MemCase`] stores a deserialization associated type,
    /// this method is useful only for types that are equal to their own
    /// deserialization type (e.g., they do not have type parameters).
    /// Moreover, type inference will not be able to deduce `S` from the
    /// argument, so you will need to specify it explicitly.
    pub fn encase(s: DeserType<'static, S>) -> Self {
        MemCase(s, MemBackend::None)
    }

    /// Returns a reference to the structure contained in this [`MemCase`].
    ///
    /// Both the lifetime of the returned reference and the lifetime of
    /// the inner deserialization type will be that of `self`.
    pub fn uncase<'a>(&'a self) -> &'a DeserType<'a, S> {
        // SAFETY: 'static outlives 'a, and DeserType<S, '_> is required to be
        // covariant (i.e., it's a normal structure and not, say, a closure with
        // 'a as argument)
        unsafe { core::mem::transmute::<&'a DeserType<'static, S>, &'a DeserType<'a, S>>(&self.0) }
    }

    /// Returns a reference to the structure contained in this [`MemCase`]
    /// with type `&DeserType<'static, S>`.
    ///
    /// # Safety
    ///
    /// The intended usage of this method is that of calling easily methods on
    /// the inner structure, as in `mem_case.uncase_static().method()`. The
    /// returned reference is dangerous, as it is decoupled from the [`MemCase`]
    /// instance; even storing it in a variable can easily lead to undefined behavior
    /// (e.g., if the [`MemCase`] is dropped before the reference is used).
    pub unsafe fn uncase_static(&self) -> &DeserType<'static, S> {
        &self.0
    }
}

unsafe impl<S: DeserializeInner + Send> Send for MemCase<S> {}
unsafe impl<S: DeserializeInner + Sync> Sync for MemCase<S> {}

impl<'a, S: DeserializeInner> IntoIterator for &'a MemCase<S>
where
    &'a DeserType<'a, S>: IntoIterator,
{
    type Item = <&'a DeserType<'a, S> as IntoIterator>::Item;
    type IntoIter = <&'a DeserType<'a, S> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.uncase().into_iter()
    }
}

impl<A, S: DeserializeInner> AsRef<A> for MemCase<S>
where
    for<'a> DeserType<'a, S>: AsRef<A>,
{
    fn as_ref(&self) -> &A {
        self.uncase().as_ref()
    }
}

impl<A: ?Sized, S: DeserializeInner> std::ops::Deref for MemCase<S>
where
    for<'a> DeserType<'a, S>: std::ops::Deref<Target = A>,
{
    type Target = <DeserType<'static, S> as std::ops::Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.uncase().deref()
    }
}

impl<Idx, S: DeserializeInner> core::ops::Index<Idx> for MemCase<S>
where
    for<'a> DeserType<'a, S>: core::ops::Index<Idx>,
{
    type Output = <DeserType<'static, S> as core::ops::Index<Idx>>::Output;

    fn index(&self, index: Idx) -> &Self::Output {
        unsafe { &self.uncase_static()[index] }
    }
}

impl<S: DeserializeInner> PartialOrd<MemCase<S>> for MemCase<S>
where
    for<'a> DeserType<'a, S>: PartialOrd<DeserType<'a, S>>,
{
    fn partial_cmp(&self, other: &MemCase<S>) -> Option<core::cmp::Ordering> {
        self.uncase().partial_cmp(other.uncase())
    }
}

impl<S: DeserializeInner> Ord for MemCase<S>
where
    for<'a> DeserType<'a, S>: Ord,
{
    fn cmp(&self, other: &MemCase<S>) -> core::cmp::Ordering {
        self.uncase().cmp(other.uncase())
    }
}

impl<S: DeserializeInner> PartialEq<MemCase<S>> for MemCase<S>
where
    for<'a> DeserType<'a, S>: PartialEq<DeserType<'a, S>>,
{
    fn eq(&self, other: &MemCase<S>) -> bool {
        self.uncase().eq(other.uncase())
    }
}

impl<S: DeserializeInner> Eq for MemCase<S> where for<'a> DeserType<'a, S>: Eq {}
