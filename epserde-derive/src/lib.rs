/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#![allow(clippy::collapsible_if)]

//! Derive procedural macros for the [`epserde`] crate.
//!
//! [`epserde`]: https://crates.io/crates/epserde

mod attrs;
mod epserde;
mod type_info;
mod utils;

/// Generates an [ε-serde] implementation for custom types.
///
/// It generates implementations for the traits `CopyType`, `TypeHash`,
/// `AlignHash`, `SerInner`, and `DeserInner` (and `PadTo` for zero-copy
/// types).
///
/// Presently we do not support unions, where clauses on the original type,
/// and lifetime generics.
///
/// The attribute `#[epserde(zero_copy)]` can be used to generate an
/// implementation for a zero-copy type, but the type must be `repr(C)` and all
/// fields must be zero-copy.
///
/// If you do not specify `#[epserde(zero_copy)]`, the macro assumes your
/// structure is deep-copy. However, if you have a structure that could be
/// zero-copy, but has no attribute, a compile-time error (a `const` assertion in
/// the generated `_ser_inner`) will be raised when you serialize an instance of
/// the type. The error can be silenced by adding the explicit attribute
/// `#[epserde(deep_copy)]`.
///
/// You can specify additional where-clause bounds for the generated
/// (de)serialization implementations using `#[epserde(bound(deser = "...", ser
/// = "..."))]`. This is useful when a field type involves an associated type
/// of an ε-copy type parameter, as the associated type needs to be pinned
/// to remain the same after replacement. For example:
/// ```ignore
/// #[derive(Epserde)]
/// #[epserde(bound(
///     deser = "for<'a> <B as DeserInner>::DeserType<'a>: WordType<Word = B::Word>"
/// ))]
/// pub struct BitFieldVec<B: WordType = Vec<usize>> {
///     bits: B,
///     mask: B::Word,
/// }
/// ```
///
/// # The `force_full_copy` field attribute
///
/// A field-level marker (no arguments) that pins a field to full-copy
/// deserialization and keeps its type verbatim in `DeserType<'_>`.
///
/// By default, when a field type mentions a type parameter, that field is
/// deserialized via the ε-copy path and the parameter is ε-copy: in
/// `Self::DeserType<'a>` it is substituted with `<T as DeserInner>::
/// DeserType<'a>`. Occurrences nested inside `PhantomData<…>` are transparent
/// and do not count. Fields whose type mentions no type parameter default to
/// full-copy: there is nothing to substitute.
///
/// `#[epserde(force_full_copy)]` opts a single field out of the default:
///
/// - the field is deserialized full-copy, rather than ε-copy;
/// - its type is preserved verbatim in `Self::DeserType<'a>`;
/// - its occurrences of type parameters do not contribute to the ε-copy parameters.
///
/// The name carries intent: the field *could* be ε-copy under the default, and
/// you are deliberately *forcing* it full-copy instead.
///
/// Typical use: a field whose type is `Vec<T>` but the surrounding struct is to
/// be full-copy, or a wrapper whose `DeserType<'_>` cannot follow
/// the uniform-substitution contract that ε-copy deserialization requires.
///
/// The marker takes no arguments and affects only deserialization.
/// It is rejected if it appears anywhere inside a type marked
/// `#[epserde(zero_copy)]`: zero-copy structs are (de)serialized as a
/// sequence of raw bytes with no field-level choice between
/// `_deser_full_inner` and `_deser_eps_inner`, so the marker has no
/// operational meaning there. On a deep-copy field whose type mentions no type
/// parameter the marker is a silent no-op: the field is already full-copy
/// by default, since there is nothing to substitute.
///
/// Example:
///
/// ```ignore
/// #[derive(Epserde)]
/// struct Outer<T> {
///     #[epserde(force_full_copy)]
///     data: Vec<T>,  // stays as Vec<T> in DeserType<'_>, full-copy
/// }
/// ```
///
/// # The `full_copy(...)` type-level attribute
///
/// A type-level attribute that pins one or more type parameters to
/// full-copy deserialization. It takes a comma-separated list of type
/// parameters of the item: `#[epserde(full_copy(T, U))]`.
///
/// The derive classifies a parameter as ε-copy whenever it occurs in an ε-copy
/// field. That syntactic test can only err in one direction: it assumes the
/// enclosing field type substitutes the parameter transitively in its own
/// `DeserType<'_>`, which a nested type need not do (it may hold the parameter
/// in its own full-copy field). When that assumption is wrong the generated
/// `_deser_eps_inner` body fails to type-check.
///
/// `full_copy(T)` is the escape hatch for that case. Unlike the field marker,
/// it is a *declaration* rather than a *force*: the parameter genuinely is
/// full-copy (a nested type holds it that way), but the local syntactic walk
/// could not see it, so no "force" is implied. It removes `T` from the
/// `DeserType` substitution set: `T` is kept verbatim in `Self::DeserType<'a>`
/// and any field whose type parameters are all listed is full-copy. It
/// affects only deserialization (`DeserType`); `SerType` keeps normalizing `T`.
///
/// It is rejected on a `#[epserde(zero_copy)]` type (whose `DeserType<'a>` is
/// `&'a Self`, substituting nothing), on a const parameter, and on an
/// identifier that is not a declared type parameter. Listing a parameter that
/// is already full-copy (or that does not occur in any field) has no effect.
///
/// Example: `Inner` holds `T` in a field-level `force_full_copy` slot, so the
/// walk's transitive-substitution assumption fails for `Outer`; the attribute
/// repairs it.
///
/// ```ignore
/// #[derive(Epserde)]
/// struct Inner<T> {
///     #[epserde(force_full_copy)]
///     x: T,
/// }
///
/// #[derive(Epserde)]
/// #[epserde(full_copy(T))]
/// struct Outer<T> {
///     inner: Inner<T>,  // Inner<T>::DeserType<'_> = Inner<T>
/// }
/// ```
///
/// # The `phantom(...)` type-level attribute
///
/// A type-level attribute that declares one or more type parameters phantom
/// throughout the type. It takes a comma-separated list of type parameters of
/// the item: `#[epserde(phantom(T, U))]`.
///
/// A parameter may be listed only if every occurrence of it in field types is
/// inside a `PhantomData`. Listed parameters are excluded from the
/// replaceable-parameter walk entirely: neither `SerType` nor `DeserType<'_>`
/// substitutes them, and no `SerInner`/`DeserInner` bounds are emitted for
/// them, so they can be instantiated with non-serializable types such as
/// `str`.
///
/// It shares the rejections of `full_copy(...)`: it is rejected on a
/// `#[epserde(zero_copy)]` type, on a const parameter, and on an identifier
/// that is not a declared type parameter. Moreover, a parameter cannot be
/// listed both in `phantom(...)` and in `full_copy(...)`.
///
/// Example: `T` occurs only inside `PhantomData`, so it can be declared
/// phantom and instantiated with `str`:
///
/// ```ignore
/// // Here the derive code can establish locally that T is inside a PhantomData
/// #[derive(Epserde)]
/// struct HiddenPhantom<T: ?Sized> {
///     marker: PhantomData<T>,
/// }
///
/// // Here we need the attribute
/// #[derive(Epserde)]
/// #[epserde(phantom(T))]
/// struct Data<T: ?Sized, U> {
///     data: U,
///     hidden: HiddenPhantom<T>,
/// }
/// ```
///
/// [ε-serde]: Epserde
#[proc_macro_derive(Epserde, attributes(epserde))]
pub fn epserde_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    epserde::derive(input)
}

/// Generates a [partial ε-serde] implementation for custom types.
///
/// It generates implementations just for the traits `TypeHash` and `AlignHash`
/// (plus `PadTo` for zero-copy types), but not for `CopyType`, `SerInner`, or
/// `DeserInner`. See the documentation of [`Epserde`] for more information.
///
/// [partial ε-serde]: TypeInfo
#[proc_macro_derive(TypeInfo, attributes(epserde))]
pub fn type_info_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    type_info::derive(input)
}
