/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Mechanisms for keeping together [ε-copy
//! deserialized](crate::deser::Deserialize::deserialize_eps) instances and the
//! memory regions they point to.
//!
//! Please refer to the documentation of [`MemCase`] for details.

use crate::{DeserInner, deser::DeserType, ser::SerInner};
use bitflags::bitflags;
use core::{fmt, mem::size_of};
use maligned::A64;
use mem_dbg::{MemDbg, MemSize};

bitflags! {
    /// Flags for [`mmap`](crate::deser::Deserialize::mmap) and
    ///  and [`load_mmap`](crate::deser::Deserialize::load_mmap).
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
/// is used when the instance is owned; the [`Memory`](MemBackend::Memory) variant
/// is used when the instance has been deserialized a heap-allocated memory
/// region; the [`Mmap`](MemBackend::Mmap) variant is used when the instance has
/// been deserialized from a `mmap()`-based region, either coming from an
/// allocation or a from mapping a file.
#[derive(Debug, MemDbg, MemSize)]
pub enum MemBackend {
    /// No backend. The instance is owned. This variant is returned by
    /// [`MemCase::encase`].
    None,
    /// The backend is a heap-allocated in a memory region aligned to 16 bytes.
    /// This variant is returned by [`crate::deser::Deserialize::load_mem`].
    Memory(Box<[MemoryAlignment]>),
    /// The backend is the result to a call to `mmap()`. This variant is
    /// returned by [`crate::deser::Deserialize::load_mmap`] and
    /// [`crate::deser::Deserialize::mmap`].
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

/// A transparent wrapper that implement the ε-serde (de)serialization traits
/// with (de)serialization type equal to `T`.
///
/// The only purpose of this wrapper is to make [encasing](MemCase::encase) of
/// arbitrary owned types possible, since the parameter of a [`MemCase`] must
/// implement [`DeserInner`].
///
/// No instance of this structure will ever be accessible to the user. All
/// methods are unimplemented. If the convenience type alias
/// [`MemOwned<T>`](MemOwned) is used, the user does not even have to ever
/// mention this type.
#[derive(Debug, MemSize, MemDbg, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Owned<T>(T);

/// A convenience type alias for the type of [`MemCase`] instances
/// containing owned instances.
///
/// This alias is particular useful in conjunction with the [implementation of
/// `From<T>` for `MemOwned<T>`](#impl-From<T>-for-MemCase<Owned<T>>).
pub type MemOwned<T> = MemCase<Owned<T>>;

impl<T> DeserInner for Owned<T> {
    type DeserType<'a> = T;

    unsafe fn _deser_full_inner(_backend: &mut impl super::ReadWithPos) -> super::Result<Self> {
        unimplemented!();
    }

    unsafe fn _deser_eps_inner<'a>(
        _backend: &mut super::SliceWithPos<'a>,
    ) -> super::Result<Self::DeserType<'a>> {
        unimplemented!();
    }
}

impl<T> SerInner for Owned<T> {
    type SerType = T;

    const IS_ZERO_COPY: bool = false;
    const ZERO_COPY_MISMATCH: bool = false;

    unsafe fn _ser_inner(
        &self,
        _backend: &mut impl crate::ser::WriteWithNames,
    ) -> crate::ser::Result<()> {
        unimplemented!();
    }
}

/// A wrapper keeping together an immutable instance and the [`MemBackend`] it
/// was deserialized from (or, possibly, an owned instance).
///
/// [`MemCase`] instances can not be cloned, but references to such instances
/// can be shared freely. For convenience we provide delegations for [`AsRef`]
/// and [`Deref`](std::ops::Deref), and an implementation of [`PartialEq`] and
/// [`Eq`] whenever the inner type supports them.
///
/// A [`MemCase`] is parameterized by a type `S` implementing [`DeserInner`],
/// but it stores an inner instance of type [`DeserType<'static,
/// S>`](DeserType). The references contained in the latter point inside the
/// [`MemBackend`] (i.e., a [`MemCase`] is in general self-referential). You
/// must use [`uncase`](MemCase::uncase) to get a reference to the inner
/// instance.
///
/// [`MemCase`] instances can also be built from owned instances using the
/// [`MemCase::encase`] method or the [convenient
/// implementation](#impl-From<T>-for-MemCase<Owned<T>>) of [`From`] for
/// [`MemOwned<T>`](MemOwned).
///
/// It is thus possible to treat ε-copy deserialized instances and owned
/// instances uniformly using [`MemCase`]. The only drawback is that
/// [`MemCase`] instances have a few dozen bytes of overhead due to the
/// [`MemBackend`] instance they contain.
///
/// Note that if you plan to bind with traits the behavior of [`MemCase`]
/// instances, you should do so on the deserialization type associated with the
/// underlying type, as that will be the type of the reference returned by
/// [`uncase`](MemCase::uncase).
///
/// # Examples
///
/// Suppose we want to write a function accepting a [`MemCase`] whose
/// inner instance implements [`Index<usize>`](std::ops::Index):
///
/// ```
/// use epserde::deser::{MemCase, DeserInner};
/// use std::ops::Index;
///
/// fn do_something<S: for<'a> DeserInner<DeserType<'a>: Index<usize, Output = usize>>>(
///     indexable: MemCase<S>
/// ) -> usize{
///     indexable.uncase()[0]
/// }
///```
#[derive(MemDbg, MemSize)]
pub struct MemCase<S: DeserInner>(pub(crate) DeserType<'static, S>, pub(crate) MemBackend);

impl<S: DeserInner> fmt::Debug for MemCase<S>
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

/// Convenience implementation to create a [`MemCase`] from an owned instance.
///
/// If you are assigning to a field of type [`MemOwned<T>`](MemOwned)
/// (a type alias for `MemCase<Owned<T>>`, you can
/// just write `field: t.into()`, where `t` is of type `T`.
///
/// # Examples
///
/// ```
/// # use epserde::deser::{MemCase, Owned};
/// let owned: MemOwned<Vec<usize>> = vec![1, 2, 3].into();
/// assert_eq!(owned.uncase(), &[1, 2, 3]);
/// ```
impl<T> From<T> for MemCase<Owned<T>> {
    fn from(t: T) -> Self {
        <MemOwned<T>>::encase(t)
    }
}

impl<S: DeserInner> MemCase<S> {
    /// Encases an owned instance in a [`MemCase`] with no backend.
    ///
    /// A [`MemCase`] must store a deserialization associated type, so this
    /// methods wraps its argument in a [`Owned`] wrapper, returning the type
    /// alias [`MemOwned<T>`](MemOwned), which is [`MemCase<Owned<T>>`]. Since
    /// the deserialization type of [`Owned<T>`] is `T`, [`MemCase::uncase`]
    /// will return a reference to the instance of `T`.
    ///
    /// Type inference will not work with this method as the compiler should be
    /// able to work back `T` from `MemOwned<T>::DeserType<'a>`. The
    /// [convenient implementation of `From<T>` for
    /// `MemOwned<T>`](#impl-From<T>-for-MemCase<Owned<T>>) is usually easier to use.
    pub fn encase<T>(s: T) -> MemOwned<T> {
        MemCase(s, MemBackend::None)
    }

    /// Returns a reference to the instance contained in this [`MemCase`].
    ///
    /// Both the lifetime of the returned reference and the lifetime of
    /// the inner deserialization type will be that of `self`.
    pub fn uncase<'a>(&'a self) -> &'a DeserType<'a, S> {
        // SAFETY: 'static outlives 'a, and DeserType<S, '_> is required to be
        // covariant (i.e., it's a normal struct/enum and not, say, a closure with
        // 'a as argument)
        unsafe { core::mem::transmute::<&'a DeserType<'static, S>, &'a DeserType<'a, S>>(&self.0) }
    }

    /// Returns a reference to the instance contained in this [`MemCase`]
    /// with type `&DeserType<'static, S>`.
    ///
    /// # Safety
    ///
    /// The intended usage of this method is that of calling easily methods on
    /// the inner instance, as in `mem_case.uncase_static().method()`. The
    /// returned reference is dangerous, as it is decoupled from the [`MemCase`]
    /// instance; even storing it in a variable can easily lead to undefined behavior
    /// (e.g., if the [`MemCase`] is dropped before the reference is used).
    pub unsafe fn uncase_static(&self) -> &DeserType<'static, S> {
        &self.0
    }
}

unsafe impl<S: DeserInner + Send> Send for MemCase<S> {}
unsafe impl<S: DeserInner + Sync> Sync for MemCase<S> {}

impl<A, S: DeserInner> AsRef<A> for MemCase<S>
where
    for<'a> DeserType<'a, S>: AsRef<A>,
{
    fn as_ref(&self) -> &A {
        self.uncase().as_ref()
    }
}

impl<A, T: AsRef<A>> AsRef<A> for Owned<T> {
    fn as_ref(&self) -> &A {
        self.0.as_ref()
    }
}

impl<A: ?Sized, S: DeserInner> std::ops::Deref for MemCase<S>
where
    for<'a> DeserType<'a, S>: std::ops::Deref<Target = A>,
{
    type Target = <DeserType<'static, S> as std::ops::Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.uncase().deref()
    }
}

impl<S: DeserInner> PartialEq<MemCase<S>> for MemCase<S>
where
    for<'a, 'b> DeserType<'a, S>: PartialEq<DeserType<'b, S>>,
{
    fn eq(&self, other: &MemCase<S>) -> bool {
        self.uncase().eq(other.uncase())
    }
}

impl<S: DeserInner> Eq for MemCase<S>
where
    for<'a, 'b> DeserType<'a, S>: PartialEq<DeserType<'b, S>>,
    for<'a> DeserType<'a, S>: Eq,
{
}
