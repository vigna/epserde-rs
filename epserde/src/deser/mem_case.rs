/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Mechanisms for keeping together [ε-copy deserialized] instances and the
//! memory regions they point to.
//!
//! Please refer to the documentation of [`MemCase`] for details.
//!
//! [ε-copy deserialized]: crate::deser::Deserialize::deserialize_eps

use crate::{DeserInner, deser::DeserType, ser::SerInner};
use aliasable::boxed::AliasableBox;
use bitflags::bitflags;
use core::fmt;
use maybe_dangling::MaybeDangling;

bitflags! {
    /// Flags for [`mmap`], [`load_mmap`], and [`read_mmap`].
    ///
    /// [`mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.mmap
    /// [`load_mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mmap
    /// [`read_mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mmap
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

/// The alignment used by the [`Memory`] variant of [`MemBackend`].
///
/// [`Memory`]: MemBackend::Memory
pub type MemoryAlignment = crate::Aligned64;

/// Possible backends of a [`MemCase`]. The [`None`] variant is used when the
/// instance is owned; the [`Memory`] variant is used when the instance has
/// been deserialized from a heap-allocated memory region; the [`Mmap`] variant
/// is used when the instance has been deserialized from a `mmap()`-based
/// region, either coming from an allocation or from mapping a file.
///
/// [`None`]: MemBackend::None
/// [`Memory`]: MemBackend::Memory
/// [`Mmap`]: https://docs.rs/epserde/latest/epserde/deser/mem_case/enum.MemBackend.html#variant.Mmap
#[derive(Debug)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
pub enum MemBackend {
    /// No backend. The instance is owned. This variant is returned by
    /// [`MemCase::encase`].
    None,
    /// The backend is heap-allocated in a memory region aligned to 64 bytes.
    /// This variant is returned by [`load_mem`].
    ///
    /// The allocation is held in an [`AliasableBox`] rather than a plain `Box`
    /// because a [`MemCase`] also stores references pointing inside this region:
    /// a plain `Box` asserts unique access to its contents, which conflicts with
    /// those aliasing references under the stricter aliasing models checked by
    /// Miri.
    ///
    /// [`load_mem`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mem
    Memory(AliasableBox<[MemoryAlignment]>),
    /// The backend is the result to a call to `mmap()`. This variant is
    /// returned by [`load_mmap`] and [`mmap`].
    ///
    /// [`load_mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mmap
    /// [`mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.mmap
    #[cfg(feature = "mmap")]
    Mmap(mmap_rs::Mmap),
}

impl MemBackend {
    /// Returns the bytes of the backing memory region, or [`None`] for the
    /// [`None`][owned] (owned) variant.
    ///
    /// [owned]: MemBackend::None
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            MemBackend::None => None,
            MemBackend::Memory(mem) => {
                // Deref-coerce the AliasableBox to the underlying slice.
                let mem: &[MemoryAlignment] = mem;
                Some(unsafe {
                    core::slice::from_raw_parts(
                        mem.as_ptr() as *const MemoryAlignment as *const u8,
                        size_of_val(mem),
                    )
                })
            }
            #[cfg(feature = "mmap")]
            MemBackend::Mmap(mmap) => Some(mmap),
        }
    }
}

/// A transparent wrapper that implements the ε-serde (de)serialization traits
/// with (de)serialization type equal to `T`.
///
/// The only purpose of this wrapper is to make [encasing] of arbitrary owned
/// types possible, since the parameter of a [`MemCase`] must implement
/// [`DeserInner`].
///
/// No instance of this structure will ever be accessible to the user. All
/// methods are unimplemented. If the convenience type alias [`MemOwned<T>`] is
/// used, the user does not even have to ever mention this type.
///
/// [encasing]: MemCase::encase
/// [`MemOwned<T>`]: MemOwned
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
#[repr(transparent)]
pub struct Owned<T>(T);

/// A convenience type alias for the type of [`MemCase`] instances containing
/// owned instances.
///
/// This alias is particularly useful in conjunction with the [implementation of
/// `From<T>` for `MemOwned<T>`].
///
/// [implementation of `From<T>` for `MemOwned<T>`]: #impl-From<T>-for-MemCase<Owned<T>>
pub type MemOwned<T> = MemCase<Owned<T>>;

impl<T> DeserInner for Owned<T> {
    type DeserType<'a> = T;

    #[inline(always)]
    fn __check_covariance<'__long: '__short, '__short>(
        proof: super::CovariantProof<Self::DeserType<'__long>>,
    ) -> super::CovariantProof<Self::DeserType<'__short>> {
        proof
    }

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
/// and [`Deref`], and an implementation of [`PartialEq`] and [`Eq`] whenever
/// the inner type supports them.
///
/// A [`MemCase`] is parameterized by a type `S` implementing [`DeserInner`],
/// but it stores an inner instance of type [`DeserType<'static, S>`]. The
/// references contained in the latter point inside the [`MemBackend`] (i.e., a
/// [`MemCase`] is in general self-referential). You must use [`uncase`] to get
/// a reference to the inner instance.
///
/// [`MemCase`] instances can also be built from owned instances using the
/// [`MemCase::encase`] method or the [convenient implementation] of [`From`]
/// for [`MemOwned<T>`].
///
/// It is thus possible to treat ε-copy deserialized instances and owned
/// instances uniformly using [`MemCase`]. The only drawback is that
/// [`MemCase`] instances have a few dozen bytes of overhead due to the
/// [`MemBackend`] instance they contain.
///
/// Note that if you plan to bound with traits the behavior of [`MemCase`]
/// instances, you should do so on the deserialization type associated with the
/// underlying type, as that will be the type of the reference returned by
/// [`uncase`].
///
/// # Why not the [`yoke`] crate?
///
/// A [`MemCase`] is structurally a [`Yoke`]. We do not wrap a [`Yoke`] for
/// three reasons:
///
/// 1. **Per-type [`Yokeable`] impls.** [`Yoke<Y, _>`][`Yoke`] requires `Y:
///    for<'a> Yokeable<'a>`, implemented on the `'static` form of a type with
///    `Output` being that type with `'static` replaced by `'a`. The bound
///    `for<'a> DeserType<'static, S>: Yokeable<'a, Output = DeserType<'a, S>>` is
///    in fact expressible, and built-in deserialization types (`&[T]`, `Vec<T>`,
///    …) are already [`Yokeable`]; but every *derived* deserialization type would
///    need a [`Yokeable`] impl emitted by our derive macro, since there is no
///    blanket impl for arbitrary structs.
/// 2. **Stronger covariance check.** [`Yokeable`] is an unsafe trait whose
///    lifetime cast is sound only by contract. We instead prove covariance with
///    the compiler-checked [`CovariantProof`] / [`DeserInner::__check_covariance`]
///    machinery.
/// 3. **No support for arbitrary owned instances.** [`encase`] wraps an
///    *arbitrary* owned `T` (see the [`From`] impl for [`MemOwned<T>`]) with no
///    trait bound on `T`. [`yoke`] cannot hold an arbitrary owned type: even
///    its owned constructors ([`Yoke::new_always_owned`], giving [`Yoke<Y,
///    ()>`][`Yoke`]) still require `Y: for<'a> Yokeable<'a>`, which a plain
///    user struct does not implement. Wrapping [`Yoke`] would therefore
///    restrict [`encase`] to [`Yokeable`] types, whereas [`MemBackend::None`]
///    supports any `T` directly.
///
/// [`yoke`]: https://docs.rs/yoke/latest/yoke/
/// [`Yoke`]: https://docs.rs/yoke/latest/yoke/struct.Yoke.html
/// [`Yoke::new_always_owned`]: https://docs.rs/yoke/latest/yoke/struct.Yoke.html#method.new_always_owned
/// [`Yokeable`]: https://docs.rs/yoke/latest/yoke/trait.Yokeable.html
/// [`CovariantProof`]: super::CovariantProof
/// [`Deref`]: core::ops::Deref
/// [`DeserType<'static, S>`]: DeserType
/// [`uncase`]: MemCase::uncase
/// [convenient implementation]: #impl-From<T>-for-MemCase<Owned<T>>
/// [`MemOwned<T>`]: MemOwned
/// [`encase`]: MemCase::encase
/// [`Index<usize>`]: core::ops::Index
///
/// # Examples
///
/// Suppose we want to write a function accepting a [`MemCase`] whose inner
/// instance implements [`Index<usize>`]:
///
/// ```
/// use epserde::deser::{MemCase, DeserInner};
/// use core::ops::Index;
///
/// fn do_something<S: for<'a> DeserInner<DeserType<'a>: Index<usize, Output = usize>>>(
///     indexable: MemCase<S>
/// ) -> usize{
///     indexable.uncase()[0]
/// }
///```
#[cfg_attr(feature = "mem_dbg", derive(mem_dbg::MemDbg, mem_dbg::MemSize))]
pub struct MemCase<S: DeserInner>(
    pub(crate) MaybeDangling<DeserType<'static, S>>,
    pub(crate) MemBackend,
);

impl<S: DeserInner> fmt::Debug for MemCase<S>
where
    for<'a> DeserType<'a, S>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MemCase")
            .field(self.uncase())
            .field(&self.1)
            .finish()
    }
}

/// Convenience implementation to create a [`MemCase`] from an owned instance.
///
/// If you are assigning to a field of type [`MemOwned<T>`] (a type alias for
/// `MemCase<Owned<T>>`), you can just write `field: t.into()`, where `t` is of
/// type `T`.
///
/// # Examples
///
/// ```
/// # use epserde::deser::MemOwned;
/// let owned: MemOwned<Vec<usize>> = vec![1, 2, 3].into();
/// assert_eq!(owned.uncase(), &[1, 2, 3]);
/// ```
///
/// [`MemOwned<T>`]: MemOwned
impl<T> From<T> for MemCase<Owned<T>> {
    fn from(t: T) -> Self {
        <MemOwned<T>>::encase(t)
    }
}

impl<T> MemCase<Owned<T>> {
    /// Encases an owned instance in a [`MemCase`] with no backend.
    ///
    /// A [`MemCase`] must store a deserialization associated type, so this
    /// method wraps its argument in a [`Owned`] wrapper, returning the type
    /// alias [`MemOwned<T>`], which is [`MemCase<Owned<T>>`]. Since the
    /// deserialization type of [`Owned<T>`] is `T`, [`MemCase::uncase`] will
    /// return a reference to the instance of `T`.
    ///
    /// The [convenient implementation of `From<T>` for `MemOwned<T>`] is
    /// usually easier to use.
    ///
    /// [`MemOwned<T>`]: MemOwned
    /// [convenient implementation of `From<T>` for `MemOwned<T>`]: #impl-From<T>-for-MemCase<Owned<T>>
    pub const fn encase(s: T) -> MemOwned<T> {
        MemCase(MaybeDangling::new(s), MemBackend::None)
    }
}

impl<S: DeserInner> MemCase<S> {
    /// Returns a reference to the instance contained in this [`MemCase`].
    ///
    /// Both the lifetime of the returned reference and the lifetime of
    /// the inner deserialization type will be that of `self`.
    pub fn uncase<'a>(&'a self) -> &'a DeserType<'a, S> {
        // Call the covariance check. This is a ZST-in, ZST-out no-op that the
        // optimizer eliminates entirely in release builds unless it doesn't
        // return (e.g., todo!(), panic!(), etc.).
        super::__check_type_covariance::<S>();
        // SAFETY: 'static outlives 'a, and DeserType<'_, S> is covariant in its
        // lifetime parameter, as enforced by the required method
        // DeserInner::__check_covariance.
        // &*self.0 derefs the MaybeDangling wrapper to &DeserType<'static, S>.
        unsafe { core::mem::transmute::<&'a DeserType<'static, S>, &'a DeserType<'a, S>>(&*self.0) }
    }

    /// Returns a reference to the instance contained in this [`MemCase`]
    /// with type `&DeserType<'static, S>`.
    ///
    /// # Safety
    ///
    /// The intended usage of this method is that of calling easily methods on
    /// the inner instance, as in `mem_case.uncase_static().method()`. The
    /// returned reference itself borrows `self`, so it cannot outlive the
    /// [`MemCase`]; the danger lies in the `'static` lifetime of the inner
    /// deserialization type: safe code can extract from the returned reference
    /// inner references with lifetime `'static` and use them after the
    /// [`MemCase`] has been dropped, leading to undefined behavior.
    pub unsafe fn uncase_static(&self) -> &DeserType<'static, S> {
        &self.0
    }
}

// SAFETY: a MemCase is the deserialized value plus its MemBackend. These impls
// assert that every backend variant is Send/Sync: None trivially, Memory
// because it is an aliasable box of plain bytes, and Mmap because
// mmap_rs::Mmap implements Send and Sync (in mmap-rs 0.7 all platform mapping
// types implement them explicitly). The deserialized value is covered by the
// where clauses, which require the deserialization type of S to be Send/Sync.
unsafe impl<S: DeserInner> Send for MemCase<S> where DeserType<'static, S>: Send {}
unsafe impl<S: DeserInner> Sync for MemCase<S> where DeserType<'static, S>: Sync {}

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

impl<A: ?Sized, S: DeserInner> core::ops::Deref for MemCase<S>
where
    for<'a> DeserType<'a, S>: core::ops::Deref<Target = A>,
{
    type Target = <DeserType<'static, S> as core::ops::Deref>::Target;

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
