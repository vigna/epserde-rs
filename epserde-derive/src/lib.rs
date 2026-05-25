/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![allow(clippy::collapsible_if)]

//! Derive procedural macros for the [`epserde`](https://crates.io/crates/epserde) crate.

use quote::{ToTokens, quote};
use std::{collections::HashSet, vec};
use syn::{
    BoundLifetimes, Data, DeriveInput, GenericParam, ImplGenerics, LifetimeParam, PredicateType,
    TypeGenerics, TypeParamBound, WhereClause, WherePredicate, parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{self, Plus},
};

//
// `Epserde` derive macro implementation
//

/// Returns an empty where clause.
fn empty_where_clause() -> WhereClause {
    WhereClause {
        where_token: token::Where::default(),
        predicates: Punctuated::new(),
    }
}

/// Returns a field name as a token stream.
///
/// This method takes care transparently of unnamed fields (i.e., fields tuple
/// structs), and for this reason it can only return a
/// [`proc_macro2::TokenStream`] instead of a more specific type such as
/// [`struct@syn::Ident`].
fn get_field_name(field: &syn::Field, field_idx: usize) -> proc_macro2::TokenStream {
    field
        .ident
        .to_owned()
        .map(|x| x.to_token_stream())
        .unwrap_or_else(|| syn::Index::from(field_idx).to_token_stream())
}

/// Returns the identifier of a type if it is a simple path type (i.e., not
/// qualified, no leading colon, no multiple segments), and `None` otherwise.
///
/// Used to identify field types that are type parameters.
fn get_ident(ty: &syn::Type) -> Option<&syn::Ident> {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    {
        if segments.len() == 1 {
            return Some(&segments[0].ident);
        }
    }

    None
}

/// Returns true if the given field carries `#[epserde(force_full)]`.
fn is_force_full(field: &syn::Field) -> bool {
    let mut found = false;
    for attr in &field.attrs {
        if !attr.meta.path().is_ident("epserde") {
            continue;
        }
        // Parse errors are intentionally swallowed; the per-field validator
        // runs the same walk with proper error propagation.
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("force_full") {
                found = true;
            }
            Ok(())
        });
    }
    found
}

/// Records into `out` every occurrence of one of `type_params` at a
/// variable position in `ty`, treating `PhantomData<…>` as a transparent
/// slot whose interior does not contribute.
fn collect_param_occurrences<'a>(
    ty: &syn::Type,
    type_params: &[&'a syn::Ident],
    out: &mut HashSet<&'a syn::Ident>,
    inside_phantom: bool,
) {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            if !inside_phantom
                && path.leading_colon.is_none()
                && path.segments.len() == 1
                && path.segments[0].arguments.is_empty()
            {
                let id = &path.segments[0].ident;
                if let Some(p) = type_params.iter().find(|p| **p == id) {
                    out.insert(*p);
                    return;
                }
            }
            for segment in &path.segments {
                let segment_is_phantom = segment.ident == "PhantomData";
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let descend_inside_phantom = inside_phantom || segment_is_phantom;
                    for arg in &ab.args {
                        match arg {
                            syn::GenericArgument::Type(t) => {
                                collect_param_occurrences(
                                    t,
                                    type_params,
                                    out,
                                    descend_inside_phantom,
                                );
                            }
                            syn::GenericArgument::AssocType(a) => {
                                collect_param_occurrences(
                                    &a.ty,
                                    type_params,
                                    out,
                                    descend_inside_phantom,
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        syn::Type::Tuple(t) => {
            for e in &t.elems {
                collect_param_occurrences(e, type_params, out, inside_phantom);
            }
        }
        syn::Type::Array(a) => collect_param_occurrences(&a.elem, type_params, out, inside_phantom),
        syn::Type::Slice(s) => collect_param_occurrences(&s.elem, type_params, out, inside_phantom),
        syn::Type::Paren(p) => collect_param_occurrences(&p.elem, type_params, out, inside_phantom),
        syn::Type::Group(g) => collect_param_occurrences(&g.elem, type_params, out, inside_phantom),
        _ => {}
    }
}

/// Returns true if `ty` contains an occurrence of any of `type_params` at
/// a variable position. Used to decide whether an unmarked field should
/// default to ε-deserialization (some occurrence present) or to full-copy
/// (no occurrences: nothing to substitute).
fn has_param_occurrence(ty: &syn::Type, type_params: &[&syn::Ident]) -> bool {
    let mut out: HashSet<&syn::Ident> = HashSet::new();
    collect_param_occurrences(ty, type_params, &mut out, false);
    !out.is_empty()
}

/// Records into `out` every type parameter that occurs as the direct element of
/// a literal `Vec<…>`, boxed/bare slice `[…]`, or array `[…; N]` anywhere within
/// `ty`. Such a parameter is forced to be deep-copy for ε-copy stability: were
/// it zero-copy, the containing sequence would ε-copy deserialize to a slice
/// reference, a type not expressible as the original sequence.
fn collect_seq_forced_deep_params<'a>(
    ty: &syn::Type,
    type_params: &[&'a syn::Ident],
    out: &mut HashSet<&'a syn::Ident>,
) {
    fn record_if_bare<'a>(
        ty: &syn::Type,
        type_params: &[&'a syn::Ident],
        out: &mut HashSet<&'a syn::Ident>,
    ) {
        if let syn::Type::Path(syn::TypePath { qself: None, path }) = ty {
            if path.leading_colon.is_none()
                && path.segments.len() == 1
                && path.segments[0].arguments.is_empty()
            {
                let id = &path.segments[0].ident;
                if let Some(p) = type_params.iter().find(|p| **p == id) {
                    out.insert(*p);
                }
            }
        }
    }

    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            for segment in &path.segments {
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let is_vec = segment.ident == "Vec";
                    for arg in &ab.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            if is_vec {
                                record_if_bare(t, type_params, out);
                            }
                            collect_seq_forced_deep_params(t, type_params, out);
                        }
                    }
                }
            }
        }
        syn::Type::Slice(s) => {
            record_if_bare(&s.elem, type_params, out);
            collect_seq_forced_deep_params(&s.elem, type_params, out);
        }
        syn::Type::Array(a) => {
            record_if_bare(&a.elem, type_params, out);
            collect_seq_forced_deep_params(&a.elem, type_params, out);
        }
        syn::Type::Tuple(t) => {
            for e in &t.elems {
                collect_seq_forced_deep_params(e, type_params, out);
            }
        }
        syn::Type::Reference(r) => collect_seq_forced_deep_params(&r.elem, type_params, out),
        syn::Type::Paren(p) => collect_seq_forced_deep_params(&p.elem, type_params, out),
        syn::Type::Group(g) => collect_seq_forced_deep_params(&g.elem, type_params, out),
        _ => {}
    }
}

/// Generates the per-field initializer used inside the derived
/// `_deser_eps_inner` body's struct literal.
///
/// For each field this is either a `_deser_full_inner` call, a
/// `_deser_eps_inner` call, a literal `PhantomData`, or a
/// `_deser_eps_inner_special` call for `PhantomDeserData`. The choice
/// between `_deser_full_inner` and `_deser_eps_inner` is taken by the
/// caller via `deser_full`; the `PhantomData` and `PhantomDeserData`
/// branches override that choice based on the field type.
///
/// The type of `field_name` is [`proc_macro2::TokenStream`] because it can be
/// either an identifier (for named fields) or an index (for unnamed fields).
fn gen_eps_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    deser_full: bool,
) -> proc_macro2::TokenStream {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments,
        },
    }) = field_type
    {
        // This is a pretty weak check, as a user could define its own
        // PhantomDeserData, but it should be good enough in practice.
        // The two checks below are mutually exclusive (a path has
        // exactly one last segment).
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: unsafe { <#field_type>::_deser_eps_inner_special(backend)? });
            }
            // PhantomData<...> is handled natively: emit a literal
            // PhantomData whose generic parameter is inferred from the
            // surrounding Self::DeserType<'a> struct literal.
            if segment.ident == "PhantomData" {
                return syn::parse_quote!(#field_name: ::core::marker::PhantomData);
            }
        }
    }

    if deser_full {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_full_inner(backend)? })
    } else {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_eps_inner(backend)? })
    }
}

/// Generates the `IS_ZERO_COPY` expression.
fn gen_is_zero_copy_expr(is_repr_c: bool, field_types: &[&syn::Type]) -> proc_macro2::TokenStream {
    if field_types.is_empty() {
        quote!(#is_repr_c)
    } else {
        quote!(#is_repr_c #(&& <#field_types>::IS_ZERO_COPY)*)
    }
}

/// Generates the fixed-point assertion injected at the top of
/// `_deser_eps_inner` when one or more type parameters appear in both an
/// unmarked field and a field marked `#[epserde(force_full)]`.
///
/// For each conflicting parameter the assertion requires the bound
/// `for<'a> <T as DeserInner>::DeserType<'a>: DeserFixedPoint<T>`. The blanket
/// impl `impl<T> DeserFixedPoint<T> for T` makes the bound trivially hold
/// when `DeserType<'a> = T` (the fixed-point condition the user can supply
/// through `bound(deser = ...)`); otherwise the impl does not apply and the
/// `#[diagnostic::on_unimplemented]` message on `DeserFixedPoint` surfaces an
/// actionable hint alongside rustc's slot-mismatch error.
///
/// Returns an empty token stream when there are no conflicts.
fn gen_fixed_point_check(conflict_params: &[&syn::Ident]) -> proc_macro2::TokenStream {
    if conflict_params.is_empty() {
        return quote!();
    }
    quote! {
        fn __epserde_fixed_point_check<__Outer, __Slot: ?Sized>()
        where
            __Slot: ::epserde::deser::DeserFixedPoint<__Outer>,
        {}
        #(
            __epserde_fixed_point_check::<
                #conflict_params,
                <#conflict_params as ::epserde::deser::DeserInner>::DeserType<'_>,
            >();
        )*
    }
}

/// Generates the ε-copy stability assertion emitted, as a standalone item, for
/// each type parameter that occurs as the direct element of a literal `Vec<…>`,
/// boxed slice, or array in an unmarked field.
///
/// The assertion requires `T: DeepCopyInSeq`; the blanket impl holds as soon as
/// the user bounds `T: DeepCopy`, so the check is silent for well-formed types
/// and surfaces the `#[diagnostic::on_unimplemented]` hint on `DeepCopyInSeq`
/// otherwise. It is emitted as a free generic function inside a `const _` block
/// (carrying the type's own generics and where clause) rather than inside the
/// (de)serialization bodies, so that its clean hint is reported before the raw
/// trait-resolution errors that the unbounded parameter triggers elsewhere.
///
/// Returns an empty token stream when there are no such parameters.
fn gen_seq_deep_check(
    seq_deep_idents: &[syn::Ident],
    generics_for_impl: &syn::ImplGenerics,
    where_clause: &syn::WhereClause,
) -> proc_macro2::TokenStream {
    if seq_deep_idents.is_empty() {
        return quote!();
    }
    quote! {
        const _: () = {
            fn __epserde_seq_deep_assert #generics_for_impl () #where_clause {
                fn __check<__SeqElem: ::epserde::deser::DeepCopyInSeq>() {}
                #(
                    __check::<#seq_deep_idents>();
                )*
            }
        };
    }
}

/// Pushes each parameter in `params` into `out`, re-spanned to `span`, so that
/// the stability assertion generated from `out` points at the field that forces
/// the parameter to be deep-copy.
fn push_seq_deep_idents(
    params: &HashSet<&syn::Ident>,
    span: proc_macro2::Span,
    out: &mut Vec<syn::Ident>,
) {
    for p in params {
        let mut id = (*p).clone();
        id.set_span(span);
        out.push(id);
    }
}

/// Generates the `MIGHT_BE_ZERO_COPY` expression.
fn gen_might_be_zero_copy_expr(
    is_repr_c: bool,
    field_types: &[&syn::Type],
) -> proc_macro2::TokenStream {
    if field_types.is_empty() {
        quote!(#is_repr_c)
    } else {
        quote!(#is_repr_c #(&& <#field_types>::MIGHT_BE_ZERO_COPY)*)
    }
}

/// Returns the identifiers of type and const parameters.
///
/// More in detail, returns a tuple containing:
///
/// - the identifiers of type and const parameters, in order of appearance (used
///   to generate associated (de)serialization type generics);
///
/// - the identifiers of type parameters as a set (used to identify fields whose
///   type is a type parameter);
///
/// - the identifiers of const parameters, also in order of appearance (used
///   to compute type hashes).
fn get_type_const_params(
    input: &DeriveInput,
) -> syn::Result<(Vec<&syn::Ident>, HashSet<&syn::Ident>, Vec<&syn::Ident>)> {
    let mut type_const_params = vec![];
    let mut type_params = HashSet::new();
    let mut const_params = vec![];

    for param in &input.generics.params {
        match param {
            syn::GenericParam::Type(t) => {
                type_const_params.push(&t.ident);
                type_params.insert(&t.ident);
            }
            syn::GenericParam::Const(c) => {
                type_const_params.push(&c.ident);
                const_params.push(&c.ident);
            }
            syn::GenericParam::Lifetime(l) => {
                return Err(syn::Error::new_spanned(
                    l,
                    "Lifetime generics are not supported",
                ));
            }
        };
    }

    Ok((type_const_params, type_params, const_params))
}

/// Parsed epserde attributes.
struct EpserdeAttrs {
    /// Whether the type has `#[repr(C)]`.
    is_repr_c: bool,
    /// Whether `#[epserde(zero_copy)]` or `#[epserde(zero_copy)]` was specified.
    is_zero_copy: bool,
    /// Whether `#[epserde(deep_copy)]` or `#[epserde_deep_copy]` was specified.
    is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl.
    deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl.
    ser_bounds: Vec<WherePredicate>,
    /// Whether old-style `#[epserde(zero_copy)]` was used.
    deprecated_zero_copy: bool,
    /// Whether old-style `#[epserde_deep_copy]` was used.
    deprecated_deep_copy: bool,
}

/// Parses epserde attributes from `#[epserde(...)]`, `#[epserde(zero_copy)]`,
/// and `#[epserde_deep_copy]`.
fn parse_epserde_attrs(input: &DeriveInput) -> syn::Result<EpserdeAttrs> {
    let is_repr_c = input.attrs.iter().any(|x| {
        x.meta.path().is_ident("repr") && x.meta.require_list().unwrap().tokens.to_string() == "C"
    });

    let mut is_zero_copy = false;
    let mut is_deep_copy = false;
    let mut deser_bounds = Vec::new();
    let mut ser_bounds = Vec::new();
    let mut deprecated_zero_copy = false;
    let mut deprecated_deep_copy = false;

    for attr in &input.attrs {
        if attr.meta.path().is_ident("epserde_zero_copy") {
            is_zero_copy = true;
            deprecated_zero_copy = true;
        } else if attr.meta.path().is_ident("epserde_deep_copy") {
            is_deep_copy = true;
            deprecated_deep_copy = true;
        } else if attr.meta.path().is_ident("epserde") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("zero_copy") {
                    is_zero_copy = true;
                    Ok(())
                } else if meta.path.is_ident("deep_copy") {
                    is_deep_copy = true;
                    Ok(())
                } else if meta.path.is_ident("bound") {
                    meta.parse_nested_meta(|inner| {
                        if inner.path.is_ident("deser") {
                            let value = inner.value()?;
                            let lit: syn::LitStr = value.parse()?;
                            let preds = lit.parse_with(
                                Punctuated::<WherePredicate, token::Comma>::parse_terminated,
                            )?;
                            deser_bounds.extend(preds);
                            Ok(())
                        } else if inner.path.is_ident("ser") {
                            let value = inner.value()?;
                            let lit: syn::LitStr = value.parse()?;
                            let preds = lit.parse_with(
                                Punctuated::<WherePredicate, token::Comma>::parse_terminated,
                            )?;
                            ser_bounds.extend(preds);
                            Ok(())
                        } else {
                            Err(inner.error("expected `deser` or `ser`"))
                        }
                    })
                } else {
                    Err(meta.error("expected \"zero_copy\", \"deep_copy\", or \"bound\""))
                }
            })?;
        }
    }

    if is_zero_copy && !is_repr_c {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "Type {} is declared as zero-copy, but it is not repr(C)",
                input.ident
            ),
        ));
    }
    if is_zero_copy && is_deep_copy {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "Type {} is declared as both zero-copy and deep-copy",
                input.ident
            ),
        ));
    }

    Ok(EpserdeAttrs {
        is_repr_c,
        is_zero_copy,
        is_deep_copy,
        deser_bounds,
        ser_bounds,
        deprecated_zero_copy,
        deprecated_deep_copy,
    })
}

/// Emits deprecation warnings for old-style `#[epserde(zero_copy)]` and
/// `#[epserde_deep_copy]` attributes during compilation.
fn emit_deprecation_warnings(attrs: &EpserdeAttrs, type_name: &syn::Ident) {
    if attrs.deprecated_zero_copy {
        eprintln!(
            "warning: use #[epserde(zero_copy)] instead of #[epserde_zero_copy] on type {type_name}"
        );
    }
    if attrs.deprecated_deep_copy {
        eprintln!(
            "warning: use #[epserde(deep_copy)] instead of #[epserde_deep_copy] on type {type_name}"
        );
    }
}

/// For each bounded type parameter that is the type of some field, bounds the
/// associated (de)serialization types with the same trait bounds of the type.
fn bound_ser_deser_types(
    derive_input: &DeriveInput,
    eps_params: &HashSet<&syn::Ident>,
    ser_where_clause: &mut WhereClause,
    deser_where_clause: &mut WhereClause,
) {
    // If there are bounded type parameters which are fields of the struct, we
    // need to impose the same bounds on the associated SerType/DeserType.
    for param in &derive_input.generics.params {
        if let syn::GenericParam::Type(t) = param {
            let ident = &t.ident;

            // We are just interested in type parameters that are types of
            // fields and that have trait bounds
            if !t.bounds.is_empty() && eps_params.contains(ident) {
                // The lifetime of the DeserType
                let mut lifetimes = Punctuated::new();
                lifetimes.push(GenericParam::Lifetime(LifetimeParam {
                    attrs: vec![],
                    lifetime: syn::Lifetime::new(
                        "'__epserde_desertype",
                        proc_macro2::Span::call_site(),
                    ),
                    colon_token: None,
                    bounds: Punctuated::new(),
                }));

                // Add the trait bounds of the type to the DeserType
                deser_where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: Some(BoundLifetimes {
                            for_token: token::For::default(),
                            lt_token: token::Lt::default(),
                            lifetimes,
                            gt_token: token::Gt::default(),
                        }),
                        bounded_ty: syn::parse_quote!(
                            ::epserde::deser::DeserType<'__epserde_desertype, #ident>
                        ),
                        colon_token: token::Colon::default(),
                        bounds: t.bounds.clone(),
                    }));

                // Add the trait bounds of the type to the SerType
                ser_where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: syn::parse_quote!(
                            ::epserde::ser::SerType<#ident>
                        ),
                        colon_token: token::Colon::default(),
                        bounds: t.bounds.clone(),
                    }));
            }
        }
    }
}

/// Adds to the given (de)serialization where clause a bound to `(De)SerInner`
/// for the given type.
///
/// In the case of zero-copy types, add also the other bounds on which
/// `ZeroCopy` depends; moreover, the bound to `SerInner` requires `SerType =
/// Self`.
fn add_ser_deser_trait_bounds(
    ty: &syn::Type,
    is_zero_copy: bool,
    ser_where_clause: &mut syn::WhereClause,
    deser_where_clause: &mut syn::WhereClause,
) {
    if is_zero_copy {
        // All fields of zero-copy types must be zero-copy
        ser_where_clause.predicates.push(syn::parse_quote!(
            #ty: ::epserde::traits::copy_type::ZeroCopy
        ));
    } else {
        ser_where_clause.predicates.push(syn::parse_quote!(
            #ty: ::epserde::ser::SerInner
        ));
    }

    // Note that we cannot impose DeserType<'_> = &Self in the zero-copy case
    // because of primitive types
    deser_where_clause.predicates.push(syn::parse_quote!(
        #ty: ::epserde::deser::DeserInner
    ));
}

/// Generates generics for the deserialization type by replacing ε-copy
/// type parameters with their associated deserialization type.
fn gen_generics_for_deser_type(
    ctx: &EpserdeContext,
    eps_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if eps_params.contains(ident) {
                quote!(::epserde::deser::DeserType<'__epserde_desertype, #ident>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates generics for the serialization type by replacing ε-copy
/// type parameters with their associated serialization type.
fn gen_generics_for_ser_type(
    ctx: &EpserdeContext,
    eps_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if eps_params.contains(ident) {
                quote!(::epserde::ser::SerType<#ident>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates where clauses for `SerInner` and `DeserInner`
/// implementations.
///
/// The where clauses bound all field types with the trait being implemented,
/// thus propagating recursively (de)serializability.
///
/// `full_deser_fields[i]` is `true` when field `i` is dispatched
/// full-copy (either because it carries `#[epserde(force_full)]` or
/// because its type contains no variable position to substitute). For
/// ε-deserialized fields the field-type bound is suppressed: it would
/// shadow the impl's `DeserType<'_>` projection (Rust issue #152409),
/// making the derived `_deser_eps_inner` body fail to type-check. The
/// per-parameter `T: SerInner`/`T: DeserInner` bounds emitted by the
/// caller are sufficient for Rust to resolve impls of wrappers whose
/// `DeserType<'_>` is uniform across kinds (`Box<T>`, `Rc<T>`,
/// `Arc<T>`, `Option<T>`, `Range<T>`, tuples). For wrappers whose
/// resolution depends on `T`'s kind (`Vec<T>`, `Box<[T]>`, `[T; N]`,
/// `String`), the user must additionally bound `T: ZeroCopy` or
/// `T: DeepCopy`; the derive does not emit those bounds because the
/// choice is not derivable from the field type alone.
fn gen_ser_deser_where_clauses(
    field_types: &[&syn::Type],
    is_zero_copy: bool,
    full_deser_fields: &[bool],
) -> (WhereClause, WhereClause) {
    debug_assert_eq!(field_types.len(), full_deser_fields.len());
    let mut ser_where_clause = empty_where_clause();
    let mut deser_where_clause = empty_where_clause();

    // Add trait bounds for all field types
    for (field_type, &is_full) in field_types.iter().zip(full_deser_fields) {
        if !is_zero_copy && !is_full {
            continue;
        }
        add_ser_deser_trait_bounds(
            field_type,
            is_zero_copy,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );
    }

    (ser_where_clause, deser_where_clause)
}

/// Generates the where clauses for `TypeHash`, `AlignHash`, and `AlignTo`.
///
/// The where clauses bound with the trait being implemented; the bound is
/// applied to the field types for zero-copy types, and to the associated
/// serialization types of field types for deep-copy types,
fn gen_type_info_where_clauses(
    base_clause: &WhereClause,
    is_zero_copy: bool,
    field_types: &[&syn::Type],
) -> (WhereClause, WhereClause, WhereClause) {
    // Generates one of the clauses by adding the given trait bound for all
    // types of fields.
    let gen_type_info_where_clause = |trait_bound: Punctuated<TypeParamBound, Plus>| {
        let mut where_clause = base_clause.clone();
        for &field_type in field_types {
            if is_zero_copy {
                // In zero-copy types bounds are propagated on the type
                // themselves, as the serialization types are always Self
                where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: field_type.clone(),
                        colon_token: token::Colon::default(),
                        bounds: trait_bound.clone(),
                    }));
            } else {
                // In deep-copy types bounds are propagated on the
                // associated serialization types
                where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: field_type.clone(),
                        colon_token: token::Colon::default(),
                        bounds: syn::parse_quote!(::epserde::ser::SerInner<SerType: #trait_bound>),
                    }));
            }
        }

        where_clause
    };

    let mut bound_type_hash = Punctuated::new();
    bound_type_hash.push(syn::parse_quote!(::epserde::traits::TypeHash));
    let type_hash = gen_type_info_where_clause(bound_type_hash);

    let mut bound_align_hash = Punctuated::new();
    bound_align_hash.push(syn::parse_quote!(::epserde::traits::AlignHash));
    let align_hash = gen_type_info_where_clause(bound_align_hash);

    let mut bound_align_to = Punctuated::new();
    bound_align_to.push(syn::parse_quote!(::epserde::traits::AlignTo));
    let align_to = gen_type_info_where_clause(bound_align_to);

    (type_hash, align_hash, align_to)
}

/// Context structure for the [`Epserde`] derive macro.
struct EpserdeContext<'a> {
    /// The original derive input.
    derive_input: &'a DeriveInput,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_params: Vec<&'a syn::Ident>,
    /// Identifiers of type parameters as a set.
    type_params: HashSet<&'a syn::Ident>,
    /// Generics for the `impl` clause as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics_for_impl: ImplGenerics<'a>,
    /// Generics for the type as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics_for_type: TypeGenerics<'a>,
    /// The where clause for the type being derived.
    where_clause: &'a WhereClause,
    /// Whether the type has `#[repr(C)]`
    is_repr_c: bool,
    /// Whether the type has `#[epserde(zero_copy)]`
    is_zero_copy: bool,
    /// Whether the type has `#[epserde(deep_copy)]`
    is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl from
    /// `#[epserde(bound(deser = "..."))]`.
    deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl from
    /// `#[epserde(bound(ser = "..."))]`.
    ser_bounds: Vec<WherePredicate>,
}

/// [`Epserde`] derive code for struct types.
fn gen_epserde_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];
    let mut eps_params = HashSet::new();
    let mut full_params = HashSet::new();
    // Parameters forced to be deep-copy because they occur as a sequence
    // element in an ε-copy field, each re-spanned to the field that forces it,
    // so that the stability diagnostic points at the offending field.
    let mut seq_deep_idents: Vec<syn::Ident> = vec![];
    let mut full_deser_fields = vec![];

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;
        let force_full = is_force_full(field);
        let has_var_pos = has_param_occurrence(field_type, &ctx.type_const_params);
        // A field is deserialized full-copy when explicitly marked with
        // #[epserde(force_full)] or when its type has no variable position
        // to substitute.
        let deser_full = force_full || !has_var_pos;

        if force_full && (ctx.is_zero_copy || !has_var_pos) {
            let type_name = &ctx.derive_input.ident;
            eprintln!(
                "warning: #[epserde(force_full)] on field {field_name} of type {type_name} has no effect; consider removing the marker"
            );
        }

        // ε-copy parameters: occurrences at a variable position in
        // an unmarked field. Full-copy parameters: occurrences at a
        // variable position in a marked field.
        if force_full {
            collect_param_occurrences(field_type, &ctx.type_const_params, &mut full_params, false);
        } else {
            collect_param_occurrences(field_type, &ctx.type_const_params, &mut eps_params, false);
            let mut field_seq_deep = HashSet::new();
            collect_seq_forced_deep_params(field_type, &ctx.type_const_params, &mut field_seq_deep);
            push_seq_deep_idents(&field_seq_deep, field_type.span(), &mut seq_deep_idents);
        }

        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            deser_full,
        ));

        field_names.push(field_name);
        field_types.push(field_type);
        full_deser_fields.push(deser_full);
    }

    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &eps_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &eps_params);
    // A type parameter that is both ε-copy and full-copy produces
    // a slot mismatch in the generated _deser_eps_inner: one occurrence
    // becomes <T as DeserInner>::DeserType<'_>, the other stays as T. The
    // user can resolve the conflict with a bound that forces DeserType<'_>
    // = T (automatic for ZeroCopy types). The assertion below requests
    // DeserFixedPoint for each conflicting parameter so that, when the
    // bound is missing, the on_unimplemented message points at the fix
    // instead of leaving the user with rustc's raw slot mismatch.
    let conflict_params: Vec<&syn::Ident> =
        eps_params.intersection(&full_params).copied().collect();
    let fixed_point_check = gen_fixed_point_check(&conflict_params);
    let seq_deep_check =
        gen_seq_deep_check(&seq_deep_idents, &ctx.generics_for_impl, ctx.where_clause);
    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &field_types);
    let might_be_zero_copy_expr = gen_might_be_zero_copy_expr(ctx.is_repr_c, &field_types);
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(&field_types, ctx.is_zero_copy, &full_deser_fields);

    // Add user-specified bounds from #[epserde(bound(...))]
    ser_where_clause
        .predicates
        .extend(ctx.ser_bounds.iter().cloned());
    deser_where_clause
        .predicates
        .extend(ctx.deser_bounds.iter().cloned());

    // For every ε-copy parameter, emit T: SerInner / T: DeserInner.
    // The field-type bound was skipped above for ε-deserialized fields;
    // these per-parameter bounds let Rust resolve kind-uniform wrapper
    // impls (Box, Rc, Arc, Option, Range, tuples). For wrappers whose
    // resolution depends on T's kind (Vec, Box<[…]>, [T; N], String),
    // the user must additionally bound T: ZeroCopy or T: DeepCopy.
    if !ctx.is_zero_copy {
        for ident in &eps_params {
            ser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::ser::SerInner
            ));
            deser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::deser::DeserInner
            ));
        }
    }

    let name = &ctx.derive_input.ident;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;
    let where_clause = &ctx.where_clause;

    if ctx.is_zero_copy {
        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.
        quote! {
            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Zero;
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = Self;
                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;
                const MIGHT_BE_ZERO_COPY: bool = #might_be_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    ::epserde::ser::helpers::ser_zero(backend, self)
                }
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause
            {
                #[inline(always)]
                fn __check_covariance<'__long: '__short, '__short>(
                    proof: ::epserde::deser::CovariantProof<Self::DeserType<'__long>>,
                ) -> ::epserde::deser::CovariantProof<Self::DeserType<'__short>> {
                    proof
                }

                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    unsafe { ::epserde::deser::helpers::deser_full_zero::<Self>(backend) }
                }

                type DeserType<'__epserde_desertype> = &'__epserde_desertype Self;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    unsafe { ::epserde::deser::helpers::deser_eps_zero::<Self>(backend) }
                }
            }
        }
    } else {
        bound_ser_deser_types(
            ctx.derive_input,
            &eps_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );

        let is_deep_copy = ctx.is_deep_copy;
        let name_str = name.to_string();

        quote! {
            #seq_deep_check

            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Deep;
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = #name<#(#generics_for_ser_type,)*>;
                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;
                const MIGHT_BE_ZERO_COPY: bool = #might_be_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    // Check whether the type could be zero-copy but it is not
                    // declared as such, and the attribute `epserde_deep_copy`
                    // is missing
                    const { assert!(!(! #is_deep_copy #(&& <#field_types>::MIGHT_BE_ZERO_COPY)*), concat!("Structure ", #name_str, " could be zero-copy, but it has not been declared as such; use either #[epserde(zero_copy)] or #[epserde(deep_copy)] to silence this error")); }

                    #(
                        unsafe { WriteWithNames::write(backend, stringify!(#field_names), &self.#field_names)?; }
                    )*
                    Ok(())
                }
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause {
                #[allow(clippy::useless_transmute)]
                #[inline(always)]
                fn __check_covariance<'__long: '__short, '__short>(
                    proof: ::epserde::deser::CovariantProof<Self::DeserType<'__long>>,
                ) -> ::epserde::deser::CovariantProof<Self::DeserType<'__short>> {
                    // SAFETY: structs are covariant in all their fields, and each
                    // field's DeserType is covariant (enforced by its own
                    // __check_covariance, which is called below).
                    #(
                        ::epserde::deser::__check_type_covariance::<#field_types>();
                    )*
                    unsafe { ::core::mem::transmute(proof) }
                }

                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserInner;

                    Ok(#name{
                        #( #field_names: unsafe { <#field_types as DeserInner>::_deser_full_inner(backend)? }, )*
                    })
                }

                type DeserType<'__epserde_desertype> = #name<#(#generics_for_deser_type,)*>;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserInner;

                    #fixed_point_check

                    Ok(#name{
                        #( #method_calls, )*
                    })
                }
            }
        }
    }
}

/// [`Epserde`] derive code for enum types.
fn gen_epserde_enum_impl(ctx: &EpserdeContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut variant_ids = vec![];
    // For each variant, a match arm as a TokenStream
    let mut variant_arm = vec![];
    // For each variant, serialization code
    let mut variant_ser = vec![];
    // For each variant, full-copy deserialization code
    let mut variant_full_des = vec![];
    // For each variant, ε-copy deserialization code
    let mut variant_eps_des = vec![];
    // ε-copy parameters: occurrences at a variable position in
    // some unmarked field of some variant. Full-copy parameters:
    // occurrences at a variable position in some marked field of some
    // variant.
    let mut all_eps_params = HashSet::new();
    let mut all_full_params = HashSet::new();
    // Parameters forced to be deep-copy because they occur as a sequence
    // element in an ε-copy field, each re-spanned to the offending field.
    let mut seq_deep_idents: Vec<syn::Ident> = vec![];
    // All field types for all variants
    let mut all_fields_types = vec![];
    // Whether each entry in all_fields_types is deserialized full-copy.
    let mut all_full_deser_fields = vec![];

    for (variant_id, variant) in e.variants.iter().enumerate() {
        let ident = &variant.ident;
        variant_ids.push(ident);

        match &variant.fields {
            syn::Fields::Unit => {
                variant_arm.push(quote! { #ident });
                variant_ser.push(quote! {{
                    WriteWithNames::write(backend, "tag", &#variant_id)?;
                }});
                variant_full_des.push(quote! {});
                variant_eps_des.push(quote! {});
            }
            syn::Fields::Named(fields) => {
                // The code in this arm is almost identical to the code for the
                // next one, except for the handling of field names.
                let mut field_names = vec![];
                let mut field_types = vec![];
                let mut method_calls = vec![];

                for field in &fields.named {
                    // It's a named field
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = &field.ty;
                    let force_full = is_force_full(field);
                    let has_var_pos = has_param_occurrence(field_type, &ctx.type_const_params);
                    let deser_full = force_full || !has_var_pos;

                    if force_full && (ctx.is_zero_copy || !has_var_pos) {
                        let type_name = &ctx.derive_input.ident;
                        eprintln!(
                            "warning: #[epserde(force_full)] on field {ident}::{field_name} of type {type_name} has no effect; consider removing the marker"
                        );
                    }

                    if force_full {
                        collect_param_occurrences(
                            field_type,
                            &ctx.type_const_params,
                            &mut all_full_params,
                            false,
                        );
                    } else {
                        collect_param_occurrences(
                            field_type,
                            &ctx.type_const_params,
                            &mut all_eps_params,
                            false,
                        );
                        let mut field_seq_deep = HashSet::new();
                        collect_seq_forced_deep_params(
                            field_type,
                            &ctx.type_const_params,
                            &mut field_seq_deep,
                        );
                        push_seq_deep_idents(
                            &field_seq_deep,
                            field_type.span(),
                            &mut seq_deep_idents,
                        );
                    }

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        deser_full,
                    ));
                    field_names.push(quote! { #field_name });
                    field_types.push(field_type);
                    all_full_deser_fields.push(deser_full);
                }

                all_fields_types.extend(&field_types);

                variant_arm.push(quote! {
                    #ident{ #( #field_names, )* }
                });

                variant_ser.push(quote! {
                    WriteWithNames::write(backend, "tag", &#variant_id)?;
                    #(
                        WriteWithNames::write(backend, stringify!(#field_names), #field_names)?;
                    )*
                });

                variant_full_des.push(quote! {
                    #(
                        #field_names: unsafe { <#field_types as DeserInner>::_deser_full_inner(backend)? },
                    )*
                });

                variant_eps_des.push(quote! {
                    #(
                        #method_calls,
                    )*
                });
            }
            syn::Fields::Unnamed(fields) => {
                let mut field_indices = vec![];
                let mut field_types = vec![];
                // Names of the form v0, v1, ... used in the match arm
                let mut field_names_in_arm = vec![];
                let mut method_calls: Vec<proc_macro2::TokenStream> = vec![];

                for (field_idx, field) in fields.unnamed.iter().enumerate() {
                    let field_name = syn::Index::from(field_idx);
                    let field_type = &field.ty;
                    let force_full = is_force_full(field);
                    let has_var_pos = has_param_occurrence(field_type, &ctx.type_const_params);
                    let deser_full = force_full || !has_var_pos;

                    if force_full && (ctx.is_zero_copy || !has_var_pos) {
                        let type_name = &ctx.derive_input.ident;
                        let idx = syn::Index::from(field_idx);
                        eprintln!(
                            "warning: #[epserde(force_full)] on field {ident}::{idx_index} of type {type_name} has no effect; consider removing the marker",
                            idx_index = idx.index,
                        );
                    }

                    if force_full {
                        collect_param_occurrences(
                            field_type,
                            &ctx.type_const_params,
                            &mut all_full_params,
                            false,
                        );
                    } else {
                        collect_param_occurrences(
                            field_type,
                            &ctx.type_const_params,
                            &mut all_eps_params,
                            false,
                        );
                        let mut field_seq_deep = HashSet::new();
                        collect_seq_forced_deep_params(
                            field_type,
                            &ctx.type_const_params,
                            &mut field_seq_deep,
                        );
                        push_seq_deep_idents(
                            &field_seq_deep,
                            field_type.span(),
                            &mut seq_deep_idents,
                        );
                    }

                    field_indices.push(
                        syn::Ident::new(&format!("v{}", field_idx), proc_macro2::Span::call_site())
                            .to_token_stream(),
                    );

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        deser_full,
                    ));
                    field_types.push(field_type);
                    field_names_in_arm.push(field_name);
                    all_full_deser_fields.push(deser_full);
                }

                all_fields_types.extend(&field_types);

                variant_arm.push(quote! {
                    #ident( #( #field_indices, )* )
                });

                variant_ser.push(quote! {
                    WriteWithNames::write(backend, "tag", &#variant_id)?;
                    #(
                        unsafe { WriteWithNames::write(backend, stringify!(#field_indices), #field_indices)? };
                    )*
                });

                variant_full_des.push(quote! {
                    #(
                        #field_names_in_arm : unsafe { <#field_types as DeserInner>::_deser_full_inner(backend)? },
                    )*
                });

                variant_eps_des.push(quote! {
                    #(
                        #method_calls,
                    )*
                });
            }
        }
    }

    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &all_eps_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &all_eps_params);
    // See the struct branch for the rationale: a parameter that appears in
    // both unmarked and force_full-marked fields needs DeserType<'_> = T to
    // make the generated body type-check.
    let conflict_params: Vec<&syn::Ident> = all_eps_params
        .intersection(&all_full_params)
        .copied()
        .collect();
    let fixed_point_check = gen_fixed_point_check(&conflict_params);
    let seq_deep_check =
        gen_seq_deep_check(&seq_deep_idents, &ctx.generics_for_impl, ctx.where_clause);
    let tag = (0..variant_arm.len()).collect::<Vec<_>>();

    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &all_fields_types);
    let might_be_zero_copy_expr = gen_might_be_zero_copy_expr(ctx.is_repr_c, &all_fields_types);
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(&all_fields_types, ctx.is_zero_copy, &all_full_deser_fields);

    // Add user-specified bounds from #[epserde(bound(...))]
    ser_where_clause
        .predicates
        .extend(ctx.ser_bounds.iter().cloned());
    deser_where_clause
        .predicates
        .extend(ctx.deser_bounds.iter().cloned());

    // For every ε-copy parameter, emit T: SerInner / T: DeserInner.
    // The field-type bound was skipped above for ε-deserialized fields;
    // these per-parameter bounds let Rust resolve kind-uniform wrapper
    // impls (Box, Rc, Arc, Option, Range, tuples). For wrappers whose
    // resolution depends on T's kind (Vec, Box<[…]>, [T; N], String),
    // the user must additionally bound T: ZeroCopy or T: DeepCopy.
    if !ctx.is_zero_copy {
        for ident in &all_eps_params {
            ser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::ser::SerInner
            ));
            deser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::deser::DeserInner
            ));
        }
    }

    let name = &ctx.derive_input.ident;
    let is_deep_copy = ctx.is_deep_copy;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;
    let where_clause = &ctx.where_clause;

    if ctx.is_zero_copy {
        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.
        quote! {
            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Zero;
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = Self;

                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;
                const MIGHT_BE_ZERO_COPY: bool = #might_be_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    unsafe { ::epserde::ser::helpers::ser_zero(backend, self) }
                }
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause {
                #[inline(always)]
                fn __check_covariance<'__long: '__short, '__short>(
                    proof: ::epserde::deser::CovariantProof<Self::DeserType<'__long>>,
                ) -> ::epserde::deser::CovariantProof<Self::DeserType<'__short>> {
                    proof
                }

                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    unsafe { ::epserde::deser::helpers::deser_full_zero::<Self>(backend) }
                }

                type DeserType<'__epserde_desertype> = &'__epserde_desertype Self;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    unsafe { ::epserde::deser::helpers::deser_eps_zero::<Self>(backend) }
                }
            }
        }
    } else {
        bound_ser_deser_types(
            ctx.derive_input,
            &all_eps_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );

        let name_str = name.to_string();

        quote! {
            #seq_deep_check

            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Deep;
            }
            #[automatically_derived]

            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = #name<#(#generics_for_ser_type,)*>;

                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;
                const MIGHT_BE_ZERO_COPY: bool = #might_be_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    // Check whether the type could be zero-copy but it is not
                    // declared as such, and the attribute `epserde_deep_copy`
                    // is missing
                    const { assert!(!(! #is_deep_copy #(&& <#all_fields_types>::MIGHT_BE_ZERO_COPY)*), concat!("Enum ", #name_str, " could be zero-copy, but it has not been declared as such; use either #[epserde(zero_copy)] or #[epserde(deep_copy)] to silence this error")); }

                    match self {
                        #(
                           Self::#variant_arm => { #variant_ser }
                        )*
                    }
                    Ok(())
                }
            }
            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause {
                #[allow(clippy::useless_transmute)]
                #[inline(always)]
                fn __check_covariance<'__long: '__short, '__short>(
                    proof: ::epserde::deser::CovariantProof<Self::DeserType<'__long>>,
                ) -> ::epserde::deser::CovariantProof<Self::DeserType<'__short>> {
                    // SAFETY: enums are covariant in all their fields, and each
                    // field's DeserType is covariant (enforced by its own
                    // __check_covariance, which is called below).
                    #(
                        ::epserde::deser::__check_type_covariance::<#all_fields_types>();
                    )*
                    unsafe { ::core::mem::transmute(proof) }
                }

                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserInner;
                    use ::epserde::deser::Error;

                    match unsafe { <usize as DeserInner>::_deser_full_inner(backend)? } {
                        #(
                            #tag => Ok(Self::#variant_ids{ #variant_full_des }),
                        )*
                        tag => Err(Error::InvalidTag(tag)),
                    }
                }

                type DeserType<'__epserde_desertype> = #name<#(#generics_for_deser_type,)*>;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserInner;
                    use ::epserde::deser::Error;

                    #fixed_point_check

                    match unsafe { <usize as DeserInner>::_deser_full_inner(backend)? } {
                        #(
                            #tag => Ok(Self::DeserType::<'_>::#variant_ids{ #variant_eps_des }),
                        )*
                        tag => Err(Error::InvalidTag(tag)),
                    }
                }
            }
        }
    }
}

/// Generates an [ε-serde](Epserde) implementation for custom types.
///
/// It generates implementations for the traits `CopyType`, `AlignTo`,
/// `TypeHash`, `AlignHash`, `SerInner`, and `DeserInner`.
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
/// zero-copy, but has no attribute, a warning will be issued every time you
/// serialize an instance of the type. The warning can be silenced adding the
/// explicit attribute `#[epserde(deep_copy)]`.
///
/// You can specify additional where-clause bounds for the generated
/// (de)serialization implementations using `#[epserde(bound(deser = "...", ser
/// = "..."))]`. This is useful when a field's type involves an associated type
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
/// # `force_full` attribute
///
/// A **field-level** marker (no arguments) that pins a field to full-copy
/// deserialization and keeps its type verbatim in `DeserType<'_>`.
///
/// By default every field of a deep-copy type whose type contains a
/// type-parameter occurrence at a variable position is deserialized via
/// the ε-copy path, and that parameter is ε-copy: in
/// `Self::DeserType<'a>` it is substituted with `<T as DeserInner>::
/// DeserType<'a>`. Occurrences nested inside `PhantomData<…>` are
/// transparent and do not count. Fields whose type contains no variable
/// position default to full-copy: there is nothing to substitute.
///
/// `#[epserde(force_full)]` opts a single field out of the default:
///
/// - the field is deserialized via `_deser_full_inner` rather than
///   `_deser_eps_inner`;
/// - its type is preserved verbatim in `Self::DeserType<'a>` (no
///   substitution inside it);
/// - its occurrences do not contribute to the ε-copy-parameter set.
///
/// Typical use: a field whose type is `Vec<T>` but the surrounding struct
/// is to be deserialized full-copy, or a wrapper whose `DeserType<'_>`
/// cannot follow the uniform-substitution contract that ε-deserialization
/// requires.
///
/// The marker takes no arguments and affects only deserialization.
/// It is rejected if it appears anywhere inside a type marked
/// `#[epserde(zero_copy)]`: zero-copy structs are (de)serialized as a
/// sequence of raw bytes with no field-level choice between
/// `_deser_full_inner` and `_deser_eps_inner`, so the marker has no
/// operational meaning there. On a deep-copy field whose type contains
/// no variable position the marker is a silent no-op: the field is
/// already deserialized full-copy by default, since there is nothing
/// to substitute.
///
/// Example:
///
/// ```ignore
/// #[derive(Epserde)]
/// struct Outer<T> {
///     #[epserde(force_full)]
///     data: Vec<T>,  // stays as Vec<T> in DeserType<'_>, full-copy
/// }
/// ```
#[proc_macro_derive(Epserde, attributes(epserde_zero_copy, epserde_deep_copy, epserde))]
pub fn epserde_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // This part is in common with type_info_derive
    let mut derive_input = parse_macro_input!(input as DeriveInput);

    if let Some(where_clause) = &derive_input.generics.where_clause {
        return syn::Error::new_spanned(
            where_clause,
            "The derive macros do not support where clauses on the original type",
        )
        .to_compile_error()
        .into();
    }

    derive_input.generics.make_where_clause();
    let (generics_for_impl, generics_for_type, where_clause) =
        derive_input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let attrs = match parse_epserde_attrs(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let (type_const_params, type_params, const_params) = match get_type_const_params(&derive_input)
    {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    emit_deprecation_warnings(&attrs, &derive_input.ident);

    // Validate the per-field force_full marker.
    let validate_field = |field: &syn::Field| -> Result<(), syn::Error> {
        let is_phantom_deser_data = matches!(
            &field.ty,
            syn::Type::Path(syn::TypePath { qself: None, path })
                if path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "PhantomDeserData")
        );
        for attr in &field.attrs {
            if !attr.meta.path().is_ident("epserde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("force_full") {
                    if meta.input.peek(syn::token::Paren) {
                        return Err(meta.error(
                            "\"force_full\" is a field-level marker and takes no arguments; \
                             use #[epserde(force_full)]",
                        ));
                    }
                    if attrs.is_zero_copy {
                        return Err(meta.error(
                            "\"force_full\" cannot appear inside a zero-copy type",
                        ));
                    }
                    if is_phantom_deser_data {
                        return Err(meta.error(
                            "\"force_full\" has no operational effect on a PhantomDeserData<T> field; \
                             remove the marker, or migrate to PhantomData<T>",
                        ));
                    }
                }
                Ok(())
            })?;
        }
        Ok(())
    };

    let validate_fields = |fields: &syn::Fields| -> Result<(), syn::Error> {
        for field in fields {
            validate_field(field)?;
        }
        Ok(())
    };

    if let Err(e) = match &derive_input.data {
        Data::Struct(s) => validate_fields(&s.fields),
        Data::Enum(e) => e
            .variants
            .iter()
            .try_for_each(|v| validate_fields(&v.fields)),
        Data::Union(_) => Ok(()),
    } {
        return e.to_compile_error().into();
    }

    let ctx = EpserdeContext {
        derive_input: &derive_input,
        type_const_params,
        type_params,
        generics_for_impl,
        generics_for_type,
        where_clause,
        is_repr_c: attrs.is_repr_c,
        is_zero_copy: attrs.is_zero_copy,
        is_deep_copy: attrs.is_deep_copy,
        deser_bounds: attrs.deser_bounds,
        ser_bounds: attrs.ser_bounds,
    };

    let mut out: proc_macro::TokenStream = match &derive_input.data {
        Data::Struct(s) => gen_epserde_struct_impl(&ctx, s),
        Data::Enum(e) => gen_epserde_enum_impl(&ctx, e),
        Data::Union(_) => {
            return syn::Error::new_spanned(&derive_input.ident, "Union types are not supported")
                .to_compile_error()
                .into();
        }
    }
    .into();

    // Automatically derive type info
    out.extend(_type_info_derive(
        &derive_input,
        ctx.type_params,
        const_params,
        ctx.generics_for_impl,
        ctx.generics_for_type,
        ctx.where_clause,
        ctx.is_zero_copy,
    ));

    out
}

//
// `TypeInfo` derive macro implementation
//

/// Context structure for the [`TypeInfo`] derive macro
struct TypeInfoContext<'a> {
    /// The name of the type
    name: &'a syn::Ident,
    /// Identifiers of type parameters as a set.
    type_params: HashSet<&'a syn::Ident>,
    /// Identifiers of const parameters, in order of appearance.
    const_params: Vec<&'a syn::Ident>,
    /// Generics for the `impl` clause as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics_for_impl: ImplGenerics<'a>,
    /// Generics for the type as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics_for_type: TypeGenerics<'a>,
    /// The where clause for the type being derived.
    where_clause: &'a WhereClause,
    /// Whether the type is zero-copy
    is_zero_copy: bool,
    /// `repr` attributes
    repr_attrs: Vec<String>,
}

/// Generates the `TypeHash` implementation body.
fn gen_type_hash_body(
    ctx: &TypeInfoContext,
    field_hashes: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let copy_type = if ctx.is_zero_copy {
        "ZeroCopy"
    } else {
        "DeepCopy"
    };
    let name = &ctx.name;
    let const_params = &ctx.const_params;

    quote! {
        use ::core::hash::Hash;
        use ::epserde::traits::TypeHash;
        use ::epserde::ser::SerType;

        // Hash in copy type
        Hash::hash(#copy_type, hasher);

        // Hash in the values of const parameters
        #(
            Hash::hash(&#const_params, hasher);
        )*

        // Hash in the identifiers of const parameters
        #(
            Hash::hash(stringify!(#const_params), hasher);
        )*

        // Hash in the struct name.
        Hash::hash(stringify!(#name), hasher);

        // Hash in first all field names and then all field types
        #(
            #field_hashes
        )*
    }
}

/// Generates the `AlignHash` implementation body for struct types.
fn gen_struct_align_hash_body(
    ctx: &TypeInfoContext,
    fields_types: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let repr_attrs = &ctx.repr_attrs;
    if ctx.is_zero_copy {
        quote! {
            use ::core::hash::Hash;
            use ::epserde::traits::AlignHash;
            use ::epserde::ser::SerType;

            // Hash in size, as padding is given by AlignTo.
            // and it is independent of the architecture.
            Hash::hash(&::core::mem::size_of::<Self>(), hasher);

            // Hash in representation data.
            #(
                Hash::hash(#repr_attrs, hasher);
            )*

            // Hash in all fields
            #(
                <#fields_types as AlignHash>::align_hash(
                    hasher,
                    offset_of,
                );
            )*
        }
    } else {
        quote! {
            use ::epserde::traits::AlignHash;
            use ::epserde::ser::SerType;

            // Hash in all fields starting at offset 0
            #(
                <#fields_types as AlignHash>::align_hash(hasher, &mut 0);
            )*
        }
    }
}

/// Generates the `AlignHash` implementation body for enum types.
fn gen_enum_align_hash_body(
    ctx: &TypeInfoContext,
    all_align_hashes: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let repr_attrs = &ctx.repr_attrs;
    if ctx.is_zero_copy {
        quote! {
            use ::core::hash::Hash;
            use ::epserde::traits::AlignHash;
            use ::epserde::ser::SerType;

            // Hash in size, as padding is given by AlignTo,
            // and it is independent of the architecture
            Hash::hash(&::core::mem::size_of::<Self>(), hasher);

            // Hash in representation data.
            #(
                Hash::hash(#repr_attrs, hasher);
            )*

            // Hash in all fields
            let old_offset_of = *offset_of;
            #(
                *offset_of = old_offset_of;
                #all_align_hashes
            )*
        }
    } else {
        // Hash in all fields starting at offset 0
        quote! {
            use ::epserde::traits::AlignHash;
            use ::epserde::ser::SerType;

            #(
                *offset_of = 0;
                #all_align_hashes
            )*
        }
    }
}

/// Generates the `AlignTo` implementation body for struct types.
fn gen_struct_align_to_body(fields_types: &[&syn::Type]) -> proc_macro2::TokenStream {
    quote! {
        use ::epserde::traits::AlignTo;

        let mut align_to = ::core::mem::align_of::<Self>();

        #(
            if align_to < <#fields_types as AlignTo>::align_to() {
                align_to = <#fields_types as AlignTo>::align_to();
            }
        )*
        align_to
    }
}

/// Generates the implementations for `TypeHash`, `AlignHash`, and
/// optionally `AlignTo`.
fn gen_type_info_traits(
    ctx: TypeInfoContext,
    type_hash_where_clause: syn::WhereClause,
    align_hash_where_clause: syn::WhereClause,
    align_to_where_clause: syn::WhereClause,
    type_hash_body: proc_macro2::TokenStream,
    align_hash_body: proc_macro2::TokenStream,
    align_to_body: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let name = &ctx.name;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;

    let align_to_impl = if let Some(align_to_body) = align_to_body {
        quote! {
            #[automatically_derived]
            impl #generics_for_impl ::epserde::traits::AlignTo for #name #generics_for_type #align_to_where_clause {
                fn align_to() -> usize {
                    #align_to_body
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[automatically_derived]
        impl #generics_for_impl ::epserde::traits::TypeHash for #name #generics_for_type #type_hash_where_clause {
            fn type_hash(hasher: &mut impl ::core::hash::Hasher) {
                #type_hash_body
            }
        }

        #[automatically_derived]
        impl #generics_for_impl ::epserde::traits::AlignHash for #name #generics_for_type #align_hash_where_clause {
            fn align_hash(
                hasher: &mut impl ::core::hash::Hasher,
                offset_of: &mut usize,
            ) {
                #align_hash_body
            }
        }

        #align_to_impl
    }
}

/// [`TypeInfo`] derive code for struct types.
fn gen_struct_type_info_impl(
    ctx: TypeInfoContext,
    s: &syn::DataStruct,
) -> proc_macro2::TokenStream {
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut field_types_ts = vec![];

    // Extract field information
    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_type = &field.ty;
        field_names.push(get_field_name(field, field_idx));
        field_types.push(field_type);

        field_types_ts.push(quote! { SerType<#field_type> });
    }

    let (type_hash_where_clause, align_hash_where_clause, align_to_where_clause) =
        gen_type_info_where_clauses(ctx.where_clause, ctx.is_zero_copy, &field_types);

    // Generate field hashes for TypeHash
    let mut field_hashes: Vec<_> = field_names
        .iter()
        .map(|name| quote! { Hash::hash(stringify!(#name), hasher); })
        .collect();

    field_hashes.extend(field_types_ts.iter().map(|field_type_ts| {
        quote! { <#field_type_ts as TypeHash>::type_hash(hasher); }
    }));

    // Generate implementation bodies
    let type_hash_body = gen_type_hash_body(&ctx, &field_hashes);
    let align_hash_body = gen_struct_align_hash_body(&ctx, &field_types_ts);
    let align_to_body = if ctx.is_zero_copy {
        Some(gen_struct_align_to_body(&field_types))
    } else {
        None
    };

    gen_type_info_traits(
        ctx,
        type_hash_where_clause,
        align_hash_where_clause,
        align_to_where_clause,
        type_hash_body,
        align_hash_body,
        align_to_body,
    )
}

/// [`TypeInfo`] derive code for enum types.
fn gen_enum_type_info_impl(ctx: TypeInfoContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut all_type_hashes = vec![];
    let mut all_align_hashes = vec![];
    let mut all_align_tos = vec![];
    let mut all_field_types = vec![];
    let mut all_eps_params = HashSet::new();

    // Process each variant
    for variant in &e.variants {
        let ident = &variant.ident;
        let mut type_hash = quote! { Hash::hash(stringify!(#ident), hasher); };
        let mut field_types = vec![];
        let mut align_hash = quote! {};
        let mut align_to = quote! {};

        match &variant.fields {
            syn::Fields::Unit => {}

            syn::Fields::Named(fields) => {
                for field in &fields.named {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = &field.ty;
                    field_types.push(field_type);

                    let field_type_ts = quote! { SerType<#field_type> };

                    type_hash.extend([quote! {
                        Hash::hash(stringify!(#field_name), hasher);
                        <#field_type_ts as TypeHash>::type_hash(hasher);
                    }]);

                    align_hash.extend([quote! {
                        <#field_type_ts as AlignHash>::align_hash(hasher, offset_of);
                    }]);

                    align_to.extend([quote! {
                        if align_to < <#field_type as AlignTo>::align_to() {
                            align_to = <#field_type as AlignTo>::align_to();
                        }
                    }]);

                    if !ctx.is_zero_copy {
                        // We look for type parameters that are types of fields
                        if let Some(field_type_id) = get_ident(field_type) {
                            if ctx.type_params.contains(field_type_id) {
                                all_eps_params.insert(field_type_id);
                            }
                        }
                    }
                }
            }

            syn::Fields::Unnamed(fields) => {
                for (field_idx, field) in fields.unnamed.iter().enumerate() {
                    let field_name = field_idx.to_string();
                    let field_type = &field.ty;
                    field_types.push(field_type);

                    let field_type_ts = quote! { SerType<#field_type> };

                    type_hash.extend([quote! {
                        Hash::hash(#field_name, hasher);
                        <#field_type_ts as TypeHash>::type_hash(hasher);
                    }]);

                    align_hash.extend([quote! {
                        <#field_type_ts as AlignHash>::align_hash(hasher, offset_of);
                    }]);

                    align_to.extend([quote! {
                        if align_to < <#field_type as AlignTo>::align_to() {
                            align_to = <#field_type as AlignTo>::align_to();
                        }
                    }]);

                    if !ctx.is_zero_copy {
                        // We look for type parameters that are types of fields
                        if let Some(field_type_id) = get_ident(field_type) {
                            if ctx.type_params.contains(field_type_id) {
                                all_eps_params.insert(field_type_id);
                            }
                        }
                    }
                }
            }
        }

        all_type_hashes.push(type_hash);
        all_align_hashes.push(align_hash);
        all_align_tos.push(align_to);
        all_field_types.extend(field_types);
    }

    let (where_clause_type_hash, where_clause_align_hash, where_clause_align_to) =
        gen_type_info_where_clauses(ctx.where_clause, ctx.is_zero_copy, &all_field_types);

    let type_hash_body = gen_type_hash_body(&ctx, &all_type_hashes);
    let align_hash_body = gen_enum_align_hash_body(&ctx, &all_align_hashes);
    let align_to_body = quote! {
        let mut align_to = core::mem::align_of::<Self>();
        #(
            #all_align_tos
        )*
        align_to
    };

    let align_to_body = if ctx.is_zero_copy {
        Some(align_to_body)
    } else {
        None
    };

    gen_type_info_traits(
        ctx,
        where_clause_type_hash,
        where_clause_align_hash,
        where_clause_align_to,
        type_hash_body,
        align_hash_body,
        align_to_body,
    )
}

/// Generates a [partial ε-serde](TypeInfo) implementation for custom types.
///
/// It generates implementations just for the traits `CopyType`, `AlignTo`,
/// `TypeHash`, and `AlignHash`. See the documentation of [`Epserde`] for
/// more information.
#[proc_macro_derive(TypeInfo, attributes(epserde_zero_copy, epserde_deep_copy, epserde))]
pub fn type_info_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut derive_input = parse_macro_input!(input as DeriveInput);

    if let Some(where_clause) = &derive_input.generics.where_clause {
        return syn::Error::new_spanned(
            where_clause,
            "The derive macros do not support where clauses on the original type",
        )
        .to_compile_error()
        .into();
    }

    derive_input.generics.make_where_clause();
    let (generics_for_impl, generics_for_type, where_clause) =
        derive_input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let attrs = match parse_epserde_attrs(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let (_, type_params, const_params) = match get_type_const_params(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    emit_deprecation_warnings(&attrs, &derive_input.ident);

    _type_info_derive(
        &derive_input,
        type_params,
        const_params,
        generics_for_impl,
        generics_for_type,
        where_clause,
        attrs.is_zero_copy,
    )
}

/// Completes the [`TypeInfo`] derive macro using precomputed data.
///
/// This method is used by the [`Epserde`] derive macro to
/// avoid recomputing the same data twice.
fn _type_info_derive(
    derive_input: &DeriveInput,
    type_params: HashSet<&syn::Ident>,
    const_params: Vec<&syn::Ident>,
    generics_for_impl: ImplGenerics<'_>,
    generics_for_type: TypeGenerics<'_>,
    where_clause: &WhereClause,
    is_zero_copy: bool,
) -> proc_macro::TokenStream {
    // Add reprs
    let mut repr_attrs = derive_input
        .attrs
        .iter()
        .filter(|x| x.meta.path().is_ident("repr"))
        .map(|x| x.meta.require_list().unwrap().tokens.to_string())
        .collect::<Vec<_>>();

    // Order of repr attributes does not matter
    repr_attrs.sort();

    let ctx = TypeInfoContext {
        name: &derive_input.ident,
        type_params,
        const_params,
        generics_for_type,
        generics_for_impl,
        where_clause,
        is_zero_copy,
        repr_attrs,
    };

    match &derive_input.data {
        Data::Struct(s) => gen_struct_type_info_impl(ctx, s).into(),
        Data::Enum(e) => gen_enum_type_info_impl(ctx, e).into(),
        Data::Union(_) => {
            syn::Error::new_spanned(&derive_input.ident, "Union types are not supported")
                .to_compile_error()
                .into()
        }
    }
}
