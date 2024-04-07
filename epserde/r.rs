#![feature(prelude_import)]
//!# ε-serde
#![deny(unconditional_recursion)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
#[cfg(feature = "derive")]
pub use epserde_derive::{Epserde, TypeInfo};
pub mod deser {
    /*!

Deserialization traits and types

[`Deserialize`] is the main deserialization trait, providing methods
[`Deserialize::deserialize_eps`] and [`Deserialize::deserialize_full`]
which implement ε-copy and full-copy deserialization, respectively.
The implementation of this trait is based on [`DeserializeInner`],
which is automatically derived with `#[derive(Deserialize)]`.

*/
    use crate::traits::*;
    use crate::{MAGIC, MAGIC_REV, VERSION};
    use core::mem::align_of;
    use core::ptr::addr_of_mut;
    use core::{hash::Hasher, mem::MaybeUninit};
    use std::{io::BufReader, path::Path};
    pub mod helpers {
        /*!

Helpers for deserialization.

*/
        use super::SliceWithPos;
        use super::{read::*, DeserializeInner};
        use crate::deser;
        use crate::traits::*;
        use core::mem::MaybeUninit;
        /// Full-copy deserialize a zero-copy structure.
        pub fn deserialize_full_zero<T: ZeroCopy>(
            backend: &mut impl ReadWithPos,
        ) -> deser::Result<T> {
            backend.align::<T>()?;
            unsafe {
                #[allow(clippy::uninit_assumed_init)]
                let mut buf: MaybeUninit<T> = MaybeUninit::uninit();
                let slice = core::slice::from_raw_parts_mut(
                    &mut buf as *mut MaybeUninit<T> as *mut u8,
                    core::mem::size_of::<T>(),
                );
                backend.read_exact(slice)?;
                Ok(buf.assume_init())
            }
        }
        /// Full-copy deserialize a vector of zero-copy structures.
        ///
        /// Note that this method uses a single [`ReadNoStd::read_exact`]
        /// call to read the entire vector.
        pub fn deserialize_full_vec_zero<T: DeserializeInner + ZeroCopy>(
            backend: &mut impl ReadWithPos,
        ) -> deser::Result<Vec<T>> {
            let len = usize::_deserialize_full_inner(backend)?;
            backend.align::<T>()?;
            let mut res = Vec::with_capacity(len);
            #[allow(clippy::uninit_vec)]
            unsafe {
                res.set_len(len);
                backend.read_exact(res.align_to_mut::<u8>().1)?;
            }
            Ok(res)
        }
        /// Full-copy deserialize a vector of deep-copy structures.
        pub fn deserialize_full_vec_deep<T: DeserializeInner + DeepCopy>(
            backend: &mut impl ReadWithPos,
        ) -> deser::Result<Vec<T>> {
            let len = usize::_deserialize_full_inner(backend)?;
            let mut res = Vec::with_capacity(len);
            for _ in 0..len {
                res.push(T::_deserialize_full_inner(backend)?);
            }
            Ok(res)
        }
        /// ε-copy deserialize a reference to a zero-copy structure
        /// backed by the `data` field of `backend`.
        pub fn deserialize_eps_zero<'a, T: ZeroCopy>(
            backend: &mut SliceWithPos<'a>,
        ) -> deser::Result<&'a T> {
            let bytes = core::mem::size_of::<T>();
            backend.align::<T>()?;
            let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
            if true {
                if !pre.is_empty() {
                    ::core::panicking::panic("assertion failed: pre.is_empty()")
                }
            }
            if true {
                if !after.is_empty() {
                    ::core::panicking::panic("assertion failed: after.is_empty()")
                }
            }
            let res = &data[0];
            backend.skip(bytes);
            Ok(res)
        }
        /// ε-copy deserialize a reference to a slice of zero-copy structures
        /// backed by the `data` field of `backend`.
        pub fn deserialize_eps_slice_zero<'a, T: ZeroCopy>(
            backend: &mut SliceWithPos<'a>,
        ) -> deser::Result<&'a [T]> {
            let len = usize::_deserialize_full_inner(backend)?;
            let bytes = len * core::mem::size_of::<T>();
            backend.align::<T>()?;
            let (pre, data, after) = unsafe { backend.data[..bytes].align_to::<T>() };
            if true {
                if !pre.is_empty() {
                    ::core::panicking::panic("assertion failed: pre.is_empty()")
                }
            }
            if true {
                if !after.is_empty() {
                    ::core::panicking::panic("assertion failed: after.is_empty()")
                }
            }
            backend.skip(bytes);
            Ok(data)
        }
        /// ε-copy deserialize a vector of deep-copy structures.
        pub fn deserialize_eps_vec_deep<'a, T: DeepCopy + DeserializeInner>(
            backend: &mut SliceWithPos<'a>,
        ) -> deser::Result<Vec<<T as DeserializeInner>::DeserType<'a>>> {
            let len = usize::_deserialize_full_inner(backend)?;
            let mut res = Vec::with_capacity(len);
            for _ in 0..len {
                res.push(T::_deserialize_eps_inner(backend)?);
            }
            Ok(res)
        }
    }
    pub use helpers::*;
    pub mod mem_case {
        use bitflags::bitflags;
        use core::{mem::size_of, ops::Deref};
        use maligned::A64;
        use mem_dbg::{MemDbg, MemSize};
        /// Flags for [`map`] and [`load_mmap`].
        pub struct Flags(<Flags as ::bitflags::__private::PublicFlags>::Internal);
        #[automatically_derived]
        impl ::core::fmt::Debug for Flags {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Flags", &&self.0)
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Flags {
            #[inline]
            fn clone(&self) -> Flags {
                let _: ::core::clone::AssertParamIsClone<
                    <Flags as ::bitflags::__private::PublicFlags>::Internal,
                >;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Flags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Flags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Flags {
            #[inline]
            fn eq(&self, other: &Flags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Flags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) -> () {
                let _: ::core::cmp::AssertParamIsEq<
                    <Flags as ::bitflags::__private::PublicFlags>::Internal,
                >;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Flags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Flags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Flags {
            #[inline]
            fn cmp(&self, other: &Flags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Flags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl Flags {
            /// Suggest to map a region using transparent huge pages. This flag
            /// is only a suggestion, and it is ignored if the kernel does not
            /// support transparent huge pages. It is mainly useful to support
            /// `madvise()`-based huge pages on Linux. Note that at the time
            /// of this writing Linux does not support transparent huge pages
            /// in file-based memory mappings.
            #[allow(deprecated, non_upper_case_globals)]
            pub const TRANSPARENT_HUGE_PAGES: Self = Self::from_bits_retain(1 << 0);
            /// Suggest that the mapped region will be accessed sequentially.
            ///
            /// This flag is only a suggestion, and it is ignored if the kernel does
            /// not support it. It is mainly useful to support `madvise()` on Linux.
            #[allow(deprecated, non_upper_case_globals)]
            pub const SEQUENTIAL: Self = Self::from_bits_retain(1 << 1);
            /// Suggest that the mapped region will be accessed randomly.
            ///
            /// This flag is only a suggestion, and it is ignored if the kernel does
            /// not support it. It is mainly useful to support `madvise()` on Linux.
            #[allow(deprecated, non_upper_case_globals)]
            pub const RANDOM_ACCESS: Self = Self::from_bits_retain(1 << 2);
        }
        impl ::bitflags::Flags for Flags {
            const FLAGS: &'static [::bitflags::Flag<Flags>] = &[
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new(
                        "TRANSPARENT_HUGE_PAGES",
                        Flags::TRANSPARENT_HUGE_PAGES,
                    )
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("SEQUENTIAL", Flags::SEQUENTIAL)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("RANDOM_ACCESS", Flags::RANDOM_ACCESS)
                },
            ];
            type Bits = u32;
            fn bits(&self) -> u32 {
                Flags::bits(self)
            }
            fn from_bits_retain(bits: u32) -> Flags {
                Flags::from_bits_retain(bits)
            }
        }
        #[allow(
            dead_code,
            deprecated,
            unused_doc_comments,
            unused_attributes,
            unused_mut,
            unused_imports,
            non_upper_case_globals,
            clippy::assign_op_pattern,
            clippy::indexing_slicing,
            clippy::same_name_method,
            clippy::iter_without_into_iter,
        )]
        const _: () = {
            #[repr(transparent)]
            pub struct InternalBitFlags(u32);
            #[automatically_derived]
            impl ::core::clone::Clone for InternalBitFlags {
                #[inline]
                fn clone(&self) -> InternalBitFlags {
                    let _: ::core::clone::AssertParamIsClone<u32>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for InternalBitFlags {}
            #[automatically_derived]
            impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
            #[automatically_derived]
            impl ::core::cmp::PartialEq for InternalBitFlags {
                #[inline]
                fn eq(&self, other: &InternalBitFlags) -> bool {
                    self.0 == other.0
                }
            }
            #[automatically_derived]
            impl ::core::cmp::Eq for InternalBitFlags {
                #[inline]
                #[doc(hidden)]
                #[coverage(off)]
                fn assert_receiver_is_total_eq(&self) -> () {
                    let _: ::core::cmp::AssertParamIsEq<u32>;
                }
            }
            #[automatically_derived]
            impl ::core::cmp::PartialOrd for InternalBitFlags {
                #[inline]
                fn partial_cmp(
                    &self,
                    other: &InternalBitFlags,
                ) -> ::core::option::Option<::core::cmp::Ordering> {
                    ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
                }
            }
            #[automatically_derived]
            impl ::core::cmp::Ord for InternalBitFlags {
                #[inline]
                fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                    ::core::cmp::Ord::cmp(&self.0, &other.0)
                }
            }
            #[automatically_derived]
            impl ::core::hash::Hash for InternalBitFlags {
                #[inline]
                fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                    ::core::hash::Hash::hash(&self.0, state)
                }
            }
            impl ::bitflags::__private::PublicFlags for Flags {
                type Primitive = u32;
                type Internal = InternalBitFlags;
            }
            impl ::bitflags::__private::core::default::Default for InternalBitFlags {
                #[inline]
                fn default() -> Self {
                    InternalBitFlags::empty()
                }
            }
            impl ::bitflags::__private::core::fmt::Debug for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter<'_>,
                ) -> ::bitflags::__private::core::fmt::Result {
                    if self.is_empty() {
                        f.write_fmt(
                            format_args!("{0:#x}", <u32 as ::bitflags::Bits>::EMPTY),
                        )
                    } else {
                        ::bitflags::__private::core::fmt::Display::fmt(self, f)
                    }
                }
            }
            impl ::bitflags::__private::core::fmt::Display for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter<'_>,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::parser::to_writer(&Flags(*self), f)
                }
            }
            impl ::bitflags::__private::core::str::FromStr for InternalBitFlags {
                type Err = ::bitflags::parser::ParseError;
                fn from_str(
                    s: &str,
                ) -> ::bitflags::__private::core::result::Result<Self, Self::Err> {
                    ::bitflags::parser::from_str::<Flags>(s).map(|flags| flags.0)
                }
            }
            impl ::bitflags::__private::core::convert::AsRef<u32> for InternalBitFlags {
                fn as_ref(&self) -> &u32 {
                    &self.0
                }
            }
            impl ::bitflags::__private::core::convert::From<u32> for InternalBitFlags {
                fn from(bits: u32) -> Self {
                    Self::from_bits_retain(bits)
                }
            }
            #[allow(dead_code, deprecated, unused_attributes)]
            impl InternalBitFlags {
                /// Get a flags value with all bits unset.
                #[inline]
                pub const fn empty() -> Self {
                    { Self(<u32 as ::bitflags::Bits>::EMPTY) }
                }
                /// Get a flags value with all known bits set.
                #[inline]
                pub const fn all() -> Self {
                    {
                        let mut truncated = <u32 as ::bitflags::Bits>::EMPTY;
                        let mut i = 0;
                        {
                            {
                                let flag = <Flags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <Flags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <Flags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        let _ = i;
                        Self::from_bits_retain(truncated)
                    }
                }
                /// Get the underlying bits value.
                ///
                /// The returned value is exactly the bits set in this flags value.
                #[inline]
                pub const fn bits(&self) -> u32 {
                    let f = self;
                    { f.0 }
                }
                /// Convert from a bits value.
                ///
                /// This method will return `None` if any unknown bits are set.
                #[inline]
                pub const fn from_bits(
                    bits: u32,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let bits = bits;
                    {
                        let truncated = Self::from_bits_truncate(bits).0;
                        if truncated == bits {
                            ::bitflags::__private::core::option::Option::Some(Self(bits))
                        } else {
                            ::bitflags::__private::core::option::Option::None
                        }
                    }
                }
                /// Convert from a bits value, unsetting any unknown bits.
                #[inline]
                pub const fn from_bits_truncate(bits: u32) -> Self {
                    let bits = bits;
                    { Self(bits & Self::all().bits()) }
                }
                /// Convert from a bits value exactly.
                #[inline]
                pub const fn from_bits_retain(bits: u32) -> Self {
                    let bits = bits;
                    { Self(bits) }
                }
                /// Get a flags value with the bits of a flag with the given name set.
                ///
                /// This method will return `None` if `name` is empty or doesn't
                /// correspond to any named flag.
                #[inline]
                pub fn from_name(
                    name: &str,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let name = name;
                    {
                        {
                            if name == "TRANSPARENT_HUGE_PAGES" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(Flags::TRANSPARENT_HUGE_PAGES.bits()),
                                );
                            }
                        };
                        {
                            if name == "SEQUENTIAL" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(Flags::SEQUENTIAL.bits()),
                                );
                            }
                        };
                        {
                            if name == "RANDOM_ACCESS" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(Flags::RANDOM_ACCESS.bits()),
                                );
                            }
                        };
                        let _ = name;
                        ::bitflags::__private::core::option::Option::None
                    }
                }
                /// Whether all bits in this flags value are unset.
                #[inline]
                pub const fn is_empty(&self) -> bool {
                    let f = self;
                    { f.bits() == <u32 as ::bitflags::Bits>::EMPTY }
                }
                /// Whether all known bits in this flags value are set.
                #[inline]
                pub const fn is_all(&self) -> bool {
                    let f = self;
                    { Self::all().bits() | f.bits() == f.bits() }
                }
                /// Whether any set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn intersects(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.bits() & other.bits() != <u32 as ::bitflags::Bits>::EMPTY }
                }
                /// Whether all set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn contains(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.bits() & other.bits() == other.bits() }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                pub fn insert(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    {
                        *f = Self::from_bits_retain(f.bits()).union(other);
                    }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `remove` won't truncate `other`, but the `!` operator will.
                #[inline]
                pub fn remove(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    {
                        *f = Self::from_bits_retain(f.bits()).difference(other);
                    }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                pub fn toggle(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    {
                        *f = Self::from_bits_retain(f.bits())
                            .symmetric_difference(other);
                    }
                }
                /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
                #[inline]
                pub fn set(&mut self, other: Self, value: bool) {
                    let f = self;
                    let other = other;
                    let value = value;
                    {
                        if value {
                            f.insert(other);
                        } else {
                            f.remove(other);
                        }
                    }
                }
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn intersection(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() & other.bits()) }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn union(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() | other.bits()) }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                #[must_use]
                pub const fn difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() & !other.bits()) }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn symmetric_difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() ^ other.bits()) }
                }
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                #[must_use]
                pub const fn complement(self) -> Self {
                    let f = self;
                    { Self::from_bits_truncate(!f.bits()) }
                }
            }
            impl ::bitflags::__private::core::fmt::Binary for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::Binary::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::fmt::Octal for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::Octal::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::fmt::LowerHex for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::LowerHex::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::fmt::UpperHex for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::UpperHex::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::ops::BitOr for InternalBitFlags {
                type Output = Self;
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor(self, other: InternalBitFlags) -> Self {
                    self.union(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitOrAssign for InternalBitFlags {
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor_assign(&mut self, other: Self) {
                    self.insert(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitXor for InternalBitFlags {
                type Output = Self;
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor(self, other: Self) -> Self {
                    self.symmetric_difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitXorAssign for InternalBitFlags {
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor_assign(&mut self, other: Self) {
                    self.toggle(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitAnd for InternalBitFlags {
                type Output = Self;
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand(self, other: Self) -> Self {
                    self.intersection(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitAndAssign for InternalBitFlags {
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand_assign(&mut self, other: Self) {
                    *self = Self::from_bits_retain(self.bits()).intersection(other);
                }
            }
            impl ::bitflags::__private::core::ops::Sub for InternalBitFlags {
                type Output = Self;
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub(self, other: Self) -> Self {
                    self.difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::SubAssign for InternalBitFlags {
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub_assign(&mut self, other: Self) {
                    self.remove(other);
                }
            }
            impl ::bitflags::__private::core::ops::Not for InternalBitFlags {
                type Output = Self;
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                fn not(self) -> Self {
                    self.complement()
                }
            }
            impl ::bitflags::__private::core::iter::Extend<InternalBitFlags>
            for InternalBitFlags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn extend<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(&mut self, iterator: T) {
                    for item in iterator {
                        self.insert(item)
                    }
                }
            }
            impl ::bitflags::__private::core::iter::FromIterator<InternalBitFlags>
            for InternalBitFlags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn from_iter<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(iterator: T) -> Self {
                    use ::bitflags::__private::core::iter::Extend;
                    let mut result = Self::empty();
                    result.extend(iterator);
                    result
                }
            }
            impl InternalBitFlags {
                /// Yield a set of contained flags values.
                ///
                /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
                /// will be yielded together as a final flags value.
                #[inline]
                pub const fn iter(&self) -> ::bitflags::iter::Iter<Flags> {
                    ::bitflags::iter::Iter::__private_const_new(
                        <Flags as ::bitflags::Flags>::FLAGS,
                        Flags::from_bits_retain(self.bits()),
                        Flags::from_bits_retain(self.bits()),
                    )
                }
                /// Yield a set of contained named flags values.
                ///
                /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
                /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
                #[inline]
                pub const fn iter_names(&self) -> ::bitflags::iter::IterNames<Flags> {
                    ::bitflags::iter::IterNames::__private_const_new(
                        <Flags as ::bitflags::Flags>::FLAGS,
                        Flags::from_bits_retain(self.bits()),
                        Flags::from_bits_retain(self.bits()),
                    )
                }
            }
            impl ::bitflags::__private::core::iter::IntoIterator for InternalBitFlags {
                type Item = Flags;
                type IntoIter = ::bitflags::iter::Iter<Flags>;
                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
            impl InternalBitFlags {
                /// Returns a mutable reference to the raw value of the flags currently stored.
                #[inline]
                pub fn bits_mut(&mut self) -> &mut u32 {
                    &mut self.0
                }
            }
            #[allow(dead_code, deprecated, unused_attributes)]
            impl Flags {
                /// Get a flags value with all bits unset.
                #[inline]
                pub const fn empty() -> Self {
                    { Self(InternalBitFlags::empty()) }
                }
                /// Get a flags value with all known bits set.
                #[inline]
                pub const fn all() -> Self {
                    { Self(InternalBitFlags::all()) }
                }
                /// Get the underlying bits value.
                ///
                /// The returned value is exactly the bits set in this flags value.
                #[inline]
                pub const fn bits(&self) -> u32 {
                    let f = self;
                    { f.0.bits() }
                }
                /// Convert from a bits value.
                ///
                /// This method will return `None` if any unknown bits are set.
                #[inline]
                pub const fn from_bits(
                    bits: u32,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let bits = bits;
                    {
                        match InternalBitFlags::from_bits(bits) {
                            ::bitflags::__private::core::option::Option::Some(bits) => {
                                ::bitflags::__private::core::option::Option::Some(
                                    Self(bits),
                                )
                            }
                            ::bitflags::__private::core::option::Option::None => {
                                ::bitflags::__private::core::option::Option::None
                            }
                        }
                    }
                }
                /// Convert from a bits value, unsetting any unknown bits.
                #[inline]
                pub const fn from_bits_truncate(bits: u32) -> Self {
                    let bits = bits;
                    { Self(InternalBitFlags::from_bits_truncate(bits)) }
                }
                /// Convert from a bits value exactly.
                #[inline]
                pub const fn from_bits_retain(bits: u32) -> Self {
                    let bits = bits;
                    { Self(InternalBitFlags::from_bits_retain(bits)) }
                }
                /// Get a flags value with the bits of a flag with the given name set.
                ///
                /// This method will return `None` if `name` is empty or doesn't
                /// correspond to any named flag.
                #[inline]
                pub fn from_name(
                    name: &str,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let name = name;
                    {
                        match InternalBitFlags::from_name(name) {
                            ::bitflags::__private::core::option::Option::Some(bits) => {
                                ::bitflags::__private::core::option::Option::Some(
                                    Self(bits),
                                )
                            }
                            ::bitflags::__private::core::option::Option::None => {
                                ::bitflags::__private::core::option::Option::None
                            }
                        }
                    }
                }
                /// Whether all bits in this flags value are unset.
                #[inline]
                pub const fn is_empty(&self) -> bool {
                    let f = self;
                    { f.0.is_empty() }
                }
                /// Whether all known bits in this flags value are set.
                #[inline]
                pub const fn is_all(&self) -> bool {
                    let f = self;
                    { f.0.is_all() }
                }
                /// Whether any set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn intersects(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.0.intersects(other.0) }
                }
                /// Whether all set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn contains(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.0.contains(other.0) }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                pub fn insert(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    { f.0.insert(other.0) }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `remove` won't truncate `other`, but the `!` operator will.
                #[inline]
                pub fn remove(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    { f.0.remove(other.0) }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                pub fn toggle(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    { f.0.toggle(other.0) }
                }
                /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
                #[inline]
                pub fn set(&mut self, other: Self, value: bool) {
                    let f = self;
                    let other = other;
                    let value = value;
                    { f.0.set(other.0, value) }
                }
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn intersection(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.intersection(other.0)) }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn union(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.union(other.0)) }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                #[must_use]
                pub const fn difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.difference(other.0)) }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn symmetric_difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.symmetric_difference(other.0)) }
                }
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                #[must_use]
                pub const fn complement(self) -> Self {
                    let f = self;
                    { Self(f.0.complement()) }
                }
            }
            impl ::bitflags::__private::core::fmt::Binary for Flags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::Binary::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::fmt::Octal for Flags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::Octal::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::fmt::LowerHex for Flags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::LowerHex::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::fmt::UpperHex for Flags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    let inner = self.0;
                    ::bitflags::__private::core::fmt::UpperHex::fmt(&inner, f)
                }
            }
            impl ::bitflags::__private::core::ops::BitOr for Flags {
                type Output = Self;
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor(self, other: Flags) -> Self {
                    self.union(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitOrAssign for Flags {
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor_assign(&mut self, other: Self) {
                    self.insert(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitXor for Flags {
                type Output = Self;
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor(self, other: Self) -> Self {
                    self.symmetric_difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitXorAssign for Flags {
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor_assign(&mut self, other: Self) {
                    self.toggle(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitAnd for Flags {
                type Output = Self;
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand(self, other: Self) -> Self {
                    self.intersection(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitAndAssign for Flags {
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand_assign(&mut self, other: Self) {
                    *self = Self::from_bits_retain(self.bits()).intersection(other);
                }
            }
            impl ::bitflags::__private::core::ops::Sub for Flags {
                type Output = Self;
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub(self, other: Self) -> Self {
                    self.difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::SubAssign for Flags {
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub_assign(&mut self, other: Self) {
                    self.remove(other);
                }
            }
            impl ::bitflags::__private::core::ops::Not for Flags {
                type Output = Self;
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                fn not(self) -> Self {
                    self.complement()
                }
            }
            impl ::bitflags::__private::core::iter::Extend<Flags> for Flags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn extend<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(&mut self, iterator: T) {
                    for item in iterator {
                        self.insert(item)
                    }
                }
            }
            impl ::bitflags::__private::core::iter::FromIterator<Flags> for Flags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn from_iter<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(iterator: T) -> Self {
                    use ::bitflags::__private::core::iter::Extend;
                    let mut result = Self::empty();
                    result.extend(iterator);
                    result
                }
            }
            impl Flags {
                /// Yield a set of contained flags values.
                ///
                /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
                /// will be yielded together as a final flags value.
                #[inline]
                pub const fn iter(&self) -> ::bitflags::iter::Iter<Flags> {
                    ::bitflags::iter::Iter::__private_const_new(
                        <Flags as ::bitflags::Flags>::FLAGS,
                        Flags::from_bits_retain(self.bits()),
                        Flags::from_bits_retain(self.bits()),
                    )
                }
                /// Yield a set of contained named flags values.
                ///
                /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
                /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
                #[inline]
                pub const fn iter_names(&self) -> ::bitflags::iter::IterNames<Flags> {
                    ::bitflags::iter::IterNames::__private_const_new(
                        <Flags as ::bitflags::Flags>::FLAGS,
                        Flags::from_bits_retain(self.bits()),
                        Flags::from_bits_retain(self.bits()),
                    )
                }
            }
            impl ::bitflags::__private::core::iter::IntoIterator for Flags {
                type Item = Flags;
                type IntoIter = ::bitflags::iter::Iter<Flags>;
                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
        };
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
        /// The [alignment](maligned::Alignment) by the [`Memory`](MemBackend::Memory) variant of [`MemBackend`].
        pub type MemoryAlignment = A64;
        /// Possible backends of a [`MemCase`]. The `None` variant is used when the data structure is
        /// created in memory; the `Memory` variant is used when the data structure is deserialized
        /// from a file loaded into a heap-allocated memory region; the `Mmap` variant is used when
        /// the data structure is deserialized from a `mmap()`-based region, either coming from
        /// an allocation or a from mapping a file.
        pub enum MemBackend {
            /// No backend. The data structure is a standard Rust data structure.
            /// This variant is returned by [`MemCase::encase`].
            None,
            /// The backend is a heap-allocated in a memory region aligned to 16 bytes.
            /// This variant is returned by [`crate::deser::Deserialize::load_mem`].
            Memory(Box<[MemoryAlignment]>),
            /// The backend is the result to a call to `mmap()`.
            /// This variant is returned by [`crate::deser::Deserialize::load_mmap`] and [`crate::deser::Deserialize::mmap`].
            Mmap(mmap_rs::Mmap),
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for MemBackend {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    MemBackend::None => ::core::fmt::Formatter::write_str(f, "None"),
                    MemBackend::Memory(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Memory",
                            &__self_0,
                        )
                    }
                    MemBackend::Mmap(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Mmap",
                            &__self_0,
                        )
                    }
                }
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for MemBackend {
            #[inline]
            fn clone(&self) -> MemBackend {
                match self {
                    MemBackend::None => MemBackend::None,
                    MemBackend::Memory(__self_0) => {
                        MemBackend::Memory(::core::clone::Clone::clone(__self_0))
                    }
                    MemBackend::Mmap(__self_0) => {
                        MemBackend::Mmap(::core::clone::Clone::clone(__self_0))
                    }
                }
            }
        }
        #[automatically_derived]
        impl mem_dbg::MemDbgImpl for MemBackend
        where
            Box<[MemoryAlignment]>: mem_dbg::MemDbgImpl,
            mmap_rs::Mmap: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                let mut _memdbg_digits_number = mem_dbg::utils::n_of_digits(
                    _memdbg_total_size,
                );
                if _memdbg_flags.contains(mem_dbg::DbgFlags::SEPARATOR) {
                    _memdbg_digits_number += _memdbg_digits_number / 3;
                }
                if _memdbg_flags.contains(mem_dbg::DbgFlags::HUMANIZE) {
                    _memdbg_digits_number = 6;
                }
                if _memdbg_flags.contains(mem_dbg::DbgFlags::PERCENTAGE) {
                    _memdbg_digits_number += 8;
                }
                for _ in 0.._memdbg_digits_number + 3 {
                    _memdbg_writer.write_char(' ')?;
                }
                if !_memdbg_prefix.is_empty() {
                    _memdbg_writer.write_str(&_memdbg_prefix[2..])?;
                }
                match self {
                    MemBackend::None => {
                        _memdbg_writer.write_char('╰')?;
                        _memdbg_writer.write_char('╴')?;
                        _memdbg_writer.write_str("Variant: None\n")?;
                    }
                    MemBackend::Memory(v0) => {
                        _memdbg_writer.write_char('├')?;
                        _memdbg_writer.write_char('╴')?;
                        _memdbg_writer.write_str("Variant: Memory\n")?;
                        v0.mem_dbg_depth_on(
                            _memdbg_writer,
                            _memdbg_total_size,
                            _memdbg_max_depth,
                            _memdbg_prefix,
                            Some("0"),
                            true,
                            _memdbg_flags,
                        )?;
                    }
                    MemBackend::Mmap(v0) => {
                        _memdbg_writer.write_char('├')?;
                        _memdbg_writer.write_char('╴')?;
                        _memdbg_writer.write_str("Variant: Mmap\n")?;
                        v0.mem_dbg_depth_on(
                            _memdbg_writer,
                            _memdbg_total_size,
                            _memdbg_max_depth,
                            _memdbg_prefix,
                            Some("0"),
                            true,
                            _memdbg_flags,
                        )?;
                    }
                }
                Ok(())
            }
        }
        #[automatically_derived]
        impl mem_dbg::CopyType for MemBackend
        where
            Box<[MemoryAlignment]>: mem_dbg::MemSize,
            mmap_rs::Mmap: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl mem_dbg::MemSize for MemBackend
        where
            Box<[MemoryAlignment]>: mem_dbg::MemSize,
            mmap_rs::Mmap: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                match self {
                    MemBackend::None => core::mem::size_of::<Self>(),
                    MemBackend::Memory(v0) => {
                        core::mem::size_of::<Self>() + v0.mem_size(_memsize_flags)
                            - core::mem::size_of::<Box<[MemoryAlignment]>>()
                    }
                    MemBackend::Mmap(v0) => {
                        core::mem::size_of::<Self>() + v0.mem_size(_memsize_flags)
                            - core::mem::size_of::<mmap_rs::Mmap>()
                    }
                }
            }
        }
        impl MemBackend {
            pub fn as_ref(&self) -> Option<&[u8]> {
                match self {
                    MemBackend::None => None,
                    MemBackend::Memory(mem) => {
                        Some(unsafe {
                            core::slice::from_raw_parts(
                                mem.as_ptr() as *const MemoryAlignment as *const u8,
                                mem.len() * size_of::<MemoryAlignment>(),
                            )
                        })
                    }
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
        #[automatically_derived]
        impl<S: ::core::fmt::Debug> ::core::fmt::Debug for MemCase<S> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field2_finish(
                    f,
                    "MemCase",
                    &self.0,
                    &&self.1,
                )
            }
        }
        #[automatically_derived]
        impl<S: ::core::clone::Clone> ::core::clone::Clone for MemCase<S> {
            #[inline]
            fn clone(&self) -> MemCase<S> {
                MemCase(
                    ::core::clone::Clone::clone(&self.0),
                    ::core::clone::Clone::clone(&self.1),
                )
            }
        }
        #[automatically_derived]
        impl<S> mem_dbg::MemDbgImpl for MemCase<S>
        where
            S: mem_dbg::MemDbgImpl,
            MemBackend: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.0
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("0"),
                        false,
                        _memdbg_flags,
                    )?;
                self.1
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("1"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl<S> mem_dbg::CopyType for MemCase<S>
        where
            S: mem_dbg::MemSize,
            MemBackend: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl<S> mem_dbg::MemSize for MemCase<S>
        where
            S: mem_dbg::MemSize,
            MemBackend: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes += self.0.mem_size(_memsize_flags) - core::mem::size_of::<S>();
                bytes
                    += self.1.mem_size(_memsize_flags)
                        - core::mem::size_of::<MemBackend>();
                bytes
            }
        }
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
    }
    pub use mem_case::*;
    pub mod read {
        /*!

No-std support for reading while keeping track of the current position.

 */
        use crate::prelude::*;
        /// [`std::io::Read`]-like trait for serialization that does not
        /// depend on [`std`].
        ///
        /// In an [`std`] context, the user does not need to use directly
        /// this trait as we provide a blanket
        /// implementation that implements [`ReadNoStd`] for all types that implement
        /// [`std::io::Read`]. In particular, in such a context you can use [`std::io::Cursor`]
        /// for in-memory deserialization.
        pub trait ReadNoStd {
            /// Read some bytes
            fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()>;
        }
        #[cfg(feature = "std")]
        use std::io::Read;
        #[cfg(feature = "std")]
        impl<W: Read> ReadNoStd for W {
            #[inline(always)]
            fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
                Read::read_exact(self, buf).map_err(|_| deser::Error::ReadError)
            }
        }
        /// A trait for [`ReadNoStd`] that also keeps track of the current position.
        ///
        /// This is needed because the [`Read`] trait doesn't have a `seek` method and
        /// [`std::io::Seek`] would be a requirement much stronger than needed.
        pub trait ReadWithPos: ReadNoStd + Sized {
            /// Return the current position.
            fn pos(&self) -> usize;
            /// Pad the cursor to the next multiple of [`MaxSizeOf::max_size_of`] 'T'.
            fn align<T: MaxSizeOf>(&mut self) -> deser::Result<()>;
        }
    }
    pub use read::*;
    pub mod reader_with_pos {
        use crate::prelude::*;
        use super::ReadNoStd;
        use mem_dbg::{MemDbg, MemSize};
        /// A wrapper for a [`ReadNoStd`] that implements [`ReadWithPos`]
        /// by keeping track of the current position.
        pub struct ReaderWithPos<'a, F: ReadNoStd> {
            /// What we actually readfrom
            backend: &'a mut F,
            /// How many bytes we have read from the start
            pos: usize,
        }
        #[automatically_derived]
        impl<'a, F: ::core::fmt::Debug + ReadNoStd> ::core::fmt::Debug
        for ReaderWithPos<'a, F> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "ReaderWithPos",
                    "backend",
                    &self.backend,
                    "pos",
                    &&self.pos,
                )
            }
        }
        #[automatically_derived]
        impl<'a, F: ::core::clone::Clone + ReadNoStd> ::core::clone::Clone
        for ReaderWithPos<'a, F> {
            #[inline]
            fn clone(&self) -> ReaderWithPos<'a, F> {
                ReaderWithPos {
                    backend: ::core::clone::Clone::clone(&self.backend),
                    pos: ::core::clone::Clone::clone(&self.pos),
                }
            }
        }
        #[automatically_derived]
        impl<'a, F: ReadNoStd> mem_dbg::MemDbgImpl for ReaderWithPos<'a, F>
        where
            &'a mut F: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.backend
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("backend"),
                        false,
                        _memdbg_flags,
                    )?;
                self.pos
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("pos"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl<'a, F: ReadNoStd> mem_dbg::CopyType for ReaderWithPos<'a, F>
        where
            &'a mut F: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl<'a, F: ReadNoStd> mem_dbg::MemSize for ReaderWithPos<'a, F>
        where
            &'a mut F: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.backend.mem_size(_memsize_flags)
                        - core::mem::size_of::<&'a mut F>();
                bytes
                    += self.pos.mem_size(_memsize_flags) - core::mem::size_of::<usize>();
                bytes
            }
        }
        impl<'a, F: ReadNoStd> ReaderWithPos<'a, F> {
            #[inline(always)]
            /// Create a new [`ReadWithPos`] on top of a generic [`ReadNoStd`].
            pub fn new(backend: &'a mut F) -> Self {
                Self { backend, pos: 0 }
            }
        }
        impl<'a, F: ReadNoStd> ReadNoStd for ReaderWithPos<'a, F> {
            fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
                self.backend.read_exact(buf)?;
                self.pos += buf.len();
                Ok(())
            }
        }
        impl<'a, F: ReadNoStd> ReadWithPos for ReaderWithPos<'a, F> {
            fn pos(&self) -> usize {
                self.pos
            }
            fn align<T: MaxSizeOf>(&mut self) -> deser::Result<()> {
                let padding = crate::pad_align_to(self.pos, T::max_size_of());
                self.read_exact(&mut ::alloc::vec::from_elem(0, padding))?;
                Ok(())
            }
        }
    }
    pub use reader_with_pos::*;
    pub mod slice_with_pos {
        use super::*;
        use crate::prelude::*;
        use mem_dbg::{MemDbg, MemSize};
        /// [`std::io::Cursor`]-like trait for deserialization that does not
        /// depend on [`std`].
        pub struct SliceWithPos<'a> {
            pub data: &'a [u8],
            pub pos: usize,
        }
        #[automatically_derived]
        impl<'a> ::core::fmt::Debug for SliceWithPos<'a> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "SliceWithPos",
                    "data",
                    &self.data,
                    "pos",
                    &&self.pos,
                )
            }
        }
        #[automatically_derived]
        impl<'a> ::core::clone::Clone for SliceWithPos<'a> {
            #[inline]
            fn clone(&self) -> SliceWithPos<'a> {
                SliceWithPos {
                    data: ::core::clone::Clone::clone(&self.data),
                    pos: ::core::clone::Clone::clone(&self.pos),
                }
            }
        }
        #[automatically_derived]
        impl<'a> mem_dbg::MemDbgImpl for SliceWithPos<'a>
        where
            &'a [u8]: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.data
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("data"),
                        false,
                        _memdbg_flags,
                    )?;
                self.pos
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("pos"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl<'a> mem_dbg::CopyType for SliceWithPos<'a>
        where
            &'a [u8]: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl<'a> mem_dbg::MemSize for SliceWithPos<'a>
        where
            &'a [u8]: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.data.mem_size(_memsize_flags)
                        - core::mem::size_of::<&'a [u8]>();
                bytes
                    += self.pos.mem_size(_memsize_flags) - core::mem::size_of::<usize>();
                bytes
            }
        }
        impl<'a> SliceWithPos<'a> {
            pub fn new(backend: &'a [u8]) -> Self {
                Self { data: backend, pos: 0 }
            }
            pub fn skip(&mut self, bytes: usize) {
                self.data = &self.data[bytes..];
                self.pos += bytes;
            }
        }
        impl<'a> ReadNoStd for SliceWithPos<'a> {
            fn read_exact(&mut self, buf: &mut [u8]) -> deser::Result<()> {
                let len = buf.len();
                if len > self.data.len() {
                    return Err(Error::ReadError);
                }
                buf.copy_from_slice(&self.data[..len]);
                self.data = &self.data[len..];
                self.pos += len;
                Ok(())
            }
        }
        impl<'a> ReadWithPos for SliceWithPos<'a> {
            fn pos(&self) -> usize {
                self.pos
            }
            /// Pad the cursor to the correct alignment.
            ///
            /// Note that this method also checks that
            /// the absolute memory position is properly aligned.
            fn align<T: MaxSizeOf>(&mut self) -> deser::Result<()> {
                let padding = crate::pad_align_to(self.pos, T::max_size_of());
                self.skip(padding);
                if self.data.as_ptr() as usize % T::max_size_of() != 0 {
                    Err(Error::AlignmentError)
                } else {
                    Ok(())
                }
            }
        }
    }
    pub use slice_with_pos::*;
    pub type Result<T> = core::result::Result<T, Error>;
    /// A shorthand for the [deserialized type associated with a type](DeserializeInner::DeserType).
    pub type DeserType<'a, T> = <T as DeserializeInner>::DeserType<'a>;
    /// Main deserialization trait. It is separated from [`DeserializeInner`] to
    /// avoid that the user modify its behavior, and hide internal serialization
    /// methods.
    ///
    /// It provides several convenience methods to load or map into memory
    /// structures that have been previously serialized. See, for example,
    /// [`Deserialize::load_full`], [`Deserialize::load_mem`], and [`Deserialize::mmap`].
    pub trait Deserialize: TypeHash + ReprHash + DeserializeInner {
        /// Fully deserialize a structure of this type from the given backend.
        fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self>;
        /// ε-copy deserialize a structure of this type from the given backend.
        fn deserialize_eps(backend: &'_ [u8]) -> Result<Self::DeserType<'_>>;
        /// Commodity method to fully deserialize from a file.
        fn load_full(path: impl AsRef<Path>) -> Result<Self> {
            let file = std::fs::File::open(path).map_err(Error::FileOpenError)?;
            let mut buf_reader = BufReader::new(file);
            Self::deserialize_full(&mut buf_reader)
        }
        /// Load a file into heap-allocated memory and ε-deserialize a data structure from it,
        /// returning a [`MemCase`] containing the data structure and the
        /// memory. Excess bytes are zeroed out.
        ///
        /// The allocated memory will have [`MemoryAlignment`] as alignment: types with
        /// a higher alignment requirement will cause an [alignment error](`Error::AlignmentError`).
        fn load_mem<'a>(
            path: impl AsRef<Path>,
        ) -> anyhow::Result<MemCase<<Self as DeserializeInner>::DeserType<'a>>> {
            let align_to = align_of::<MemoryAlignment>();
            if align_of::<Self>() > align_to {
                return Err(Error::AlignmentError.into());
            }
            let file_len = path.as_ref().metadata()?.len() as usize;
            let mut file = std::fs::File::open(path)?;
            let capacity = file_len + crate::pad_align_to(file_len, align_to);
            let mut uninit: MaybeUninit<
                MemCase<<Self as DeserializeInner>::DeserType<'_>>,
            > = MaybeUninit::uninit();
            let ptr = uninit.as_mut_ptr();
            #[allow(invalid_value)]
            let mut aligned_vec = unsafe {
                <Vec<
                    MemoryAlignment,
                >>::from_raw_parts(
                    std::alloc::alloc(
                        std::alloc::Layout::from_size_align(capacity, align_to)?,
                    ) as *mut MemoryAlignment,
                    capacity / align_to,
                    capacity / align_to,
                )
            };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    aligned_vec.as_mut_ptr() as *mut u8,
                    capacity,
                )
            };
            file.read_exact(&mut bytes[..file_len])?;
            bytes[file_len..].fill(0);
            let backend = MemBackend::Memory(aligned_vec.into_boxed_slice());
            unsafe {
                (&raw mut (*ptr).1).write(backend);
            }
            let mem = unsafe { (*ptr).1.as_ref().unwrap() };
            let s = Self::deserialize_eps(mem)?;
            unsafe {
                (&raw mut (*ptr).0).write(s);
            }
            Ok(unsafe { uninit.assume_init() })
        }
        /// Load a file into `mmap()`-allocated memory and ε-deserialize a data structure from it,
        /// returning a [`MemCase`] containing the data structure and the
        /// memory. Excess bytes are zeroed out.
        ///
        /// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
        /// just pass `Flags::empty()`.
        #[allow(clippy::uninit_vec)]
        fn load_mmap<'a>(
            path: impl AsRef<Path>,
            flags: Flags,
        ) -> anyhow::Result<MemCase<<Self as DeserializeInner>::DeserType<'a>>> {
            let file_len = path.as_ref().metadata()?.len() as usize;
            let mut file = std::fs::File::open(path)?;
            let capacity = file_len + crate::pad_align_to(file_len, 16);
            let mut uninit: MaybeUninit<
                MemCase<<Self as DeserializeInner>::DeserType<'_>>,
            > = MaybeUninit::uninit();
            let ptr = uninit.as_mut_ptr();
            let mut mmap = mmap_rs::MmapOptions::new(capacity)?
                .with_flags(flags.mmap_flags())
                .map_mut()?;
            file.read_exact(&mut mmap[..file_len])?;
            mmap[file_len..].fill(0);
            let backend = MemBackend::Mmap(
                mmap.make_read_only().map_err(|(_, err)| err)?,
            );
            unsafe {
                (&raw mut (*ptr).1).write(backend);
            }
            let mem = unsafe { (*ptr).1.as_ref().unwrap() };
            let s = Self::deserialize_eps(mem)?;
            unsafe {
                (&raw mut (*ptr).0).write(s);
            }
            Ok(unsafe { uninit.assume_init() })
        }
        /// Memory map a file and ε-deserialize a data structure from it,
        /// returning a [`MemCase`] containing the data structure and the
        /// memory mapping.
        ///
        /// The behavior of `mmap()` can be modified by passing some [`Flags`]; otherwise,
        /// just pass `Flags::empty()`.
        #[allow(clippy::uninit_vec)]
        fn mmap<'a>(
            path: impl AsRef<Path>,
            flags: Flags,
        ) -> anyhow::Result<MemCase<<Self as DeserializeInner>::DeserType<'a>>> {
            let file_len = path.as_ref().metadata()?.len();
            let file = std::fs::File::open(path)?;
            let mut uninit: MaybeUninit<
                MemCase<<Self as DeserializeInner>::DeserType<'_>>,
            > = MaybeUninit::uninit();
            let ptr = uninit.as_mut_ptr();
            let mmap = unsafe {
                mmap_rs::MmapOptions::new(file_len as _)?
                    .with_flags(flags.mmap_flags())
                    .with_file(&file, 0)
                    .map()?
            };
            unsafe {
                (&raw mut (*ptr).1).write(MemBackend::Mmap(mmap));
            }
            let mmap = unsafe { (*ptr).1.as_ref().unwrap() };
            let s = Self::deserialize_eps(mmap)?;
            unsafe {
                (&raw mut (*ptr).0).write(s);
            }
            Ok(unsafe { uninit.assume_init() })
        }
    }
    /// Inner trait to implement deserialization of a type. This trait exists
    /// to separate the user-facing [`Deserialize`] trait from the low-level
    /// deserialization mechanisms of [`DeserializeInner::_deserialize_full_inner`]
    /// and [`DeserializeInner::_deserialize_eps_inner`]. Moreover,
    /// it makes it possible to behave slighly differently at the top
    /// of the recursion tree (e.g., to check the endianness marker), and to prevent
    /// the user from modifying the methods in [`Deserialize`].
    ///
    /// The user should not implement this trait directly, but rather derive it.
    pub trait DeserializeInner: Sized {
        /// The deserialization type associated with this type. It can be
        /// retrieved conveniently with the alias [`DeserType`].
        type DeserType<'a>;
        fn _deserialize_full_inner(backend: &mut impl ReadWithPos) -> Result<Self>;
        fn _deserialize_eps_inner<'a>(
            backend: &mut SliceWithPos<'a>,
        ) -> Result<Self::DeserType<'a>>;
    }
    /// Blanket implementation that prevents the user from overwriting the
    /// methods in [`Deserialize`].
    ///
    /// This implementation [checks the header](`check_header`) written
    /// by the blanket implementation of [`crate::ser::Serialize`] and then delegates to
    /// [`DeserializeInner::_deserialize_full_inner`] or
    /// [`DeserializeInner::_deserialize_eps_inner`].
    impl<T: TypeHash + ReprHash + DeserializeInner> Deserialize for T {
        fn deserialize_full(backend: &mut impl ReadNoStd) -> Result<Self> {
            let mut backend = ReaderWithPos::new(backend);
            check_header::<Self>(&mut backend)?;
            Self::_deserialize_full_inner(&mut backend)
        }
        fn deserialize_eps(backend: &'_ [u8]) -> Result<Self::DeserType<'_>> {
            let mut backend = SliceWithPos::new(backend);
            check_header::<Self>(&mut backend)?;
            Self::_deserialize_eps_inner(&mut backend)
        }
    }
    /// Common header check code for both ε-copy and full-copy deserialization.
    ///
    /// Must be kept in sync with [`crate::ser::write_header`].
    pub fn check_header<T: Deserialize>(backend: &mut impl ReadWithPos) -> Result<()> {
        let self_type_name = core::any::type_name::<T>().to_string();
        let mut type_hasher = xxhash_rust::xxh3::Xxh3::new();
        T::type_hash(&mut type_hasher);
        let self_type_hash = type_hasher.finish();
        let mut repr_hasher = xxhash_rust::xxh3::Xxh3::new();
        let mut offset_of = 0;
        T::repr_hash(&mut repr_hasher, &mut offset_of);
        let self_repr_hash = repr_hasher.finish();
        let magic = u64::_deserialize_full_inner(backend)?;
        match magic {
            MAGIC => Ok(()),
            MAGIC_REV => Err(Error::EndiannessError),
            magic => Err(Error::MagicCookieError(magic)),
        }?;
        let major = u16::_deserialize_full_inner(backend)?;
        if major != VERSION.0 {
            return Err(Error::MajorVersionMismatch(major));
        }
        let minor = u16::_deserialize_full_inner(backend)?;
        if minor > VERSION.1 {
            return Err(Error::MinorVersionMismatch(minor));
        }
        let usize_size = u8::_deserialize_full_inner(backend)?;
        let usize_size = usize_size as usize;
        let native_usize_size = core::mem::size_of::<usize>();
        if usize_size != native_usize_size {
            return Err(Error::UsizeSizeMismatch(usize_size));
        }
        let ser_type_hash = u64::_deserialize_full_inner(backend)?;
        let ser_repr_hash = u64::_deserialize_full_inner(backend)?;
        let ser_type_name = String::_deserialize_full_inner(backend)?;
        if ser_type_hash != self_type_hash {
            return Err(Error::WrongTypeHash {
                got_type_name: self_type_name,
                got: self_type_hash,
                expected_type_name: ser_type_name,
                expected: ser_type_hash,
            });
        }
        if ser_repr_hash != self_repr_hash {
            return Err(Error::WrongTypeReprHash {
                got_type_name: self_type_name,
                got: self_repr_hash,
                expected_type_name: ser_type_name,
                expected: ser_repr_hash,
            });
        }
        Ok(())
    }
    /// A helper trait that makes it possible to implement differently
    /// deserialization for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types.
    /// See [`crate::traits::CopyType`] for more information.
    pub trait DeserializeHelper<T: CopySelector> {
        type FullType;
        type DeserType<'a>;
        fn _deserialize_full_inner_impl(
            backend: &mut impl ReadWithPos,
        ) -> Result<Self::FullType>;
        fn _deserialize_eps_inner_impl<'a>(
            backend: &mut SliceWithPos<'a>,
        ) -> Result<Self::DeserType<'a>>;
    }
    /// Errors that can happen during deserialization.
    pub enum Error {
        /// [`Deserialize::load_full`] could not open the provided file.
        FileOpenError(std::io::Error),
        /// The underlying reader returned an error.
        ReadError,
        /// The file is from ε-serde but the endianess is wrong.
        EndiannessError,
        /// Some fields are not properly aligned.
        AlignmentError,
        /// The file was serialized with a version of ε-serde that is not compatible.
        MajorVersionMismatch(u16),
        /// The file was serialized with a compatible, but too new version of ε-serde
        /// so we might be missing features.
        MinorVersionMismatch(u16),
        /// The the `pointer_width` of the serialized file is different from the
        /// `pointer_width` of the current architecture.
        /// For example, the file was serialized on a 64-bit machine and we are trying to
        /// deserialize it on a 32-bit machine.
        UsizeSizeMismatch(usize),
        /// The magic coookie is wrong. The byte sequence does not come from ε-serde.
        MagicCookieError(u64),
        /// A tag is wrong (e.g., for [`Option`]).
        InvalidTag(usize),
        /// The type hash is wrong. Probably the user is trying to deserialize a
        /// file with the wrong type.
        WrongTypeHash {
            got_type_name: String,
            expected_type_name: String,
            expected: u64,
            got: u64,
        },
        /// The type representation hash is wrong. Probabliy the user is trying to
        /// deserialize a file with some zero-copy type that has different
        /// in-memory representations on the serialization arch and on the current one,
        /// usually because of alignment issues.
        WrongTypeReprHash {
            got_type_name: String,
            expected_type_name: String,
            expected: u64,
            got: u64,
        },
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Error {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Error::FileOpenError(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "FileOpenError",
                        &__self_0,
                    )
                }
                Error::ReadError => ::core::fmt::Formatter::write_str(f, "ReadError"),
                Error::EndiannessError => {
                    ::core::fmt::Formatter::write_str(f, "EndiannessError")
                }
                Error::AlignmentError => {
                    ::core::fmt::Formatter::write_str(f, "AlignmentError")
                }
                Error::MajorVersionMismatch(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "MajorVersionMismatch",
                        &__self_0,
                    )
                }
                Error::MinorVersionMismatch(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "MinorVersionMismatch",
                        &__self_0,
                    )
                }
                Error::UsizeSizeMismatch(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "UsizeSizeMismatch",
                        &__self_0,
                    )
                }
                Error::MagicCookieError(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "MagicCookieError",
                        &__self_0,
                    )
                }
                Error::InvalidTag(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "InvalidTag",
                        &__self_0,
                    )
                }
                Error::WrongTypeHash {
                    got_type_name: __self_0,
                    expected_type_name: __self_1,
                    expected: __self_2,
                    got: __self_3,
                } => {
                    ::core::fmt::Formatter::debug_struct_field4_finish(
                        f,
                        "WrongTypeHash",
                        "got_type_name",
                        __self_0,
                        "expected_type_name",
                        __self_1,
                        "expected",
                        __self_2,
                        "got",
                        &__self_3,
                    )
                }
                Error::WrongTypeReprHash {
                    got_type_name: __self_0,
                    expected_type_name: __self_1,
                    expected: __self_2,
                    got: __self_3,
                } => {
                    ::core::fmt::Formatter::debug_struct_field4_finish(
                        f,
                        "WrongTypeReprHash",
                        "got_type_name",
                        __self_0,
                        "expected_type_name",
                        __self_1,
                        "expected",
                        __self_2,
                        "got",
                        &__self_3,
                    )
                }
            }
        }
    }
    impl std::error::Error for Error {}
    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                Self::ReadError => {
                    f.write_fmt(
                        format_args!("Read error during ε-serde deserialization"),
                    )
                }
                Self::FileOpenError(error) => {
                    f.write_fmt(
                        format_args!(
                            "Error opening file during ε-serde deserialization: {0}",
                            error,
                        ),
                    )
                }
                Self::EndiannessError => {
                    f.write_fmt(
                        format_args!(
                            "The current arch is {0}-endian but the data is {1}-endian.",
                            if true { "little" } else { "big" },
                            if true { "big" } else { "little" },
                        ),
                    )
                }
                Self::MagicCookieError(magic) => {
                    f.write_fmt(
                        format_args!(
                            "Wrong magic cookie 0x{0:016x}. The byte stream does not come from ε-serde.",
                            magic,
                        ),
                    )
                }
                Self::MajorVersionMismatch(found_major) => {
                    f.write_fmt(
                        format_args!(
                            "Major version mismatch. Expected {0} but got {1}.",
                            VERSION.0,
                            found_major,
                        ),
                    )
                }
                Self::MinorVersionMismatch(found_minor) => {
                    f.write_fmt(
                        format_args!(
                            "Minor version mismatch. Expected {0} but got {1}.",
                            VERSION.1,
                            found_minor,
                        ),
                    )
                }
                Self::UsizeSizeMismatch(usize_size) => {
                    f.write_fmt(
                        format_args!(
                            "The file was serialized on an architecture where a usize has size {0}, but on the current architecture it has size {1}.",
                            usize_size,
                            core::mem::size_of::<usize>(),
                        ),
                    )
                }
                Self::AlignmentError => {
                    f.write_fmt(
                        format_args!(
                            "Alignment error. Most likely you are deserializing from a memory region with insufficient alignment.",
                        ),
                    )
                }
                Self::InvalidTag(tag) => {
                    f.write_fmt(format_args!("Invalid tag: 0x{0:02x}", tag))
                }
                Self::WrongTypeHash {
                    got_type_name,
                    expected_type_name,
                    expected,
                    got,
                } => {
                    f.write_fmt(
                        format_args!(
                            "Wrong type hash. Expected: 0x{0:016x} Actual: 0x{1:016x}.\nYou are trying to deserialize a file with the wrong type.\nThe serialized type is \'{2}\' and the deserialized type is \'{3}\'.",
                            expected,
                            got,
                            expected_type_name,
                            got_type_name,
                        ),
                    )
                }
                Self::WrongTypeReprHash {
                    got_type_name,
                    expected_type_name,
                    expected,
                    got,
                } => {
                    f.write_fmt(
                        format_args!(
                            "Wrong type repr hash. Expected: 0x{0:016x} Actual: 0x{1:016x}.\nYou might be trying to deserialize a file that was serialized on an architecture with different alignment requirements, or some of the fields of the type have changed their copy type (zero or deep).\nThe serialized type is \'{2}\' and the deserialized type is \'{3}\'.",
                            expected,
                            got,
                            expected_type_name,
                            got_type_name,
                        ),
                    )
                }
            }
        }
    }
}
pub mod impls {
    /*!

Implementations of [`SerializeInner`](crate::ser::SerializeInner)
and [`DeserializeInner`](crate::deser::DeserializeInner) for standard Rust types.

*/
    pub mod array {
        /*!

Implementations for arrays.

*/
        use crate::prelude::*;
        use core::hash::Hash;
        use core::mem::MaybeUninit;
        use deser::*;
        use ser::*;
        impl<T: CopyType, const N: usize> CopyType for [T; N] {
            type Copy = T::Copy;
        }
        impl<T: TypeHash, const N: usize> TypeHash for [T; N] {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "[]".hash(hasher);
                hasher.write_usize(N);
                T::type_hash(hasher);
            }
        }
        impl<T: Sized, const N: usize> ReprHash for [T; N] {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl<T: MaxSizeOf, const N: usize> MaxSizeOf for [T; N] {
            fn max_size_of() -> usize {
                T::max_size_of()
            }
        }
        impl<T: CopyType + SerializeInner + TypeHash, const N: usize> SerializeInner
        for [T; N]
        where
            [T; N]: SerializeHelper<<T as CopyType>::Copy>,
        {
            const IS_ZERO_COPY: bool = T::IS_ZERO_COPY;
            const ZERO_COPY_MISMATCH: bool = T::ZERO_COPY_MISMATCH;
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                SerializeHelper::_serialize_inner(self, backend)
            }
        }
        impl<
            T: ZeroCopy + SerializeInner + TypeHash,
            const N: usize,
        > SerializeHelper<Zero> for [T; N] {
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<T: DeepCopy + SerializeInner, const N: usize> SerializeHelper<Deep>
        for [T; N] {
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                for item in self.iter() {
                    backend.write("item", item)?;
                }
                Ok(())
            }
        }
        impl<T: CopyType + DeserializeInner + 'static, const N: usize> DeserializeInner
        for [T; N]
        where
            [T; N]: DeserializeHelper<<T as CopyType>::Copy, FullType = [T; N]>,
        {
            type DeserType<'a> = <[T; N] as DeserializeHelper<
                <T as CopyType>::Copy,
            >>::DeserType<'a>;
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                <[T; N] as DeserializeHelper<
                    <T as CopyType>::Copy,
                >>::_deserialize_full_inner_impl(backend)
            }
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<
                <[T; N] as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>,
            > {
                <[T; N] as DeserializeHelper<
                    <T as CopyType>::Copy,
                >>::_deserialize_eps_inner_impl(backend)
            }
        }
        impl<
            T: ZeroCopy + DeserializeInner + 'static,
            const N: usize,
        > DeserializeHelper<Zero> for [T; N] {
            type FullType = Self;
            type DeserType<'a> = &'a [T; N];
            #[inline(always)]
            fn _deserialize_full_inner_impl(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                let mut res = MaybeUninit::<[T; N]>::uninit();
                backend.align::<T>()?;
                unsafe {
                    backend.read_exact(res.assume_init_mut().align_to_mut::<u8>().1)?;
                    Ok(res.assume_init())
                }
            }
            #[inline(always)]
            fn _deserialize_eps_inner_impl<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
                backend.align::<T>()?;
                let bytes = std::mem::size_of::<[T; N]>();
                let (pre, data, after) = unsafe {
                    backend.data[..bytes].align_to::<[T; N]>()
                };
                if true {
                    if !pre.is_empty() {
                        ::core::panicking::panic("assertion failed: pre.is_empty()")
                    }
                }
                if true {
                    if !after.is_empty() {
                        ::core::panicking::panic("assertion failed: after.is_empty()")
                    }
                }
                let res = &data[0];
                backend.skip(bytes);
                Ok(res)
            }
        }
        impl<
            T: DeepCopy + DeserializeInner + 'static,
            const N: usize,
        > DeserializeHelper<Deep> for [T; N] {
            type FullType = Self;
            type DeserType<'a> = [<T as DeserializeInner>::DeserType<'a>; N];
            #[inline(always)]
            fn _deserialize_full_inner_impl(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                let mut res = MaybeUninit::<[T; N]>::uninit();
                unsafe {
                    for item in &mut res.assume_init_mut().iter_mut() {
                        std::ptr::write(item, T::_deserialize_full_inner(backend)?);
                    }
                    Ok(res.assume_init())
                }
            }
            #[inline(always)]
            fn _deserialize_eps_inner_impl<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
                let mut res = MaybeUninit::<
                    <Self as DeserializeInner>::DeserType<'_>,
                >::uninit();
                unsafe {
                    for item in &mut res.assume_init_mut().iter_mut() {
                        std::ptr::write(item, T::_deserialize_eps_inner(backend)?);
                    }
                    Ok(res.assume_init())
                }
            }
        }
    }
    pub mod boxed_slice {
        /*!

Implementations for boxed slices.

*/
        use crate::deser::helpers::*;
        use crate::prelude::*;
        use core::hash::Hash;
        use deser::*;
        use ser::*;
        impl<T> CopyType for Box<[T]> {
            type Copy = Deep;
        }
        impl<T: TypeHash> TypeHash for Box<[T]> {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "Box<[]>".hash(hasher);
                T::type_hash(hasher);
            }
        }
        impl<T: ReprHash> ReprHash for Box<[T]> {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                *offset_of = 0;
                T::repr_hash(hasher, offset_of);
            }
        }
        impl<T: CopyType + TypeHash + ReprHash + SerializeInner> SerializeInner
        for Box<[T]>
        where
            Box<[T]>: SerializeHelper<<T as CopyType>::Copy>,
        {
            const IS_ZERO_COPY: bool = false;
            const ZERO_COPY_MISMATCH: bool = false;
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                SerializeHelper::_serialize_inner(self, backend)
            }
        }
        impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Box<[T]> {
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_slice_zero(backend, self)
            }
        }
        impl<T: DeepCopy + SerializeInner> SerializeHelper<Deep> for Box<[T]> {
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_slice_deep(backend, self)
            }
        }
        impl<T: DeserializeInner + CopyType + 'static> DeserializeInner for Box<[T]>
        where
            Box<[T]>: DeserializeHelper<<T as CopyType>::Copy, FullType = Box<[T]>>,
        {
            type DeserType<'a> = <Box<
                [T],
            > as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                <Box<
                    [T],
                > as DeserializeHelper<
                    <T as CopyType>::Copy,
                >>::_deserialize_full_inner_impl(backend)
            }
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<
                <Box<[T]> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>,
            > {
                <Box<
                    [T],
                > as DeserializeHelper<
                    <T as CopyType>::Copy,
                >>::_deserialize_eps_inner_impl(backend)
            }
        }
        impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero>
        for Box<[T]> {
            type FullType = Self;
            type DeserType<'a> = &'a [T];
            #[inline(always)]
            fn _deserialize_full_inner_impl(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                Ok(deserialize_full_vec_zero::<T>(backend)?.into_boxed_slice())
            }
            #[inline(always)]
            fn _deserialize_eps_inner_impl<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
                deserialize_eps_slice_zero(backend)
            }
        }
        impl<T: DeepCopy + DeserializeInner + 'static> DeserializeHelper<Deep>
        for Box<[T]> {
            type FullType = Self;
            type DeserType<'a> = Box<[<T as DeserializeInner>::DeserType<'a>]>;
            #[inline(always)]
            fn _deserialize_full_inner_impl(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                Ok(deserialize_full_vec_deep(backend)?.into_boxed_slice())
            }
            #[inline(always)]
            fn _deserialize_eps_inner_impl<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
                Ok(deserialize_eps_vec_deep::<T>(backend)?.into_boxed_slice())
            }
        }
    }
    pub mod prim {
        /*!

Implementations for primitive types, `()`, [`PhantomData`] and [`Option`].

*/
        use crate::prelude::*;
        use common_traits::NonZero;
        use core::hash::Hash;
        use core::marker::PhantomData;
        use core::mem::size_of;
        use core::num::{
            NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
            NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
        };
        use deser::*;
        use ser::*;
        impl CopyType for isize {
            type Copy = Zero;
        }
        impl TypeHash for isize {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "isize".hash(hasher);
            }
        }
        impl ReprHash for isize {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for isize {
            fn max_size_of() -> usize {
                size_of::<isize>()
            }
        }
        impl CopyType for i8 {
            type Copy = Zero;
        }
        impl TypeHash for i8 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "i8".hash(hasher);
            }
        }
        impl ReprHash for i8 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for i8 {
            fn max_size_of() -> usize {
                size_of::<i8>()
            }
        }
        impl CopyType for i16 {
            type Copy = Zero;
        }
        impl TypeHash for i16 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "i16".hash(hasher);
            }
        }
        impl ReprHash for i16 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for i16 {
            fn max_size_of() -> usize {
                size_of::<i16>()
            }
        }
        impl CopyType for i32 {
            type Copy = Zero;
        }
        impl TypeHash for i32 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "i32".hash(hasher);
            }
        }
        impl ReprHash for i32 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for i32 {
            fn max_size_of() -> usize {
                size_of::<i32>()
            }
        }
        impl CopyType for i64 {
            type Copy = Zero;
        }
        impl TypeHash for i64 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "i64".hash(hasher);
            }
        }
        impl ReprHash for i64 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for i64 {
            fn max_size_of() -> usize {
                size_of::<i64>()
            }
        }
        impl CopyType for i128 {
            type Copy = Zero;
        }
        impl TypeHash for i128 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "i128".hash(hasher);
            }
        }
        impl ReprHash for i128 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for i128 {
            fn max_size_of() -> usize {
                size_of::<i128>()
            }
        }
        impl CopyType for usize {
            type Copy = Zero;
        }
        impl TypeHash for usize {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "usize".hash(hasher);
            }
        }
        impl ReprHash for usize {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for usize {
            fn max_size_of() -> usize {
                size_of::<usize>()
            }
        }
        impl CopyType for u8 {
            type Copy = Zero;
        }
        impl TypeHash for u8 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "u8".hash(hasher);
            }
        }
        impl ReprHash for u8 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for u8 {
            fn max_size_of() -> usize {
                size_of::<u8>()
            }
        }
        impl CopyType for u16 {
            type Copy = Zero;
        }
        impl TypeHash for u16 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "u16".hash(hasher);
            }
        }
        impl ReprHash for u16 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for u16 {
            fn max_size_of() -> usize {
                size_of::<u16>()
            }
        }
        impl CopyType for u32 {
            type Copy = Zero;
        }
        impl TypeHash for u32 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "u32".hash(hasher);
            }
        }
        impl ReprHash for u32 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for u32 {
            fn max_size_of() -> usize {
                size_of::<u32>()
            }
        }
        impl CopyType for u64 {
            type Copy = Zero;
        }
        impl TypeHash for u64 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "u64".hash(hasher);
            }
        }
        impl ReprHash for u64 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for u64 {
            fn max_size_of() -> usize {
                size_of::<u64>()
            }
        }
        impl CopyType for u128 {
            type Copy = Zero;
        }
        impl TypeHash for u128 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "u128".hash(hasher);
            }
        }
        impl ReprHash for u128 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for u128 {
            fn max_size_of() -> usize {
                size_of::<u128>()
            }
        }
        impl CopyType for f32 {
            type Copy = Zero;
        }
        impl TypeHash for f32 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "f32".hash(hasher);
            }
        }
        impl ReprHash for f32 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for f32 {
            fn max_size_of() -> usize {
                size_of::<f32>()
            }
        }
        impl CopyType for f64 {
            type Copy = Zero;
        }
        impl TypeHash for f64 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "f64".hash(hasher);
            }
        }
        impl ReprHash for f64 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for f64 {
            fn max_size_of() -> usize {
                size_of::<f64>()
            }
        }
        impl SerializeInner for isize {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for isize {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<isize> {
                let mut buf = [0; size_of::<isize>()];
                backend.read_exact(&mut buf)?;
                Ok(<isize>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <isize>::from_ne_bytes(
                    backend.data[..size_of::<isize>()].try_into().unwrap(),
                );
                backend.skip(size_of::<isize>());
                Ok(res)
            }
        }
        impl SerializeInner for i8 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for i8 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<i8> {
                let mut buf = [0; size_of::<i8>()];
                backend.read_exact(&mut buf)?;
                Ok(<i8>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <i8>::from_ne_bytes(
                    backend.data[..size_of::<i8>()].try_into().unwrap(),
                );
                backend.skip(size_of::<i8>());
                Ok(res)
            }
        }
        impl SerializeInner for i16 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for i16 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<i16> {
                let mut buf = [0; size_of::<i16>()];
                backend.read_exact(&mut buf)?;
                Ok(<i16>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <i16>::from_ne_bytes(
                    backend.data[..size_of::<i16>()].try_into().unwrap(),
                );
                backend.skip(size_of::<i16>());
                Ok(res)
            }
        }
        impl SerializeInner for i32 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for i32 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<i32> {
                let mut buf = [0; size_of::<i32>()];
                backend.read_exact(&mut buf)?;
                Ok(<i32>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <i32>::from_ne_bytes(
                    backend.data[..size_of::<i32>()].try_into().unwrap(),
                );
                backend.skip(size_of::<i32>());
                Ok(res)
            }
        }
        impl SerializeInner for i64 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for i64 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<i64> {
                let mut buf = [0; size_of::<i64>()];
                backend.read_exact(&mut buf)?;
                Ok(<i64>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <i64>::from_ne_bytes(
                    backend.data[..size_of::<i64>()].try_into().unwrap(),
                );
                backend.skip(size_of::<i64>());
                Ok(res)
            }
        }
        impl SerializeInner for i128 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for i128 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<i128> {
                let mut buf = [0; size_of::<i128>()];
                backend.read_exact(&mut buf)?;
                Ok(<i128>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <i128>::from_ne_bytes(
                    backend.data[..size_of::<i128>()].try_into().unwrap(),
                );
                backend.skip(size_of::<i128>());
                Ok(res)
            }
        }
        impl SerializeInner for usize {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for usize {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<usize> {
                let mut buf = [0; size_of::<usize>()];
                backend.read_exact(&mut buf)?;
                Ok(<usize>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <usize>::from_ne_bytes(
                    backend.data[..size_of::<usize>()].try_into().unwrap(),
                );
                backend.skip(size_of::<usize>());
                Ok(res)
            }
        }
        impl SerializeInner for u8 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for u8 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<u8> {
                let mut buf = [0; size_of::<u8>()];
                backend.read_exact(&mut buf)?;
                Ok(<u8>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <u8>::from_ne_bytes(
                    backend.data[..size_of::<u8>()].try_into().unwrap(),
                );
                backend.skip(size_of::<u8>());
                Ok(res)
            }
        }
        impl SerializeInner for u16 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for u16 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<u16> {
                let mut buf = [0; size_of::<u16>()];
                backend.read_exact(&mut buf)?;
                Ok(<u16>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <u16>::from_ne_bytes(
                    backend.data[..size_of::<u16>()].try_into().unwrap(),
                );
                backend.skip(size_of::<u16>());
                Ok(res)
            }
        }
        impl SerializeInner for u32 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for u32 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<u32> {
                let mut buf = [0; size_of::<u32>()];
                backend.read_exact(&mut buf)?;
                Ok(<u32>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <u32>::from_ne_bytes(
                    backend.data[..size_of::<u32>()].try_into().unwrap(),
                );
                backend.skip(size_of::<u32>());
                Ok(res)
            }
        }
        impl SerializeInner for u64 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for u64 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<u64> {
                let mut buf = [0; size_of::<u64>()];
                backend.read_exact(&mut buf)?;
                Ok(<u64>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <u64>::from_ne_bytes(
                    backend.data[..size_of::<u64>()].try_into().unwrap(),
                );
                backend.skip(size_of::<u64>());
                Ok(res)
            }
        }
        impl SerializeInner for u128 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for u128 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<u128> {
                let mut buf = [0; size_of::<u128>()];
                backend.read_exact(&mut buf)?;
                Ok(<u128>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <u128>::from_ne_bytes(
                    backend.data[..size_of::<u128>()].try_into().unwrap(),
                );
                backend.skip(size_of::<u128>());
                Ok(res)
            }
        }
        impl SerializeInner for f32 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for f32 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<f32> {
                let mut buf = [0; size_of::<f32>()];
                backend.read_exact(&mut buf)?;
                Ok(<f32>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <f32>::from_ne_bytes(
                    backend.data[..size_of::<f32>()].try_into().unwrap(),
                );
                backend.skip(size_of::<f32>());
                Ok(res)
            }
        }
        impl SerializeInner for f64 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.to_ne_bytes())
            }
        }
        impl DeserializeInner for f64 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<f64> {
                let mut buf = [0; size_of::<f64>()];
                backend.read_exact(&mut buf)?;
                Ok(<f64>::from_ne_bytes(buf))
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <f64>::from_ne_bytes(
                    backend.data[..size_of::<f64>()].try_into().unwrap(),
                );
                backend.skip(size_of::<f64>());
                Ok(res)
            }
        }
        impl CopyType for NonZeroIsize {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroIsize {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroIsize".hash(hasher);
            }
        }
        impl ReprHash for NonZeroIsize {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroIsize {
            fn max_size_of() -> usize {
                size_of::<NonZeroIsize>()
            }
        }
        impl CopyType for NonZeroI8 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroI8 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroI8".hash(hasher);
            }
        }
        impl ReprHash for NonZeroI8 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroI8 {
            fn max_size_of() -> usize {
                size_of::<NonZeroI8>()
            }
        }
        impl CopyType for NonZeroI16 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroI16 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroI16".hash(hasher);
            }
        }
        impl ReprHash for NonZeroI16 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroI16 {
            fn max_size_of() -> usize {
                size_of::<NonZeroI16>()
            }
        }
        impl CopyType for NonZeroI32 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroI32 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroI32".hash(hasher);
            }
        }
        impl ReprHash for NonZeroI32 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroI32 {
            fn max_size_of() -> usize {
                size_of::<NonZeroI32>()
            }
        }
        impl CopyType for NonZeroI64 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroI64 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroI64".hash(hasher);
            }
        }
        impl ReprHash for NonZeroI64 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroI64 {
            fn max_size_of() -> usize {
                size_of::<NonZeroI64>()
            }
        }
        impl CopyType for NonZeroI128 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroI128 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroI128".hash(hasher);
            }
        }
        impl ReprHash for NonZeroI128 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroI128 {
            fn max_size_of() -> usize {
                size_of::<NonZeroI128>()
            }
        }
        impl CopyType for NonZeroUsize {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroUsize {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroUsize".hash(hasher);
            }
        }
        impl ReprHash for NonZeroUsize {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroUsize {
            fn max_size_of() -> usize {
                size_of::<NonZeroUsize>()
            }
        }
        impl CopyType for NonZeroU8 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroU8 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroU8".hash(hasher);
            }
        }
        impl ReprHash for NonZeroU8 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroU8 {
            fn max_size_of() -> usize {
                size_of::<NonZeroU8>()
            }
        }
        impl CopyType for NonZeroU16 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroU16 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroU16".hash(hasher);
            }
        }
        impl ReprHash for NonZeroU16 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroU16 {
            fn max_size_of() -> usize {
                size_of::<NonZeroU16>()
            }
        }
        impl CopyType for NonZeroU32 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroU32 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroU32".hash(hasher);
            }
        }
        impl ReprHash for NonZeroU32 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroU32 {
            fn max_size_of() -> usize {
                size_of::<NonZeroU32>()
            }
        }
        impl CopyType for NonZeroU64 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroU64 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroU64".hash(hasher);
            }
        }
        impl ReprHash for NonZeroU64 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroU64 {
            fn max_size_of() -> usize {
                size_of::<NonZeroU64>()
            }
        }
        impl CopyType for NonZeroU128 {
            type Copy = Zero;
        }
        impl TypeHash for NonZeroU128 {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "NonZeroU128".hash(hasher);
            }
        }
        impl ReprHash for NonZeroU128 {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for NonZeroU128 {
            fn max_size_of() -> usize {
                size_of::<NonZeroU128>()
            }
        }
        impl SerializeInner for NonZeroIsize {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroIsize {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroIsize> {
                let mut buf = [0; size_of::<NonZeroIsize>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroIsize as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroIsize as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroIsize>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroIsize>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroI8 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroI8 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroI8> {
                let mut buf = [0; size_of::<NonZeroI8>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroI8 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroI8 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroI8>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroI8>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroI16 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroI16 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroI16> {
                let mut buf = [0; size_of::<NonZeroI16>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroI16 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroI16 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroI16>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroI16>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroI32 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroI32 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroI32> {
                let mut buf = [0; size_of::<NonZeroI32>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroI32 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroI32 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroI32>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroI32>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroI64 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroI64 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroI64> {
                let mut buf = [0; size_of::<NonZeroI64>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroI64 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroI64 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroI64>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroI64>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroI128 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroI128 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroI128> {
                let mut buf = [0; size_of::<NonZeroI128>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroI128 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroI128 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroI128>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroI128>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroUsize {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroUsize {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroUsize> {
                let mut buf = [0; size_of::<NonZeroUsize>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroUsize as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroUsize as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroUsize>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroUsize>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroU8 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroU8 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroU8> {
                let mut buf = [0; size_of::<NonZeroU8>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroU8 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroU8 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroU8>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroU8>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroU16 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroU16 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroU16> {
                let mut buf = [0; size_of::<NonZeroU16>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroU16 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroU16 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroU16>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroU16>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroU32 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroU32 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroU32> {
                let mut buf = [0; size_of::<NonZeroU32>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroU32 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroU32 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroU32>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroU32>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroU64 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroU64 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroU64> {
                let mut buf = [0; size_of::<NonZeroU64>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroU64 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroU64 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroU64>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroU64>());
                Ok(res)
            }
        }
        impl SerializeInner for NonZeroU128 {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                backend.write_all(&self.get().to_ne_bytes())
            }
        }
        impl DeserializeInner for NonZeroU128 {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<NonZeroU128> {
                let mut buf = [0; size_of::<NonZeroU128>()];
                backend.read_exact(&mut buf)?;
                Ok(
                    <NonZeroU128 as NonZero>::BaseType::from_ne_bytes(buf)
                        .try_into()
                        .unwrap(),
                )
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = <NonZeroU128 as NonZero>::BaseType::from_ne_bytes(
                        backend.data[..size_of::<NonZeroU128>()].try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();
                backend.skip(size_of::<NonZeroU128>());
                Ok(res)
            }
        }
        impl CopyType for bool {
            type Copy = Zero;
        }
        impl TypeHash for bool {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "bool".hash(hasher);
            }
        }
        impl ReprHash for bool {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for bool {
            fn max_size_of() -> usize {
                size_of::<bool>()
            }
        }
        impl CopyType for char {
            type Copy = Zero;
        }
        impl TypeHash for char {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "char".hash(hasher);
            }
        }
        impl ReprHash for char {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for char {
            fn max_size_of() -> usize {
                size_of::<char>()
            }
        }
        impl CopyType for () {
            type Copy = Zero;
        }
        impl TypeHash for () {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
            }
        }
        impl ReprHash for () {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for () {
            fn max_size_of() -> usize {
                size_of::<()>()
            }
        }
        impl SerializeInner for bool {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                let val = if *self { 1 } else { 0 };
                backend.write_all(&[val])
            }
        }
        impl DeserializeInner for bool {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<bool> {
                Ok(u8::_deserialize_full_inner(backend)? != 0)
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let res = backend.data[0] != 0;
                backend.skip(1);
                Ok(res)
            }
        }
        impl SerializeInner for char {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                (*self as u32)._serialize_inner(backend)
            }
        }
        impl DeserializeInner for char {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                Ok(char::from_u32(u32::_deserialize_full_inner(backend)?).unwrap())
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                Ok(char::from_u32(u32::_deserialize_eps_inner(backend)?).unwrap())
            }
        }
        impl SerializeInner for () {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                _backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                Ok(())
            }
        }
        impl DeserializeInner for () {
            #[inline(always)]
            fn _deserialize_full_inner(
                _backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                Ok(())
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                _backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                Ok(())
            }
        }
        impl<T: ?Sized> CopyType for PhantomData<T> {
            type Copy = Zero;
        }
        impl<T: ?Sized + TypeHash> TypeHash for PhantomData<T> {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "PhantomData".hash(hasher);
                T::type_hash(hasher);
            }
        }
        impl<T: ?Sized> ReprHash for PhantomData<T> {
            #[inline(always)]
            fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
        }
        impl<T: ?Sized + TypeHash> SerializeInner for PhantomData<T> {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                _backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                Ok(())
            }
        }
        impl<T: ?Sized + TypeHash> DeserializeInner for PhantomData<T> {
            #[inline(always)]
            fn _deserialize_full_inner(
                _backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                Ok(PhantomData::<T>)
            }
            type DeserType<'a> = Self;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                _backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                Ok(PhantomData)
            }
        }
        impl<T> CopyType for Option<T> {
            type Copy = Deep;
        }
        impl<T: TypeHash> TypeHash for Option<T> {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "Option".hash(hasher);
                T::type_hash(hasher);
            }
        }
        impl<T: ReprHash> ReprHash for Option<T> {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                *offset_of = 0;
                T::repr_hash(hasher, offset_of);
            }
        }
        impl<T: SerializeInner> SerializeInner for Option<T> {
            const IS_ZERO_COPY: bool = false;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                match self {
                    None => backend.write("Tag", &0_u8),
                    Some(val) => {
                        backend.write("Tag", &1_u8)?;
                        backend.write("Some", val)
                    }
                }
            }
        }
        impl<T: DeserializeInner> DeserializeInner for Option<T> {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                let tag = u8::_deserialize_full_inner(backend)?;
                match tag {
                    0 => Ok(None),
                    1 => Ok(Some(T::_deserialize_full_inner(backend)?)),
                    _ => Err(deser::Error::InvalidTag(tag as usize)),
                }
            }
            type DeserType<'a> = Option<<T as DeserializeInner>::DeserType<'a>>;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let tag = u8::_deserialize_full_inner(backend)?;
                match tag {
                    0 => Ok(None),
                    1 => Ok(Some(T::_deserialize_eps_inner(backend)?)),
                    _ => Err(deser::Error::InvalidTag(backend.data[0] as usize)),
                }
            }
        }
    }
    pub mod slice {
        /*!

Implementations for slices.

Slices cannot be serialized in isolation, but they must implement [`TypeHash`] and
[`ReprHash`] so that they can be used with [`PhantomData`](std::marker::PhantomData).

We also provide a serialize-only (slightly cheaty) implementation
for slices that deserializes to vectors.

It is slightly cheaty in that it serializes a vector using the
slice as a backing array, so it must be deserialized using a vector as type.

Note that if you ε-copy deserialize the vector, you will
get back the same slice.
```rust
use epserde::prelude::*;
use maligned::A16;
let a = vec![1, 2, 3, 4];
let s = a.as_slice();
let mut cursor = <AlignedCursor<A16>>::new();
s.serialize(&mut cursor).unwrap();
cursor.set_position(0);
let b: Vec<i32> = <Vec<i32>>::deserialize_full(&mut cursor).unwrap();
assert_eq!(a, b);
let b: &[i32] = <Vec<i32>>::deserialize_eps(cursor.as_bytes()).unwrap();
assert_eq!(a, *b);
```

*/
        use crate::prelude::*;
        use ser::*;
        use std::hash::Hash;
        impl<T: TypeHash> TypeHash for [T] {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "[]".hash(hasher);
                T::type_hash(hasher);
            }
        }
        impl<T> ReprHash for [T] {
            #[inline(always)]
            fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
        }
        impl<T: SerializeInner + CopyType + TypeHash + ReprHash> Serialize for [T]
        where
            Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
        {
            fn serialize_on_field_write(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                write_header::<Vec<T>>(backend)?;
                let fake = unsafe {
                    Vec::from_raw_parts(self.as_ptr() as *mut T, self.len(), self.len())
                };
                backend.write("ROOT", &fake)?;
                core::mem::forget(fake);
                backend.flush()
            }
        }
    }
    #[cfg(feature = "std")]
    pub mod stdlib {
        //! Implementation of traits for struts from the std library
        use crate::prelude::*;
        use core::hash::Hash;
        use std::collections::hash_map::DefaultHasher;
        impl TypeHash for DefaultHasher {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "std::hash::DefaultHasher".hash(hasher);
            }
        }
        impl ReprHash for DefaultHasher {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                crate::traits::std_repr_hash::<Self>(hasher, offset_of)
            }
        }
        impl MaxSizeOf for DefaultHasher {
            fn max_size_of() -> usize {
                core::mem::size_of::<Self>()
            }
        }
    }
    pub mod string {
        /*!

Implementations for strings.

*/
        use crate::prelude::*;
        use core::hash::Hash;
        use deser::*;
        use ser::*;
        impl CopyType for String {
            type Copy = Deep;
        }
        #[cfg(feature = "alloc")]
        impl TypeHash for String {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "String".hash(hasher);
            }
        }
        impl ReprHash for String {
            fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
        }
        impl TypeHash for Box<str> {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "Box<str>".hash(hasher);
            }
        }
        impl ReprHash for Box<str> {
            fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
        }
        impl TypeHash for str {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "str".hash(hasher);
            }
        }
        impl ReprHash for str {
            fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {}
        }
        impl SerializeInner for String {
            const IS_ZERO_COPY: bool = false;
            const ZERO_COPY_MISMATCH: bool = false;
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_slice_zero(backend, self.as_bytes())
            }
        }
        impl DeserializeInner for String {
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                let slice = deserialize_full_vec_zero(backend)?;
                Ok(String::from_utf8(slice).unwrap())
            }
            type DeserType<'a> = &'a str;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                let slice = deserialize_eps_slice_zero(backend)?;
                Ok(unsafe {
                    #[allow(clippy::transmute_bytes_to_str)]
                    core::mem::transmute::<&'_ [u8], &'_ str>(slice)
                })
            }
        }
        impl CopyType for Box<str> {
            type Copy = Deep;
        }
        impl SerializeInner for Box<str> {
            const IS_ZERO_COPY: bool = false;
            const ZERO_COPY_MISMATCH: bool = false;
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_slice_zero(backend, self.as_bytes())
            }
        }
        impl DeserializeInner for Box<str> {
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                Ok(String::_deserialize_full_inner(backend)?.into_boxed_str())
            }
            type DeserType<'a> = &'a str;
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                String::_deserialize_eps_inner(backend)
            }
        }
    }
    pub mod tuple {
        /*!

Implementations for tuples.

For the time being, we only support tuples of up to 10 elements all of which
are [`ZeroCopy`] and parameterless. For tuples of more than 10 elements, tuples with elements
that are not [`ZeroCopy`], or types with parameters, you must use [`epserde_derive::Epserde`] on a newtype.

*/
        use crate::prelude::*;
        use core::hash::Hash;
        use deser::*;
        use ser::*;
        impl<
            T0: ZeroCopy,
            T1: ZeroCopy,
            T2: ZeroCopy,
            T3: ZeroCopy,
            T4: ZeroCopy,
            T5: ZeroCopy,
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T0: TypeHash,
            T1: TypeHash,
            T2: TypeHash,
            T3: TypeHash,
            T4: TypeHash,
            T5: TypeHash,
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T0>::type_hash(hasher);
                <T1>::type_hash(hasher);
                <T2>::type_hash(hasher);
                <T3>::type_hash(hasher);
                <T4>::type_hash(hasher);
                <T5>::type_hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T0: ReprHash,
            T1: ReprHash,
            T2: ReprHash,
            T3: ReprHash,
            T4: ReprHash,
            T5: ReprHash,
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T0>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T0>();
                let curr_offset_of = *offset_of;
                <T1>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T1>();
                let curr_offset_of = *offset_of;
                <T2>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T2>();
                let curr_offset_of = *offset_of;
                <T3>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T3>();
                let curr_offset_of = *offset_of;
                <T4>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T4>();
                let curr_offset_of = *offset_of;
                <T5>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T5>();
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T0: MaxSizeOf,
            T1: MaxSizeOf,
            T2: MaxSizeOf,
            T3: MaxSizeOf,
            T4: MaxSizeOf,
            T5: MaxSizeOf,
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T0>::max_size_of()) {
                    max_size_of = <T0>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T1>::max_size_of()) {
                    max_size_of = <T1>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T2>::max_size_of()) {
                    max_size_of = <T2>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T3>::max_size_of()) {
                    max_size_of = <T3>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T4>::max_size_of()) {
                    max_size_of = <T4>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T5>::max_size_of()) {
                    max_size_of = <T5>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T0: ZeroCopy + TypeHash + ReprHash,
            T1: ZeroCopy + TypeHash + ReprHash,
            T2: ZeroCopy + TypeHash + ReprHash,
            T3: ZeroCopy + TypeHash + ReprHash,
            T4: ZeroCopy + TypeHash + ReprHash,
            T5: ZeroCopy + TypeHash + ReprHash,
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T0: ZeroCopy + TypeHash + ReprHash + 'static,
            T1: ZeroCopy + TypeHash + ReprHash + 'static,
            T2: ZeroCopy + TypeHash + ReprHash + 'static,
            T3: ZeroCopy + TypeHash + ReprHash + 'static,
            T4: ZeroCopy + TypeHash + ReprHash + 'static,
            T5: ZeroCopy + TypeHash + ReprHash + 'static,
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<
                    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
                >(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<
                    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
                >(backend)
            }
        }
        impl<
            T1: ZeroCopy,
            T2: ZeroCopy,
            T3: ZeroCopy,
            T4: ZeroCopy,
            T5: ZeroCopy,
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T1: TypeHash,
            T2: TypeHash,
            T3: TypeHash,
            T4: TypeHash,
            T5: TypeHash,
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T1>::type_hash(hasher);
                <T2>::type_hash(hasher);
                <T3>::type_hash(hasher);
                <T4>::type_hash(hasher);
                <T5>::type_hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T1: ReprHash,
            T2: ReprHash,
            T3: ReprHash,
            T4: ReprHash,
            T5: ReprHash,
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T1>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T1>();
                let curr_offset_of = *offset_of;
                <T2>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T2>();
                let curr_offset_of = *offset_of;
                <T3>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T3>();
                let curr_offset_of = *offset_of;
                <T4>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T4>();
                let curr_offset_of = *offset_of;
                <T5>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T5>();
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T1: MaxSizeOf,
            T2: MaxSizeOf,
            T3: MaxSizeOf,
            T4: MaxSizeOf,
            T5: MaxSizeOf,
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T1>::max_size_of()) {
                    max_size_of = <T1>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T2>::max_size_of()) {
                    max_size_of = <T2>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T3>::max_size_of()) {
                    max_size_of = <T3>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T4>::max_size_of()) {
                    max_size_of = <T4>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T5>::max_size_of()) {
                    max_size_of = <T5>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T1: ZeroCopy + TypeHash + ReprHash,
            T2: ZeroCopy + TypeHash + ReprHash,
            T3: ZeroCopy + TypeHash + ReprHash,
            T4: ZeroCopy + TypeHash + ReprHash,
            T5: ZeroCopy + TypeHash + ReprHash,
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T1: ZeroCopy + TypeHash + ReprHash + 'static,
            T2: ZeroCopy + TypeHash + ReprHash + 'static,
            T3: ZeroCopy + TypeHash + ReprHash + 'static,
            T4: ZeroCopy + TypeHash + ReprHash + 'static,
            T5: ZeroCopy + TypeHash + ReprHash + 'static,
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<
                    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
                >(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<
                    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
                >(backend)
            }
        }
        impl<
            T2: ZeroCopy,
            T3: ZeroCopy,
            T4: ZeroCopy,
            T5: ZeroCopy,
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T2: TypeHash,
            T3: TypeHash,
            T4: TypeHash,
            T5: TypeHash,
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T2>::type_hash(hasher);
                <T3>::type_hash(hasher);
                <T4>::type_hash(hasher);
                <T5>::type_hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T2: ReprHash,
            T3: ReprHash,
            T4: ReprHash,
            T5: ReprHash,
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T2>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T2>();
                let curr_offset_of = *offset_of;
                <T3>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T3>();
                let curr_offset_of = *offset_of;
                <T4>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T4>();
                let curr_offset_of = *offset_of;
                <T5>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T5>();
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T2: MaxSizeOf,
            T3: MaxSizeOf,
            T4: MaxSizeOf,
            T5: MaxSizeOf,
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T2>::max_size_of()) {
                    max_size_of = <T2>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T3>::max_size_of()) {
                    max_size_of = <T3>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T4>::max_size_of()) {
                    max_size_of = <T4>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T5>::max_size_of()) {
                    max_size_of = <T5>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T2: ZeroCopy + TypeHash + ReprHash,
            T3: ZeroCopy + TypeHash + ReprHash,
            T4: ZeroCopy + TypeHash + ReprHash,
            T5: ZeroCopy + TypeHash + ReprHash,
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T2: ZeroCopy + TypeHash + ReprHash + 'static,
            T3: ZeroCopy + TypeHash + ReprHash + 'static,
            T4: ZeroCopy + TypeHash + ReprHash + 'static,
            T5: ZeroCopy + TypeHash + ReprHash + 'static,
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T2, T3, T4, T5, T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T2, T3, T4, T5, T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T2, T3, T4, T5, T6, T7, T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T2, T3, T4, T5, T6, T7, T8, T9, T10)>(backend)
            }
        }
        impl<
            T3: ZeroCopy,
            T4: ZeroCopy,
            T5: ZeroCopy,
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T3, T4, T5, T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T3: TypeHash,
            T4: TypeHash,
            T5: TypeHash,
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T3>::type_hash(hasher);
                <T4>::type_hash(hasher);
                <T5>::type_hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T3: ReprHash,
            T4: ReprHash,
            T5: ReprHash,
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T3>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T3>();
                let curr_offset_of = *offset_of;
                <T4>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T4>();
                let curr_offset_of = *offset_of;
                <T5>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T5>();
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T3: MaxSizeOf,
            T4: MaxSizeOf,
            T5: MaxSizeOf,
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T3, T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T3>::max_size_of()) {
                    max_size_of = <T3>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T4>::max_size_of()) {
                    max_size_of = <T4>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T5>::max_size_of()) {
                    max_size_of = <T5>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T3: ZeroCopy + TypeHash + ReprHash,
            T4: ZeroCopy + TypeHash + ReprHash,
            T5: ZeroCopy + TypeHash + ReprHash,
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T3, T4, T5, T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T3: ZeroCopy + TypeHash + ReprHash + 'static,
            T4: ZeroCopy + TypeHash + ReprHash + 'static,
            T5: ZeroCopy + TypeHash + ReprHash + 'static,
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T3, T4, T5, T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T3, T4, T5, T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T3, T4, T5, T6, T7, T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T3, T4, T5, T6, T7, T8, T9, T10)>(backend)
            }
        }
        impl<
            T4: ZeroCopy,
            T5: ZeroCopy,
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T4, T5, T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T4: TypeHash,
            T5: TypeHash,
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T4>::type_hash(hasher);
                <T5>::type_hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T4: ReprHash,
            T5: ReprHash,
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T4>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T4>();
                let curr_offset_of = *offset_of;
                <T5>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T5>();
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T4: MaxSizeOf,
            T5: MaxSizeOf,
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T4, T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T4>::max_size_of()) {
                    max_size_of = <T4>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T5>::max_size_of()) {
                    max_size_of = <T5>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T4: ZeroCopy + TypeHash + ReprHash,
            T5: ZeroCopy + TypeHash + ReprHash,
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T4, T5, T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T4: ZeroCopy + TypeHash + ReprHash + 'static,
            T5: ZeroCopy + TypeHash + ReprHash + 'static,
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T4, T5, T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T4, T5, T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T4, T5, T6, T7, T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T4, T5, T6, T7, T8, T9, T10)>(backend)
            }
        }
        impl<
            T5: ZeroCopy,
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T5, T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T5: TypeHash,
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T5>::type_hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T5: ReprHash,
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T5>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T5>();
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T5: MaxSizeOf,
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T5, T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T5>::max_size_of()) {
                    max_size_of = <T5>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T5: ZeroCopy + TypeHash + ReprHash,
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T5, T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T5: ZeroCopy + TypeHash + ReprHash + 'static,
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T5, T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T5, T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T5, T6, T7, T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T5, T6, T7, T8, T9, T10)>(backend)
            }
        }
        impl<
            T6: ZeroCopy,
            T7: ZeroCopy,
            T8: ZeroCopy,
            T9: ZeroCopy,
            T10: ZeroCopy,
        > CopyType for (T6, T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<
            T6: TypeHash,
            T7: TypeHash,
            T8: TypeHash,
            T9: TypeHash,
            T10: TypeHash,
        > TypeHash for (T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T6>::type_hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<
            T6: ReprHash,
            T7: ReprHash,
            T8: ReprHash,
            T9: ReprHash,
            T10: ReprHash,
        > ReprHash for (T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T6>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T6>();
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<
            T6: MaxSizeOf,
            T7: MaxSizeOf,
            T8: MaxSizeOf,
            T9: MaxSizeOf,
            T10: MaxSizeOf,
        > MaxSizeOf for (T6, T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T6>::max_size_of()) {
                    max_size_of = <T6>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T6: ZeroCopy + TypeHash + ReprHash,
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T6, T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T6: ZeroCopy + TypeHash + ReprHash + 'static,
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T6, T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T6, T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T6, T7, T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T6, T7, T8, T9, T10)>(backend)
            }
        }
        impl<T7: ZeroCopy, T8: ZeroCopy, T9: ZeroCopy, T10: ZeroCopy> CopyType
        for (T7, T8, T9, T10) {
            type Copy = Zero;
        }
        impl<T7: TypeHash, T8: TypeHash, T9: TypeHash, T10: TypeHash> TypeHash
        for (T7, T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T7>::type_hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<T7: ReprHash, T8: ReprHash, T9: ReprHash, T10: ReprHash> ReprHash
        for (T7, T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T7>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T7>();
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<T7: MaxSizeOf, T8: MaxSizeOf, T9: MaxSizeOf, T10: MaxSizeOf> MaxSizeOf
        for (T7, T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T7>::max_size_of()) {
                    max_size_of = <T7>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T7: ZeroCopy + TypeHash + ReprHash,
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T7, T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T7: ZeroCopy + TypeHash + ReprHash + 'static,
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T7, T8, T9, T10) {
            type DeserType<'a> = &'a (T7, T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T7, T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T7, T8, T9, T10)>(backend)
            }
        }
        impl<T8: ZeroCopy, T9: ZeroCopy, T10: ZeroCopy> CopyType for (T8, T9, T10) {
            type Copy = Zero;
        }
        impl<T8: TypeHash, T9: TypeHash, T10: TypeHash> TypeHash for (T8, T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T8>::type_hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<T8: ReprHash, T9: ReprHash, T10: ReprHash> ReprHash for (T8, T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T8>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T8>();
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<T8: MaxSizeOf, T9: MaxSizeOf, T10: MaxSizeOf> MaxSizeOf for (T8, T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T8>::max_size_of()) {
                    max_size_of = <T8>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T8: ZeroCopy + TypeHash + ReprHash,
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T8, T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T8: ZeroCopy + TypeHash + ReprHash + 'static,
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T8, T9, T10) {
            type DeserType<'a> = &'a (T8, T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T8, T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T8, T9, T10)>(backend)
            }
        }
        impl<T9: ZeroCopy, T10: ZeroCopy> CopyType for (T9, T10) {
            type Copy = Zero;
        }
        impl<T9: TypeHash, T10: TypeHash> TypeHash for (T9, T10) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T9>::type_hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<T9: ReprHash, T10: ReprHash> ReprHash for (T9, T10) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T9>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T9>();
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<T9: MaxSizeOf, T10: MaxSizeOf> MaxSizeOf for (T9, T10) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T9>::max_size_of()) {
                    max_size_of = <T9>::max_size_of();
                }
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<
            T9: ZeroCopy + TypeHash + ReprHash,
            T10: ZeroCopy + TypeHash + ReprHash,
        > SerializeInner for (T9, T10) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<
            T9: ZeroCopy + TypeHash + ReprHash + 'static,
            T10: ZeroCopy + TypeHash + ReprHash + 'static,
        > DeserializeInner for (T9, T10) {
            type DeserType<'a> = &'a (T9, T10);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T9, T10)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T9, T10)>(backend)
            }
        }
        impl<T10: ZeroCopy> CopyType for (T10,) {
            type Copy = Zero;
        }
        impl<T10: TypeHash> TypeHash for (T10,) {
            #[inline(always)]
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "()".hash(hasher);
                <T10>::type_hash(hasher);
            }
        }
        impl<T10: ReprHash> ReprHash for (T10,) {
            #[inline(always)]
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                let curr_offset_of = *offset_of;
                <T10>::repr_hash(hasher, offset_of);
                *offset_of = curr_offset_of + core::mem::size_of::<T10>();
            }
        }
        impl<T10: MaxSizeOf> MaxSizeOf for (T10,) {
            #[inline(always)]
            fn max_size_of() -> usize {
                let mut max_size_of = 0;
                if max_size_of < std::cmp::max(max_size_of, <T10>::max_size_of()) {
                    max_size_of = <T10>::max_size_of();
                }
                max_size_of
            }
        }
        impl<T10: ZeroCopy + TypeHash + ReprHash> SerializeInner for (T10,) {
            const IS_ZERO_COPY: bool = true;
            const ZERO_COPY_MISMATCH: bool = false;
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_zero(backend, self)
            }
        }
        impl<T10: ZeroCopy + TypeHash + ReprHash + 'static> DeserializeInner for (T10,) {
            type DeserType<'a> = &'a (T10,);
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_zero::<(T10,)>(backend)
            }
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<Self::DeserType<'a>> {
                deserialize_eps_zero::<(T10,)>(backend)
            }
        }
    }
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub mod vec {
        /*!

Implementations for vectors.

*/
        use crate::deser;
        use crate::deser::helpers::*;
        use crate::deser::*;
        use crate::ser;
        use crate::ser::helpers::*;
        use crate::ser::*;
        use crate::traits::*;
        use core::hash::Hash;
        impl<T> CopyType for Vec<T> {
            type Copy = Deep;
        }
        impl<T: TypeHash> TypeHash for Vec<T> {
            fn type_hash(hasher: &mut impl core::hash::Hasher) {
                "Vec".hash(hasher);
                T::type_hash(hasher);
            }
        }
        impl<T: ReprHash> ReprHash for Vec<T> {
            fn repr_hash(hasher: &mut impl core::hash::Hasher, offset_of: &mut usize) {
                *offset_of = 0;
                T::repr_hash(hasher, offset_of);
            }
        }
        impl<T: CopyType + SerializeInner + TypeHash> SerializeInner for Vec<T>
        where
            Vec<T>: SerializeHelper<<T as CopyType>::Copy>,
        {
            const IS_ZERO_COPY: bool = false;
            const ZERO_COPY_MISMATCH: bool = false;
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                SerializeHelper::_serialize_inner(self, backend)
            }
        }
        impl<T: ZeroCopy + SerializeInner> SerializeHelper<Zero> for Vec<T> {
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_slice_zero(backend, self.as_slice())
            }
        }
        impl<T: DeepCopy + SerializeInner> SerializeHelper<Deep> for Vec<T> {
            #[inline(always)]
            fn _serialize_inner(
                &self,
                backend: &mut impl WriteWithNames,
            ) -> ser::Result<()> {
                serialize_slice_deep(backend, self.as_slice())
            }
        }
        impl<T: CopyType + DeserializeInner + 'static> DeserializeInner for Vec<T>
        where
            Vec<T>: DeserializeHelper<<T as CopyType>::Copy, FullType = Vec<T>>,
        {
            type DeserType<'a> = <Vec<
                T,
            > as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>;
            #[inline(always)]
            fn _deserialize_full_inner(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                <Vec<
                    T,
                > as DeserializeHelper<
                    <T as CopyType>::Copy,
                >>::_deserialize_full_inner_impl(backend)
            }
            #[inline(always)]
            fn _deserialize_eps_inner<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<
                <Vec<T> as DeserializeHelper<<T as CopyType>::Copy>>::DeserType<'a>,
            > {
                <Vec<
                    T,
                > as DeserializeHelper<
                    <T as CopyType>::Copy,
                >>::_deserialize_eps_inner_impl(backend)
            }
        }
        impl<T: ZeroCopy + DeserializeInner + 'static> DeserializeHelper<Zero>
        for Vec<T> {
            type FullType = Self;
            type DeserType<'a> = &'a [T];
            #[inline(always)]
            fn _deserialize_full_inner_impl(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_vec_zero(backend)
            }
            #[inline(always)]
            fn _deserialize_eps_inner_impl<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
                deserialize_eps_slice_zero(backend)
            }
        }
        impl<T: DeepCopy + DeserializeInner + 'static> DeserializeHelper<Deep>
        for Vec<T> {
            type FullType = Self;
            type DeserType<'a> = Vec<<T as DeserializeInner>::DeserType<'a>>;
            #[inline(always)]
            fn _deserialize_full_inner_impl(
                backend: &mut impl ReadWithPos,
            ) -> deser::Result<Self> {
                deserialize_full_vec_deep::<T>(backend)
            }
            #[inline(always)]
            fn _deserialize_eps_inner_impl<'a>(
                backend: &mut SliceWithPos<'a>,
            ) -> deser::Result<<Self as DeserializeInner>::DeserType<'a>> {
                deserialize_eps_vec_deep::<T>(backend)
            }
        }
    }
}
pub mod ser {
    /*!

Serialization traits and types.

[`Serialize`] is the main serialization trait, providing a
[`Serialize::serialize`] method that serializes the type into a
generic [`WriteNoStd`] backend, and a [`Serialize::serialize_with_schema`] method
that additionally returns a [`Schema`] describing the data that has been written.
The implementation of this trait
is based on [`SerializeInner`], which is automatically derived
with `#[derive(Serialize)]`.

*/
    use crate::traits::*;
    use crate::*;
    use core::hash::Hasher;
    use std::{io::BufWriter, path::Path};
    pub mod write_with_names {
        /*!

Traits and implementations to write named field during serialization.

[`SerializeInner::_serialize_inner`] writes on a [`WriteWithNames`], rather
than on a [`WriteWithPos`], with the purpose of easily recording write
events happening during a serialization.

*/
        use super::*;
        use mem_dbg::{MemDbg, MemSize};
        /// Trait extending [`WriteWithPos`] with methods providing
        /// alignment, serialization of named data, and writing of byte slices
        /// of zero-copy types.
        ///
        /// The purpose of this trait is that of interposing between [`SerializeInner`]
        /// and the underlying [`WriteWithPos`] a layer in which serialization operations
        /// can be easily intercepted and recorded. In particular, serialization methods
        /// must use the methods of this trait if they want to record the schema of the
        /// serialized data; this is true (maybe counterintuitively) even of ancillary
        /// data such as tags and slice lengths: see [`helpers`] or the
        /// [implementation of `Option`](impls::prim) for examples.
        /// All methods have a default
        /// implementation that must be replicated in other implementations.
        ///
        /// There are two implementations of [`WriteWithNames`]: [`WriterWithPos`],
        /// which uses the default implementation, and [`SchemaWriter`],
        /// which additionally records a [`Schema`] of the serialized data.
        pub trait WriteWithNames: WriteWithPos + Sized {
            /// Add some zero padding so that `self.pos() % V:max_size_of() == 0.`
            ///
            /// Other implementations must write the same number of zeros.
            fn align<V: MaxSizeOf>(&mut self) -> Result<()> {
                let padding = pad_align_to(self.pos(), V::max_size_of());
                for _ in 0..padding {
                    self.write_all(&[0])?;
                }
                Ok(())
            }
            /// Write a value with an associated name.
            ///
            /// The default implementation simply delegates to [`SerializeInner::_serialize_inner`].
            /// Other implementations might use the name information (e.g., [`SchemaWriter`]),
            /// but they must in the end delegate to [`SerializeInner::_serialize_inner`].
            fn write<V: SerializeInner>(
                &mut self,
                _field_name: &str,
                value: &V,
            ) -> Result<()> {
                value._serialize_inner(self)
            }
            /// Write the memory representation of a (slice of a) zero-copy type.
            ///
            /// The default implementation simply delegates to [`WriteNoStd::write_all`].
            /// Other implementations might use the type information in `V` (e.g., [`SchemaWriter`]),
            /// but they must in the end delegate to [`WriteNoStd::write_all`].
            fn write_bytes<V: SerializeInner + ZeroCopy>(
                &mut self,
                value: &[u8],
            ) -> Result<()> {
                self.write_all(value)
            }
        }
        impl<F: WriteNoStd> WriteWithNames for WriterWithPos<'_, F> {}
        /// Information about data written during serialization, either fields or
        /// ancillary data such as option tags and slice lengths.
        pub struct SchemaRow {
            /// Name of the piece of data.
            pub field: String,
            /// Type of the piece of data.
            pub ty: String,
            /// Offset from the start of the file.
            pub offset: usize,
            /// Length in bytes of the piece of data.
            pub size: usize,
            /// The alignment needed by the piece of data, zero if not applicable
            /// (e.g., primitive fields, ancillary data, or structures).
            pub align: usize,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for SchemaRow {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field5_finish(
                    f,
                    "SchemaRow",
                    "field",
                    &self.field,
                    "ty",
                    &self.ty,
                    "offset",
                    &self.offset,
                    "size",
                    &self.size,
                    "align",
                    &&self.align,
                )
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for SchemaRow {
            #[inline]
            fn clone(&self) -> SchemaRow {
                SchemaRow {
                    field: ::core::clone::Clone::clone(&self.field),
                    ty: ::core::clone::Clone::clone(&self.ty),
                    offset: ::core::clone::Clone::clone(&self.offset),
                    size: ::core::clone::Clone::clone(&self.size),
                    align: ::core::clone::Clone::clone(&self.align),
                }
            }
        }
        #[automatically_derived]
        impl mem_dbg::MemDbgImpl for SchemaRow
        where
            String: mem_dbg::MemDbgImpl,
            String: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.field
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("field"),
                        false,
                        _memdbg_flags,
                    )?;
                self.ty
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("ty"),
                        false,
                        _memdbg_flags,
                    )?;
                self.offset
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("offset"),
                        false,
                        _memdbg_flags,
                    )?;
                self.size
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("size"),
                        false,
                        _memdbg_flags,
                    )?;
                self.align
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("align"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl mem_dbg::CopyType for SchemaRow
        where
            String: mem_dbg::MemSize,
            String: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl mem_dbg::MemSize for SchemaRow
        where
            String: mem_dbg::MemSize,
            String: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.field.mem_size(_memsize_flags)
                        - core::mem::size_of::<String>();
                bytes
                    += self.ty.mem_size(_memsize_flags) - core::mem::size_of::<String>();
                bytes
                    += self.offset.mem_size(_memsize_flags)
                        - core::mem::size_of::<usize>();
                bytes
                    += self.size.mem_size(_memsize_flags)
                        - core::mem::size_of::<usize>();
                bytes
                    += self.align.mem_size(_memsize_flags)
                        - core::mem::size_of::<usize>();
                bytes
            }
        }
        /// A vector containing all the fields written during serialization, including
        /// ancillary data such as slice lengths and [`Option`] tags.
        pub struct Schema(pub Vec<SchemaRow>);
        #[automatically_derived]
        impl ::core::default::Default for Schema {
            #[inline]
            fn default() -> Schema {
                Schema(::core::default::Default::default())
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Schema {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Schema", &&self.0)
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Schema {
            #[inline]
            fn clone(&self) -> Schema {
                Schema(::core::clone::Clone::clone(&self.0))
            }
        }
        #[automatically_derived]
        impl mem_dbg::MemDbgImpl for Schema
        where
            Vec<SchemaRow>: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.0
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("0"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl mem_dbg::CopyType for Schema
        where
            Vec<SchemaRow>: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl mem_dbg::MemSize for Schema
        where
            Vec<SchemaRow>: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.0.mem_size(_memsize_flags)
                        - core::mem::size_of::<Vec<SchemaRow>>();
                bytes
            }
        }
        impl Schema {
            /// Return a CSV representation of the schema, including data.
            ///
            /// WARNING: the size of the CSV will be larger than the size of the
            /// serialized file, so it is not a good idea to call this method
            /// on big structures.
            pub fn debug(&self, data: &[u8]) -> String {
                let mut result = "field,offset,align,size,ty,bytes\n".to_string();
                for i in 0..self.0.len().saturating_sub(1) {
                    let row = &self.0[i];
                    if row.offset == self.0[i + 1].offset {
                        result
                            .push_str(
                                &{
                                    let res = ::alloc::fmt::format(
                                        format_args!(
                                            "{0},{1},{2},{3},{4},\n",
                                            row.field,
                                            row.offset,
                                            row.align,
                                            row.size,
                                            row.ty,
                                        ),
                                    );
                                    res
                                },
                            );
                    } else {
                        result
                            .push_str(
                                &{
                                    let res = ::alloc::fmt::format(
                                        format_args!(
                                            "{0},{1},{2},{3},{4},{5:02x?}\n",
                                            row.field,
                                            row.offset,
                                            row.align,
                                            row.size,
                                            row.ty,
                                            &data[row.offset..row.offset + row.size],
                                        ),
                                    );
                                    res
                                },
                            );
                    }
                }
                if let Some(row) = self.0.last() {
                    result
                        .push_str(
                            &{
                                let res = ::alloc::fmt::format(
                                    format_args!(
                                        "{0},{1},{2},{3},{4},{5:02x?}\n",
                                        row.field,
                                        row.offset,
                                        row.align,
                                        row.size,
                                        row.ty,
                                        &data[row.offset..row.offset + row.size],
                                    ),
                                );
                                res
                            },
                        );
                }
                result
            }
            /// Return a CSV representation of the schema, excluding data.
            pub fn to_csv(&self) -> String {
                let mut result = "field,offset,align,size,ty\n".to_string();
                for row in &self.0 {
                    result
                        .push_str(
                            &{
                                let res = ::alloc::fmt::format(
                                    format_args!(
                                        "{0},{1},{2},{3},{4}\n",
                                        row.field,
                                        row.offset,
                                        row.align,
                                        row.size,
                                        row.ty,
                                    ),
                                );
                                res
                            },
                        );
                }
                result
            }
        }
        /// A [`WriteWithNames`] that keeps track of the data written on an underlying
        /// [`WriteWithPos`] in a [`Schema`].
        pub struct SchemaWriter<'a, W> {
            /// The schema so far.
            pub schema: Schema,
            /// A recursively-built sequence of previous names.
            path: Vec<String>,
            /// What we actually write on.
            writer: &'a mut W,
        }
        #[automatically_derived]
        impl<'a, W: ::core::fmt::Debug> ::core::fmt::Debug for SchemaWriter<'a, W> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field3_finish(
                    f,
                    "SchemaWriter",
                    "schema",
                    &self.schema,
                    "path",
                    &self.path,
                    "writer",
                    &&self.writer,
                )
            }
        }
        #[automatically_derived]
        impl<'a, W: ::core::clone::Clone> ::core::clone::Clone for SchemaWriter<'a, W> {
            #[inline]
            fn clone(&self) -> SchemaWriter<'a, W> {
                SchemaWriter {
                    schema: ::core::clone::Clone::clone(&self.schema),
                    path: ::core::clone::Clone::clone(&self.path),
                    writer: ::core::clone::Clone::clone(&self.writer),
                }
            }
        }
        #[automatically_derived]
        impl<'a, W> mem_dbg::MemDbgImpl for SchemaWriter<'a, W>
        where
            Schema: mem_dbg::MemDbgImpl,
            Vec<String>: mem_dbg::MemDbgImpl,
            &'a mut W: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.schema
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("schema"),
                        false,
                        _memdbg_flags,
                    )?;
                self.path
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("path"),
                        false,
                        _memdbg_flags,
                    )?;
                self.writer
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("writer"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl<'a, W> mem_dbg::CopyType for SchemaWriter<'a, W>
        where
            Schema: mem_dbg::MemSize,
            Vec<String>: mem_dbg::MemSize,
            &'a mut W: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl<'a, W> mem_dbg::MemSize for SchemaWriter<'a, W>
        where
            Schema: mem_dbg::MemSize,
            Vec<String>: mem_dbg::MemSize,
            &'a mut W: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.schema.mem_size(_memsize_flags)
                        - core::mem::size_of::<Schema>();
                bytes
                    += self.path.mem_size(_memsize_flags)
                        - core::mem::size_of::<Vec<String>>();
                bytes
                    += self.writer.mem_size(_memsize_flags)
                        - core::mem::size_of::<&'a mut W>();
                bytes
            }
        }
        impl<'a, W: WriteWithPos> SchemaWriter<'a, W> {
            #[inline(always)]
            /// Create a new empty [`SchemaWriter`] on top of a generic writer `W`.
            pub fn new(backend: &'a mut W) -> Self {
                Self {
                    schema: Default::default(),
                    path: ::alloc::vec::Vec::new(),
                    writer: backend,
                }
            }
        }
        impl<W: WriteNoStd> WriteNoStd for SchemaWriter<'_, W> {
            fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
                self.writer.write_all(buf)
            }
            fn flush(&mut self) -> ser::Result<()> {
                self.writer.flush()
            }
        }
        impl<W: WriteWithPos> WriteWithPos for SchemaWriter<'_, W> {
            fn pos(&self) -> usize {
                self.writer.pos()
            }
        }
        /// WARNING: these implementations must be kept in sync with the ones
        /// in the default implementation of [`WriteWithNames`].
        impl<W: WriteWithPos> WriteWithNames for SchemaWriter<'_, W> {
            #[inline(always)]
            fn align<T: MaxSizeOf>(&mut self) -> Result<()> {
                let padding = pad_align_to(self.pos(), T::max_size_of());
                if padding != 0 {
                    self.schema
                        .0
                        .push(SchemaRow {
                            field: "PADDING".into(),
                            ty: {
                                let res = ::alloc::fmt::format(
                                    format_args!("[u8; {0}]", padding),
                                );
                                res
                            },
                            offset: self.pos(),
                            size: padding,
                            align: 1,
                        });
                    for _ in 0..padding {
                        self.write_all(&[0])?;
                    }
                }
                Ok(())
            }
            #[inline(always)]
            fn write<V: SerializeInner>(
                &mut self,
                field_name: &str,
                value: &V,
            ) -> Result<()> {
                self.path.push(field_name.into());
                let pos = self.pos();
                let len = self.schema.0.len();
                value._serialize_inner(self)?;
                self.schema
                    .0
                    .insert(
                        len,
                        SchemaRow {
                            field: self.path.join("."),
                            ty: core::any::type_name::<V>().to_string(),
                            offset: pos,
                            align: 0,
                            size: self.pos() - pos,
                        },
                    );
                self.path.pop();
                Ok(())
            }
            #[inline(always)]
            fn write_bytes<V: SerializeInner + ZeroCopy>(
                &mut self,
                value: &[u8],
            ) -> Result<()> {
                self.path.push("zero".to_string());
                self.schema
                    .0
                    .push(SchemaRow {
                        field: self.path.join("."),
                        ty: core::any::type_name::<V>().to_string(),
                        offset: self.pos(),
                        size: value.len(),
                        align: V::max_size_of(),
                    });
                self.path.pop();
                self.write_all(value)
            }
        }
    }
    pub use write_with_names::*;
    pub mod helpers {
        /*!

Helpers for serialization.

*/
        use super::{SerializeInner, WriteWithNames};
        use crate::ser;
        use crate::traits::*;
        pub fn check_zero_copy<V: SerializeInner>() {
            if !V::IS_ZERO_COPY {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Cannot serialize type {0} declared as zero-copy as it is not zero-copy",
                            core::any::type_name::<V>(),
                        ),
                    );
                };
            }
        }
        /// Serialize a zero-copy structure by writing its bytes properly [aligned](WriteWithNames::align).
        ///
        /// Note that this method uses a single `write_all` call to write the entire structure.
        ///
        /// Here we check [that the type is actually zero-copy](SerializeInner::IS_ZERO_COPY).
        pub fn serialize_zero<V: ZeroCopy + SerializeInner>(
            backend: &mut impl WriteWithNames,
            value: &V,
        ) -> ser::Result<()> {
            check_zero_copy::<V>();
            let buffer = unsafe {
                #[allow(clippy::manual_slice_size_calculation)]
                core::slice::from_raw_parts(
                    value as *const V as *const u8,
                    core::mem::size_of::<V>(),
                )
            };
            backend.align::<V>()?;
            backend.write_bytes::<V>(buffer)
        }
        /// Serialize a slice of zero-copy structures by encoding
        /// its length first, and then its bytes properly [aligned](WriteWithNames::align).
        ///
        /// Note that this method uses a single `write_all`
        /// call to write the entire slice.
        ///
        /// Here we check [that the type is actually zero-copy](SerializeInner::IS_ZERO_COPY).
        pub fn serialize_slice_zero<V: SerializeInner + ZeroCopy>(
            backend: &mut impl WriteWithNames,
            data: &[V],
        ) -> ser::Result<()> {
            check_zero_copy::<V>();
            let len = data.len();
            backend.write("len", &len)?;
            let buffer = unsafe {
                #[allow(clippy::manual_slice_size_calculation)]
                core::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    len * core::mem::size_of::<V>(),
                )
            };
            backend.align::<V>()?;
            backend.write_bytes::<V>(buffer)
        }
        pub fn check_mismatch<V: SerializeInner>() {
            if V::ZERO_COPY_MISMATCH {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "Type {0} is zero-copy, but it has not declared as such; use the #[deep_copy] attribute to silence this warning\n",
                            core::any::type_name::<V>(),
                        ),
                    );
                };
            }
        }
        /// Serialize a slice of deep-copy structures by encoding
        /// its length first, and then the contents item by item.
        ///
        /// Here we warn [that the type might actually be zero-copy](SerializeInner::ZERO_COPY_MISMATCH).
        pub fn serialize_slice_deep<V: SerializeInner>(
            backend: &mut impl WriteWithNames,
            data: &[V],
        ) -> ser::Result<()> {
            check_mismatch::<V>();
            let len = data.len();
            backend.write("len", &len)?;
            for item in data.iter() {
                backend.write("item", item)?;
            }
            Ok(())
        }
    }
    pub use helpers::*;
    pub mod write {
        /*!

No-std support for writing while keeping track of the current position.

 */
        use crate::prelude::*;
        use mem_dbg::{MemDbg, MemSize};
        /// [`std::io::Write`]-like trait for serialization that does not
        /// depend on [`std`].
        ///
        /// In an [`std`] context, the user does not need to use directly
        /// this trait as we provide a blanket
        /// implementation that implements [`WriteNoStd`] for all types that implement
        /// [`std::io::Write`]. In particular, in such a context you can use [`std::io::Cursor`]
        /// for in-memory serialization.
        pub trait WriteNoStd {
            /// Write some bytes.
            fn write_all(&mut self, buf: &[u8]) -> ser::Result<()>;
            /// Flush all changes to the underlying storage if applicable.
            fn flush(&mut self) -> ser::Result<()>;
        }
        #[cfg(feature = "std")]
        use std::io::Write;
        #[cfg(feature = "std")]
        impl<W: Write> WriteNoStd for W {
            #[inline(always)]
            fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
                Write::write_all(self, buf).map_err(|_| ser::Error::WriteError)
            }
            #[inline(always)]
            fn flush(&mut self) -> ser::Result<()> {
                Write::flush(self).map_err(|_| ser::Error::WriteError)
            }
        }
        /// A trait for [`WriteNoStd`] that also keeps track of the current position.
        ///
        /// This is needed because the [`Write`] trait doesn't have a `seek` method and
        /// [`std::io::Seek`] would be a requirement much stronger than needed.
        pub trait WriteWithPos: WriteNoStd {
            fn pos(&self) -> usize;
        }
        /// A wrapper for a [`WriteNoStd`] that implements [`WriteWithPos`]
        /// by keeping track of the current position.
        pub struct WriterWithPos<'a, F: WriteNoStd> {
            /// What we actually write on.
            backend: &'a mut F,
            /// How many bytes we have written from the start.
            pos: usize,
        }
        #[automatically_derived]
        impl<'a, F: ::core::fmt::Debug + WriteNoStd> ::core::fmt::Debug
        for WriterWithPos<'a, F> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "WriterWithPos",
                    "backend",
                    &self.backend,
                    "pos",
                    &&self.pos,
                )
            }
        }
        #[automatically_derived]
        impl<'a, F: ::core::clone::Clone + WriteNoStd> ::core::clone::Clone
        for WriterWithPos<'a, F> {
            #[inline]
            fn clone(&self) -> WriterWithPos<'a, F> {
                WriterWithPos {
                    backend: ::core::clone::Clone::clone(&self.backend),
                    pos: ::core::clone::Clone::clone(&self.pos),
                }
            }
        }
        #[automatically_derived]
        impl<'a, F: WriteNoStd> mem_dbg::MemDbgImpl for WriterWithPos<'a, F>
        where
            &'a mut F: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.backend
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("backend"),
                        false,
                        _memdbg_flags,
                    )?;
                self.pos
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("pos"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl<'a, F: WriteNoStd> mem_dbg::CopyType for WriterWithPos<'a, F>
        where
            &'a mut F: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl<'a, F: WriteNoStd> mem_dbg::MemSize for WriterWithPos<'a, F>
        where
            &'a mut F: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.backend.mem_size(_memsize_flags)
                        - core::mem::size_of::<&'a mut F>();
                bytes
                    += self.pos.mem_size(_memsize_flags) - core::mem::size_of::<usize>();
                bytes
            }
        }
        impl<'a, F: WriteNoStd> WriterWithPos<'a, F> {
            #[inline(always)]
            /// Create a new [`WriterWithPos`] on top of a generic [`WriteNoStd`] `F`.
            pub fn new(backend: &'a mut F) -> Self {
                Self { backend, pos: 0 }
            }
        }
        impl<'a, F: WriteNoStd> WriteNoStd for WriterWithPos<'a, F> {
            #[inline(always)]
            fn write_all(&mut self, buf: &[u8]) -> ser::Result<()> {
                self.backend.write_all(buf)?;
                self.pos += buf.len();
                Ok(())
            }
            #[inline(always)]
            fn flush(&mut self) -> ser::Result<()> {
                self.backend.flush()
            }
        }
        impl<'a, F: WriteNoStd> WriteWithPos for WriterWithPos<'a, F> {
            #[inline(always)]
            fn pos(&self) -> usize {
                self.pos
            }
        }
    }
    pub use write::*;
    pub type Result<T> = core::result::Result<T, Error>;
    /// Main serialization trait. It is separated from [`SerializeInner`] to
    /// avoid that the user modify its behavior, and hide internal serialization
    /// methods.
    ///
    /// It provides a convenience method [`Serialize::store`] that serializes
    /// the type to a file.
    pub trait Serialize: TypeHash + ReprHash {
        /// Serialize the type using the given backend.
        fn serialize(&self, backend: &mut impl WriteNoStd) -> Result<usize> {
            let mut write_with_pos = WriterWithPos::new(backend);
            self.serialize_on_field_write(&mut write_with_pos)?;
            Ok(write_with_pos.pos())
        }
        /// Serialize the type using the given backend and return a [schema](Schema)
        /// describing the data that has been written.
        ///
        /// This method is mainly useful for debugging and to check cross-language
        /// interoperability.
        fn serialize_with_schema(
            &self,
            backend: &mut impl WriteNoStd,
        ) -> Result<Schema> {
            let mut writer_with_pos = WriterWithPos::new(backend);
            let mut schema_writer = SchemaWriter::new(&mut writer_with_pos);
            self.serialize_on_field_write(&mut schema_writer)?;
            Ok(schema_writer.schema)
        }
        /// Serialize the type using the given [`WriteWithNames`].
        fn serialize_on_field_write(
            &self,
            backend: &mut impl WriteWithNames,
        ) -> Result<()>;
        /// Commodity method to serialize to a file.
        fn store(&self, path: impl AsRef<Path>) -> Result<()> {
            let file = std::fs::File::create(path).map_err(Error::FileOpenError)?;
            let mut buf_writer = BufWriter::new(file);
            self.serialize(&mut buf_writer)?;
            Ok(())
        }
    }
    /// Inner trait to implement serialization of a type. This trait exists
    /// to separate the user-facing [`Serialize`] trait from the low-level
    /// serialization mechanism of [`SerializeInner::_serialize_inner`]. Moreover,
    /// it makes it possible to behave slighly differently at the top
    /// of the recursion tree (e.g., to write the endianness marker), and to prevent
    /// the user from modifying the methods in [`Serialize`].
    ///
    /// The user should not implement this trait directly, but rather derive it.
    pub trait SerializeInner {
        /// Inner constant used by the derive macros to keep
        /// track recursively of whether the type
        /// satisfies the conditions for being zero-copy. It is checked
        /// at runtime against the trait implemented by the type, and
        /// if a [`ZeroCopy`] type has this constant set to `false`
        /// serialization will panic.
        const IS_ZERO_COPY: bool;
        /// Inner constant used by the derive macros to keep
        /// track of whether all fields of a type are zero-copy
        /// but neither the attribute `#[zero_copy]` nor the attribute `#[deep_copy]`
        /// was specified. It is checked at runtime, and if it is true
        /// a warning will be issued, as the type could be zero-copy,
        /// which would be more efficient.
        const ZERO_COPY_MISMATCH: bool;
        /// Serialize this structure using the given backend.
        fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> Result<()>;
    }
    /// Blanket implementation that prevents the user from overwriting the
    /// methods in [`Serialize`].
    ///
    /// This implementation [writes a header](`write_header`) containing some hashes
    /// and debug information and then delegates to [WriteWithNames::write].
    impl<T: SerializeInner + TypeHash + ReprHash> Serialize for T {
        /// Serialize the type using the given [`WriteWithNames`].
        fn serialize_on_field_write(
            &self,
            backend: &mut impl WriteWithNames,
        ) -> Result<()> {
            write_header::<Self>(backend)?;
            backend.write("ROOT", self)?;
            backend.flush()
        }
    }
    /// Write the header.
    ///
    /// Must be kept in sync with [`crate::deser::check_header`].
    pub fn write_header<T: TypeHash + ReprHash>(
        backend: &mut impl WriteWithNames,
    ) -> Result<()> {
        backend.write("MAGIC", &MAGIC)?;
        backend.write("VERSION_MAJOR", &VERSION.0)?;
        backend.write("VERSION_MINOR", &VERSION.1)?;
        backend.write("USIZE_SIZE", &(core::mem::size_of::<usize>() as u8))?;
        let mut type_hasher = xxhash_rust::xxh3::Xxh3::new();
        T::type_hash(&mut type_hasher);
        let mut repr_hasher = xxhash_rust::xxh3::Xxh3::new();
        let mut offset_of = 0;
        T::repr_hash(&mut repr_hasher, &mut offset_of);
        backend.write("TYPE_HASH", &type_hasher.finish())?;
        backend.write("REPR_HASH", &repr_hasher.finish())?;
        backend.write("TYPE_NAME", &core::any::type_name::<T>().to_string())
    }
    /// A helper trait that makes it possible to implement differently
    /// serialization for [`crate::traits::ZeroCopy`] and [`crate::traits::DeepCopy`] types.
    /// See [`crate::traits::CopyType`] for more information.
    pub trait SerializeHelper<T: CopySelector> {
        fn _serialize_inner(&self, backend: &mut impl WriteWithNames) -> Result<()>;
    }
    /// Errors that can happen during serialization.
    pub enum Error {
        /// The underlying writer returned an error.
        WriteError,
        /// [`Serialize::store`] could not open the provided file.
        FileOpenError(std::io::Error),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Error {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Error::WriteError => ::core::fmt::Formatter::write_str(f, "WriteError"),
                Error::FileOpenError(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "FileOpenError",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl std::error::Error for Error {}
    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                Self::WriteError => {
                    f.write_fmt(
                        format_args!("Write error during ε-serde serialization"),
                    )
                }
                Self::FileOpenError(error) => {
                    f.write_fmt(
                        format_args!(
                            "Error opening file during ε-serde serialization: {0}",
                            error,
                        ),
                    )
                }
            }
        }
    }
}
pub mod traits {
    /*!

Basic traits that must be implemented by all types using ε-serde.

If you use the procedural macro [`Epserde`](epserde_derive::Epserde), you do not
need to worry about these traits—they will be implemented for you.

*/
    pub mod type_info {
        /*!

Traits computing information about a type.

*/
        use crate::pad_align_to;
        use core::hash::Hash;
        /// Recursively compute a type hash for a type.
        ///
        /// [`TypeHash::type_hash`] is a recursive function that computes information
        /// about a type. It is used to
        /// check that the type of the data being deserialized matches
        /// syntactically the type of the data that was written.
        ///
        /// The type hasher should store information about the name and the type
        /// of the fields of a type, and the name of the type itself.
        pub trait TypeHash {
            /// Accumulate type information in `hasher`.
            fn type_hash(hasher: &mut impl core::hash::Hasher);
            /// Call [`TypeHash::type_hash`] on a value.
            fn type_hash_val(&self, hasher: &mut impl core::hash::Hasher) {
                Self::type_hash(hasher);
            }
        }
        /// Recursively compute a representational hash for a type.
        ///
        /// [`ReprHash::repr_hash`] is a recursive function that computes
        /// representation information about a zero-copy type. It is used to
        /// check that the the alignment and the representation data
        /// of the data being deserialized.
        ///
        /// More precisely, at each call a zero-copy type look at `offset_of`,
        /// assuming that the type is stored at that offset in the structure,
        /// hashes in the padding necessary to make `offset_of` a multiple of
        /// [`core::mem::align_of`] the type, hashes in the type size, and
        /// finally increases `offset_of` by [`core::mem::size_of`] the type.
        pub trait ReprHash {
            /// Accumulate representional information in `hasher` assuming to
            /// be positioned at `offset_of`.
            fn repr_hash(_hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize);
            /// Call [`ReprHash::repr_hash`] on a value.
            fn repr_hash_val(
                &self,
                hasher: &mut impl core::hash::Hasher,
                offset_of: &mut usize,
            ) {
                Self::repr_hash(hasher, offset_of);
            }
        }
        /// A function providing a reasonable default
        /// implementation of [`ReprHash::repr_hash`] for basic sized types.
        pub(crate) fn std_repr_hash<T>(
            hasher: &mut impl core::hash::Hasher,
            offset_of: &mut usize,
        ) {
            let padding = pad_align_to(*offset_of, core::mem::align_of::<T>());
            padding.hash(hasher);
            core::mem::size_of::<T>().hash(hasher);
            *offset_of += padding;
            *offset_of += core::mem::size_of::<T>();
        }
        /// A trait providing the maximum size of a primitive field in a type
        /// maximized with [`core::mem::align_of`].
        ///
        /// We use the value returned by [`MaxSizeOf::max_size_of`]
        /// to generate padding before storing a zero-copy type. Note that this
        /// is different from the padding used to align the same type inside
        /// a struct, which is not under our control and is
        /// given by [`core::mem::align_of`].
        ///
        /// In this way we increase interoperability between architectures
        /// with different alignment requirements for the same types (e.g.,
        /// 4 or 8 bytes for `u64`).
        ///
        /// By maximizing with [`core::mem::align_of`] we ensure that
        /// we provide sufficient alignment in case the attribute `repr(align(N))`
        /// was specified.
        pub trait MaxSizeOf: Sized {
            fn max_size_of() -> usize;
        }
    }
    pub use type_info::*;
    pub mod copy_type {
        /*!

Traits to mark types as zero-copy or deep-copy.

*/
        use crate::prelude::MaxSizeOf;
        use sealed::sealed;
        #[automatically_derived]
        mod __seal_copy_selector {
            use super::*;
            pub trait Sealed {}
        }
        /// Internal trait used to select whether a type is zero-copy
        /// or deep-copy.
        ///
        /// It has only two implementations, [`Zero`] and [`Deep`].
        ///
        /// In the first case, the type can be serialized
        /// from memory and deserialized to memory as a sequence of bytes;
        /// in the second case, one has to deserialize the type field
        /// by field.
        pub trait CopySelector: __seal_copy_selector::Sealed {
            const IS_ZERO_COPY: bool;
        }
        /// An implementation of a [`CopySelector`] specifying that a type is zero-copy.
        pub struct Zero {}
        #[automatically_derived]
        impl __seal_copy_selector::Sealed for Zero {}
        impl CopySelector for Zero {
            const IS_ZERO_COPY: bool = true;
        }
        /// An implementation of a [`CopySelector`] specifying that a type is deep-copy.
        pub struct Deep {}
        #[automatically_derived]
        impl __seal_copy_selector::Sealed for Deep {}
        impl CopySelector for Deep {
            const IS_ZERO_COPY: bool = false;
        }
        /**

Marker trait for data specifying whether it is zero-copy or deep-copy.

The trait comes in two flavors: `CopySelector<Type=Zero>` and
`CopySelector<Type=Deep>`. To each of these flavors corresponds two
dependent traits, [`ZeroCopy`] (which requires implementing [`MaxSizeOf`])
and [`DeepCopy`], which are automatically
implemented.
```rust
use epserde::traits::*;

struct MyType {}

impl CopyType for MyType {
    type Copy = Deep;
}
// Now MyType implements DeepCopy
```
You should not implement this trait manually, but rather use the provided [derive macro](epserde_derive::Epserde).

We use this trait to implement a different behavior for [`ZeroCopy`] and [`DeepCopy`] types,
in particular on arrays, vectors, and boxed slices,
[working around the bug that prevents the compiler from understanding that implementations
for the two flavors of `CopySelector` are mutually
exclusive](https://github.com/rust-lang/rfcs/pull/1672#issuecomment-1405377983).

For an array of elements of type `T` to be zero-copy serializable and
deserializable, `T` must implement `CopySelector<Type=Zero>`. The conditions for this marker trait are that
`T` is a [copy type](Copy), that it has a fixed memory layout,
and that it does not contain any reference (in particular, that it has `'static` lifetime).
If this happen vectors of `T` or boxed slices of `T` can be ε-copy deserialized
using a reference to a slice of `T`.

You can make zero-copy your own types, but you must ensure that they do not
contain references and that they have a fixed memory layout; for structures, this requires
`repr(C)`. ε-serde will track these conditions at compile time and check them at
runtime: in case of failure, serialization will panic.

Since we cannot use negative trait bounds, every type that is used as a parameter of
an array, vector or boxed slice must implement either `CopySelector<Type=Zero>`
or `CopySelector<Type=Deep>`. In the latter
case, slices will be deserialized element by element, and the result will be a fully
deserialized vector or boxed
slice. If you do not implement either of these traits, the type will not be serializable inside
vectors or boxed slices but error messages will be very unhelpful due to the
contrived way we have to implement mutually exclusive types.

If you use the provided derive macros all this logic will be hidden from you. You'll
just have to add `#[zero_copy]` to your structures (if you want them to be zero-copy)
and ε-serde will do the rest.

*/
        pub trait CopyType: Sized {
            type Copy: CopySelector;
        }
        /// Marker trait for zero-copy types. You should never implement
        /// this trait directly, but rather implement [`CopyType`] with `Copy=Zero`.
        pub trait ZeroCopy: CopyType<Copy = Zero> + Copy + MaxSizeOf + 'static {}
        impl<T: CopyType<Copy = Zero> + Copy + MaxSizeOf + 'static> ZeroCopy for T {}
        /// Marker trait for deep-copy types. You should never implement
        /// this trait directly, but rather implement [`CopyType`] with `Copy=Deep`.
        pub trait DeepCopy: CopyType<Copy = Deep> {}
        impl<T: CopyType<Copy = Deep>> DeepCopy for T {}
    }
    pub use copy_type::*;
}
pub mod utils {
    mod aligned_cursor {
        use core::slice;
        use std::io::{Read, Seek, SeekFrom, Write};
        use maligned::{Alignment, A16};
        use mem_dbg::{MemDbg, MemSize};
        /// An aligned version of [`Cursor`](std::io::Cursor).
        ///
        /// The standard library implementation of a [cursor](std::io::Cursor) is not
        /// aligned, and thus cannot be used to create examples or unit tests for
        /// ε-serde. This version has a [settable alignment](maligned::Alignment) that
        /// is guaranteed to be respected by the underlying storage.
        ///
        /// Note that length and position are stored as `usize` values, so the maximum
        /// length and position are `usize::MAX`. This is different from
        /// [`Cursor`](std::io::Cursor), which uses a `u64`.
        pub struct AlignedCursor<T: Alignment = A16> {
            vec: Vec<T>,
            pos: usize,
            len: usize,
        }
        #[automatically_derived]
        impl<T: ::core::fmt::Debug + Alignment> ::core::fmt::Debug for AlignedCursor<T> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field3_finish(
                    f,
                    "AlignedCursor",
                    "vec",
                    &self.vec,
                    "pos",
                    &self.pos,
                    "len",
                    &&self.len,
                )
            }
        }
        #[automatically_derived]
        impl<T: ::core::clone::Clone + Alignment> ::core::clone::Clone
        for AlignedCursor<T> {
            #[inline]
            fn clone(&self) -> AlignedCursor<T> {
                AlignedCursor {
                    vec: ::core::clone::Clone::clone(&self.vec),
                    pos: ::core::clone::Clone::clone(&self.pos),
                    len: ::core::clone::Clone::clone(&self.len),
                }
            }
        }
        #[automatically_derived]
        impl<T: Alignment> mem_dbg::MemDbgImpl for AlignedCursor<T>
        where
            Vec<T>: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
            usize: mem_dbg::MemDbgImpl,
        {
            #[inline(always)]
            fn _mem_dbg_rec_on(
                &self,
                _memdbg_writer: &mut impl core::fmt::Write,
                _memdbg_total_size: usize,
                _memdbg_max_depth: usize,
                _memdbg_prefix: &mut String,
                _memdbg_is_last: bool,
                _memdbg_flags: mem_dbg::DbgFlags,
            ) -> core::fmt::Result {
                self.vec
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("vec"),
                        false,
                        _memdbg_flags,
                    )?;
                self.pos
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("pos"),
                        false,
                        _memdbg_flags,
                    )?;
                self.len
                    .mem_dbg_depth_on(
                        _memdbg_writer,
                        _memdbg_total_size,
                        _memdbg_max_depth,
                        _memdbg_prefix,
                        Some("len"),
                        true,
                        _memdbg_flags,
                    )?;
                Ok(())
            }
        }
        #[automatically_derived]
        impl<T: Alignment> mem_dbg::CopyType for AlignedCursor<T>
        where
            Vec<T>: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            type Copy = mem_dbg::False;
        }
        #[automatically_derived]
        impl<T: Alignment> mem_dbg::MemSize for AlignedCursor<T>
        where
            Vec<T>: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
            usize: mem_dbg::MemSize,
        {
            fn mem_size(&self, _memsize_flags: mem_dbg::SizeFlags) -> usize {
                let mut bytes = core::mem::size_of::<Self>();
                bytes
                    += self.vec.mem_size(_memsize_flags)
                        - core::mem::size_of::<Vec<T>>();
                bytes
                    += self.pos.mem_size(_memsize_flags) - core::mem::size_of::<usize>();
                bytes
                    += self.len.mem_size(_memsize_flags) - core::mem::size_of::<usize>();
                bytes
            }
        }
        impl<T: Alignment> AlignedCursor<T> {
            /// Return a new empty [`AlignedCursor`].
            pub fn new() -> Self {
                Self {
                    vec: Vec::new(),
                    pos: 0,
                    len: 0,
                }
            }
            /// Return a new empty [`AlignedCursor`] with a specified capacity.
            pub fn with_capacity(capacity: usize) -> Self {
                Self {
                    vec: Vec::with_capacity(capacity.div_ceil(std::mem::size_of::<T>())),
                    pos: 0,
                    len: 0,
                }
            }
            /// Consume this cursor, returning the underlying storage and the length of
            /// the data in bytes.
            pub fn into_parts(self) -> (Vec<T>, usize) {
                (self.vec, self.len)
            }
            /// Return a reference to the underlying storage as bytes.
            ///
            /// Only the first [len](AlignedCursor::len) bytes are valid.
            ///
            /// Note that the reference is always to the whole storage,
            /// independently of the current [position](AlignedCursor::position).
            pub fn as_bytes(&mut self) -> &[u8] {
                let ptr = self.vec.as_mut_ptr() as *mut u8;
                unsafe { slice::from_raw_parts(ptr, self.len) }
            }
            /// Return a mutable reference to the underlying storage as bytes.
            ///
            /// Only the first [len](AlignedCursor::len) bytes are valid.
            ///
            /// Note that the reference is always to the whole storage,
            /// independently of the current [position](AlignedCursor::position).
            pub fn as_bytes_mut(&mut self) -> &mut [u8] {
                let ptr = self.vec.as_mut_ptr() as *mut u8;
                unsafe { slice::from_raw_parts_mut(ptr, self.len) }
            }
            /// Return the length in bytes of the data in this cursor.
            pub fn len(&self) -> usize {
                self.len
            }
            /// Return whether this cursor contains no data.
            pub fn is_empty(&self) -> bool {
                self.len == 0
            }
            /// Return the current position of this cursor.
            pub fn position(&self) -> usize {
                self.pos
            }
            /// Set the current position of this cursor.
            ///
            /// Valid positions are all `usize` values.
            pub fn set_position(&mut self, pos: usize) {
                self.pos = pos;
            }
        }
        impl<T: Alignment> Default for AlignedCursor<T> {
            fn default() -> Self {
                Self::new()
            }
        }
        impl<T: Alignment> Read for AlignedCursor<T> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.pos >= self.len {
                    return Ok(0);
                }
                let pos = self.pos;
                let rem = self.len - pos;
                let to_copy = std::cmp::min(buf.len(), rem) as usize;
                buf[..to_copy].copy_from_slice(&self.as_bytes()[pos..pos + to_copy]);
                self.pos += to_copy;
                Ok(to_copy)
            }
        }
        impl<T: Alignment> Write for AlignedCursor<T> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let len = buf.len().min(usize::MAX - self.pos);
                if !buf.is_empty() && len == 0 {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "write operation overflows usize::MAX length limit",
                        ),
                    );
                }
                let cap = self.vec.len().saturating_mul(std::mem::size_of::<T>());
                let rem = cap - self.pos;
                if rem < len {
                    self.vec
                        .resize(
                            (self.pos + len).div_ceil(std::mem::size_of::<T>()),
                            T::default(),
                        );
                }
                let pos = self.pos;
                let bytes = unsafe {
                    slice::from_raw_parts_mut(
                        self.vec.as_mut_ptr() as *mut u8,
                        self.vec.len() * std::mem::size_of::<T>(),
                    )
                };
                bytes[pos..pos + len].copy_from_slice(buf);
                self.pos += len;
                self.len = self.len.max(self.pos);
                Ok(len)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<T: Alignment> Seek for AlignedCursor<T> {
            fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
                let (base_pos, offset) = match style {
                    SeekFrom::Start(n) if n > usize::MAX as u64 => {
                        return Err(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                "cursor length would be greater than usize::MAX",
                            ),
                        );
                    }
                    SeekFrom::Start(n) => {
                        self.pos = n as usize;
                        return Ok(self.pos as u64);
                    }
                    SeekFrom::End(n) => (self.len as u64, n),
                    SeekFrom::Current(n) => (self.pos as u64, n),
                };
                match base_pos.checked_add_signed(offset) {
                    Some(n) if n <= usize::MAX as u64 => {
                        self.pos = n as usize;
                        Ok(n)
                    }
                    _ => {
                        Err(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                "invalid seek to a negative or overflowing position",
                            ),
                        )
                    }
                }
            }
            fn stream_position(&mut self) -> std::io::Result<u64> {
                Ok(self.pos as u64)
            }
        }
    }
    pub use aligned_cursor::AlignedCursor;
}
pub mod prelude {
    pub use crate::deser;
    pub use crate::deser::DeserType;
    pub use crate::deser::Deserialize;
    pub use crate::deser::DeserializeHelper;
    pub use crate::deser::DeserializeInner;
    pub use crate::deser::Flags;
    pub use crate::deser::MemCase;
    pub use crate::deser::ReadWithPos;
    pub use crate::deser::SliceWithPos;
    pub use crate::ser;
    pub use crate::ser::Serialize;
    pub use crate::ser::SerializeHelper;
    pub use crate::ser::SerializeInner;
    pub use crate::traits::*;
    pub use crate::utils::*;
    #[cfg(feature = "derive")]
    pub use epserde_derive::Epserde;
}
/// (Major, Minor) version of the file format, this follows semantic versioning
pub const VERSION: (u16, u16) = (1, 1);
/// Magic cookie, also used as endianess marker.
pub const MAGIC: u64 = u64::from_ne_bytes(*b"epserde ");
/// What we will read if the endianness is mismatched.
pub const MAGIC_REV: u64 = u64::from_le_bytes(MAGIC.to_be_bytes());
/// Compute the padding needed for alignment, that is, the smallest
/// number such that `((value + pad_align_to(value, align_to) & (align_to - 1) == 0`.
pub fn pad_align_to(value: usize, align_to: usize) -> usize {
    value.wrapping_neg() & (align_to - 1)
}
