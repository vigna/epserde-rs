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

/// Field-level marker state for the new symmetric attributes.
///
/// `Default` means "neither marker present" — the field follows the
/// default classification and dispatch rules.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum FieldMarker {
    #[default]
    None,
    /// `#[epserde(force_repl)]` — contributes wrapper occurrences to
    /// replaceability; dispatch flips to `_deser_eps_inner`.
    ForceRepl,
    /// `#[epserde(force_irrepl)]` — contributes a direct (single-segment
    /// generic) occurrence to irreplaceability; dispatch flips to
    /// `_deser_full_inner`.
    ForceIrrepl,
}

/// Reads `#[epserde(force_repl)]` / `#[epserde(force_irrepl)]` off a field.
/// The two markers are mutually exclusive on the same field; validation
/// for that (and for argument shape) lives in Task 7.
fn field_marker(field: &syn::Field) -> FieldMarker {
    let mut result = FieldMarker::None;
    for attr in &field.attrs {
        if !attr.meta.path().is_ident("epserde") {
            continue;
        }
        // Parse errors are intentionally swallowed here; the per-field validator runs the same attribute walk with proper error propagation and emits the diagnostic.
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("force_repl") {
                result = FieldMarker::ForceRepl;
            } else if meta.path.is_ident("force_irrepl") {
                result = FieldMarker::ForceIrrepl;
            }
            Ok(())
        });
    }
    result
}

/// Returns `true` if `ty` syntactically contains any identifier in `params`
/// at any position (path segment, type argument, tuple element, etc.).
///
/// Used to decide whether a field should be ε-copy deserialized: a field
/// whose type mentions a replaceable parameter must be ε-copy deserialized
/// so that the result's type matches the corresponding slot in the parent's
/// substituted `DeserType<'_>`.
///
/// Recurses into the variants of [`syn::Type`] that epserde supports:
/// `Path`, `Tuple`, `Array`, `Slice`, `Paren`, and `Group`. All other
/// variants return `false`.
fn type_contains_any(ty: &syn::Type, params: &HashSet<&syn::Ident>) -> bool {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            // A bare single-segment path like T (no leading colon, no angle
            // brackets) is itself a replaceable parameter if found in params.
            if path.leading_colon.is_none() && path.segments.len() == 1 {
                let seg = &path.segments[0];
                if seg.arguments.is_empty() && params.contains(&seg.ident) {
                    return true;
                }
            }
            // In all cases, recurse into angle-bracketed generic arguments of
            // every segment (e.g. Vec<T>, Box<T>, HashMap<K, V>).
            // We do NOT match the segment ident itself in multi-segment paths:
            // a path like B::Word has B as a qualifier, not as the type,
            // so B should not be counted as "containing" the param B.
            for segment in &path.segments {
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    for arg in &ab.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            if type_contains_any(t, params) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        syn::Type::Tuple(t) => t.elems.iter().any(|e| type_contains_any(e, params)),
        syn::Type::Array(a) => type_contains_any(&a.elem, params),
        syn::Type::Slice(s) => type_contains_any(&s.elem, params),
        syn::Type::Paren(p) => type_contains_any(&p.elem, params),
        syn::Type::Group(g) => type_contains_any(&g.elem, params),
        _ => false,
    }
}

/// Per-field classification record produced by `classify_repl_params`.
///
/// One entry per generic type parameter of the struct/enum being derived.
/// The same parameter may show up replaceable (from one field) and
/// irreplaceable (from another) — the caller detects this as a conflict.
struct ParamClassification<'a> {
    /// The parameter being classified.
    ident: &'a syn::Ident,
    /// Set of field names where the parameter appears in a position that
    /// contributes to replaceability. Used for diagnostic messages.
    replaceable_in: Vec<proc_macro2::TokenStream>,
    /// Set of field names where the parameter appears in a position that
    /// contributes to irreplaceability. Used for diagnostic messages.
    irreplaceable_in: Vec<proc_macro2::TokenStream>,
}

/// Walks every field's type and classifies each generic parameter's
/// occurrences as replaceable, irreplaceable, or neither (inside
/// PhantomData<…> or absent). Returns one record per generic parameter,
/// in declaration order.
///
/// The walker treats PhantomData<…> as a barrier: occurrences inside it
/// (at any depth) contribute to neither classification.
///
/// Marker handling at the direct (single-segment) field level:
/// - `None` (default) → contributes to replaceable.
/// - `ForceIrrepl` → contributes to irreplaceable (the marker exists to
///   override the natural-repl default).
/// - `ForceRepl` on a direct field → contributes to replaceable (same as
///   default; the marker is a silent no-op there).
///
/// Marker handling at the type-argument level:
/// - `None` → contributes to irreplaceable.
/// - `ForceRepl` → contributes to replaceable.
/// - `ForceIrrepl` is rejected at validation time on non-direct fields
///   (Task 6), so the walker does not need to handle that combination.
fn classify_repl_params<'a>(
    type_params: &[&'a syn::Ident],
    fields: &[(proc_macro2::TokenStream, &syn::Type, FieldMarker)],
) -> Vec<ParamClassification<'a>> {
    let mut out: Vec<ParamClassification<'a>> = type_params
        .iter()
        .map(|p| ParamClassification {
            ident: p,
            replaceable_in: Vec::new(),
            irreplaceable_in: Vec::new(),
        })
        .collect();

    for (field_name, field_type, marker) in fields {
        // A field whose type is exactly a single-segment generic adds the
        // parameter to one of the buckets per the marker.
        if let Some(ident) = get_ident(field_type) {
            if let Some(rec) = out.iter_mut().find(|r| r.ident == ident) {
                match marker {
                    FieldMarker::ForceIrrepl => rec.irreplaceable_in.push(field_name.clone()),
                    _ => rec.replaceable_in.push(field_name.clone()),
                }
                continue;
            }
        }
        // Otherwise walk the field's type, classifying each generic-ident
        // occurrence. ForceRepl-marked fields contribute to replaceable;
        // unmarked fields contribute to irreplaceable. Inside PhantomData<…>
        // nothing is recorded.
        let field_is_marked = matches!(marker, FieldMarker::ForceRepl);
        collect_occurrences(
            field_type, field_is_marked, field_name, type_params, &mut out, false,
        );
    }

    out
}

/// Recursive helper for `classify_repl_params`. `inside_phantom` becomes
/// true when the walk descends into the type arguments of a PhantomData
/// path segment; while it is true, no occurrences are recorded.
fn collect_occurrences<'a>(
    ty: &syn::Type,
    field_marked: bool,
    field_name: &proc_macro2::TokenStream,
    type_params: &[&'a syn::Ident],
    out: &mut [ParamClassification<'a>],
    inside_phantom: bool,
) {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            for segment in &path.segments {
                let segment_is_phantom = segment.ident == "PhantomData";
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let descend_inside_phantom = inside_phantom || segment_is_phantom;
                    for arg in &ab.args {
                        match arg {
                            syn::GenericArgument::Type(t) => {
                                // If t is a bare single-segment generic ident,
                                // record this position; otherwise recurse into t.
                                let bare = get_ident(t).and_then(|id| {
                                    type_params.iter().find(|p| **p == id).copied()
                                });
                                if let Some(p_ident) = bare {
                                    if !descend_inside_phantom {
                                        let rec = out
                                            .iter_mut()
                                            .find(|r| r.ident == p_ident)
                                            .expect("ident is in type_params");
                                        if field_marked {
                                            rec.replaceable_in.push(field_name.clone());
                                        } else {
                                            rec.irreplaceable_in.push(field_name.clone());
                                        }
                                    }
                                } else {
                                    collect_occurrences(
                                        t,
                                        field_marked,
                                        field_name,
                                        type_params,
                                        out,
                                        descend_inside_phantom,
                                    );
                                }
                            }
                            syn::GenericArgument::AssocType(a) => {
                                collect_occurrences(
                                    &a.ty,
                                    field_marked,
                                    field_name,
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
                collect_occurrences(
                    e, field_marked, field_name, type_params, out, inside_phantom,
                );
            }
        }
        syn::Type::Array(a) => collect_occurrences(
            &a.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        syn::Type::Slice(s) => collect_occurrences(
            &s.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        syn::Type::Paren(p) => collect_occurrences(
            &p.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        syn::Type::Group(g) => collect_occurrences(
            &g.elem, field_marked, field_name, type_params, out, inside_phantom,
        ),
        _ => {}
    }
}

/// Generates a method call for field ε-copy deserialization.
///
/// Takes care of choosing `_deser_eps_inner` or `_deser_full_inner`
/// depending on whether the field type mentions a replaceable parameter,
/// and uses the special method `_deser_eps_inner_special` for
/// `PhantomDeserData`.
///
/// The type of `field_name` is [`proc_macro2::TokenStream`] because it
/// can be either an identifier (for named fields) or an index (for
/// unnamed fields).
fn gen_eps_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    repl_params: &HashSet<&syn::Ident>,
    _marker: FieldMarker,
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
        // PhantomDeserData, but it should be good enough in practice
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: unsafe { <#field_type>::_deser_eps_inner_special(backend)? });
            }
            // PhantomData<...> is handled natively: we emit a literal
            // PhantomData whose generic parameter is inferred from the
            // surrounding Self::DeserType<'a> struct literal. This
            // matches whatever substitution is applied to the parent
            // type, without the derive computing it explicitly.
            if segment.ident == "PhantomData" {
                return syn::parse_quote!(#field_name: ::core::marker::PhantomData);
            }
        }
    }

    // If the field type mentions any replaceable parameter we proceed
    // with ε-copy deserialization; otherwise full-copy.
    if type_contains_any(field_type, repl_params) {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_eps_inner(backend)? })
    } else {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_full_inner(backend)? })
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
    /// Type-parameter idents listed in `#[epserde(force_repl(...))]`.
    force_repl: Vec<syn::Ident>,
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
    let mut force_repl: Vec<syn::Ident> = Vec::new();

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
                } else if meta.path.is_ident("force_repl") {
                    meta.parse_nested_meta(|inner| {
                        let ident = inner.path.require_ident()?.clone();
                        force_repl.push(ident);
                        Ok(())
                    })
                } else {
                    Err(meta.error("expected `zero_copy`, `deep_copy`, `bound`, or `force_repl`"))
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
        force_repl,
    })
}

/// Emits deprecation warnings for old-style `#[epserde(zero_copy)]` and
/// `#[epserde_deep_copy]` attributes during compilation.
fn emit_deprecation_warnings(attrs: &EpserdeAttrs, type_name: &syn::Ident) {
    if attrs.deprecated_zero_copy {
        eprintln!(
            "warning: use `#[epserde(zero_copy)]` instead of `#[epserde_zero_copy]` on type `{type_name}`"
        );
    }
    if attrs.deprecated_deep_copy {
        eprintln!(
            "warning: use `#[epserde(deep_copy)]` instead of `#[epserde_deep_copy]` on type `{type_name}`"
        );
    }
}

/// For each bounded type parameter that is the type of some field, bounds the
/// associated (de)serialization types with the same trait bounds of the type.
fn bound_ser_deser_types(
    derive_input: &DeriveInput,
    repl_params: &HashSet<&syn::Ident>,
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
            if !t.bounds.is_empty() && repl_params.contains(ident) {
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

/// Generates generics for the deserialization type by replacing replaceable
/// type parameters with their associated deserialization type.
fn gen_generics_for_deser_type(
    ctx: &EpserdeContext,
    repl_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if repl_params.contains(ident) {
                quote!(::epserde::deser::DeserType<'__epserde_desertype, #ident>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates generics for the serialization type by replacing replaceable
/// type parameters with their associated serialization type.
fn gen_generics_for_ser_type(
    ctx: &EpserdeContext,
    repl_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if repl_params.contains(ident) {
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
fn gen_ser_deser_where_clauses(
    field_types: &[&syn::Type],
    is_zero_copy: bool,
    force_repl: &HashSet<&syn::Ident>,
) -> (WhereClause, WhereClause) {
    let mut ser_where_clause = empty_where_clause();
    let mut deser_where_clause = empty_where_clause();

    // Add trait bounds for all field types
    for field_type in field_types {
        // Skip the field_type: SerInner/DeserInner bound for field types
        // that mention a force_repl parameter. Such bounds would shadow
        // the impl's DeserType<'_> projection (Rust issue #152409) and
        // prevent the derived _deser_eps_inner body from type-checking.
        // The forced-repl T: SerInner/DeserInner bound added by the caller
        // is enough for Rust to resolve the wrapper's impl directly.
        if !is_zero_copy && type_contains_any(field_type, force_repl) {
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
    /// Type-parameter idents listed in `#[epserde(force_repl(...))]`,
    /// validated against `type_params`.
    force_repl: Vec<syn::Ident>,
}

/// [`Epserde`] derive code for struct types.
fn gen_epserde_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    // Per-field metadata: (display name, type, marker). The marker is
    // consumed by the classifier and by per-field dispatch.
    let fields_info: Vec<(proc_macro2::TokenStream, &syn::Type, FieldMarker)> = s
        .fields
        .iter()
        .enumerate()
        .map(|(idx, field)| (get_field_name(field, idx), &field.ty, field_marker(field)))
        .collect();

    // Classify each generic parameter's occurrences.
    let classifications = classify_repl_params(&ctx.type_const_params, &fields_info);

    // Conflict diagnostic comes in Task 7; for now just compute repl_params.
    // Also union in ctx.force_repl (struct-level attribute, removed in Task 6)
    // so that existing #[epserde(force_repl(T))] tests continue to pass.
    let mut repl_params: HashSet<&syn::Ident> = classifications
        .iter()
        .filter(|c| !c.replaceable_in.is_empty())
        .map(|c| c.ident)
        .collect();
    for ident in &ctx.force_repl {
        repl_params.insert(ident);
    }

    // Gather field metadata and generate the per-field ε-deser method calls.
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;
        let marker = field_marker(field);
        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            &repl_params,
            marker,
        ));
        field_names.push(field_name);
        field_types.push(field_type);
    }

    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &repl_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &repl_params);
    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &field_types);
    let force_repl_set: HashSet<&syn::Ident> = ctx.force_repl.iter().collect();
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(&field_types, ctx.is_zero_copy, &force_repl_set);

    // Add user-specified bounds from #[epserde(bound(...))]
    ser_where_clause
        .predicates
        .extend(ctx.ser_bounds.iter().cloned());
    deser_where_clause
        .predicates
        .extend(ctx.deser_bounds.iter().cloned());

    // For force_repl parameters, add T: SerInner and T: DeserInner so
    // that the substituted forms SerType<T> and DeserType<'_, T> are
    // well-formed. Naturally replaceable parameters get these bounds for
    // free because the parameter is itself a field type, but forced-repl
    // parameters appear only inside wrappers, so we need them explicitly.
    if !ctx.is_zero_copy {
        for ident in &ctx.force_repl {
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
            &repl_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );

        let is_deep_copy = ctx.is_deep_copy;
        let name_str = name.to_string();

        quote! {
            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Deep;
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = #name<#(#generics_for_ser_type,)*>;
                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    // Check whether the type could be zero-copy but it is not
                    // declared as such, and the attribute `epserde_deep_copy`
                    // is missing
                    const { assert!(!(! #is_deep_copy #(&& <#field_types>::IS_ZERO_COPY)*), concat!("Structure ", #name_str, " could be zero-copy, but it has not been declared as such; use either #[epserde(zero_copy)] or #[epserde(deep_copy)] to silence this error")); }

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
    // Type parameters that are types of some fields in some variant,
    // unioned with the user-declared force_repl idents.
    let mut all_repl_params: HashSet<&syn::Ident> = HashSet::new();
    for variant in &e.variants {
        for field in variant.fields.iter() {
            if let Some(field_type_id) = get_ident(&field.ty) {
                if ctx.type_params.contains(field_type_id) {
                    all_repl_params.insert(field_type_id);
                }
            }
        }
    }
    for ident in &ctx.force_repl {
        all_repl_params.insert(ident);
    }
    // All field types for all variants
    let mut all_fields_types = vec![];

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

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
                        FieldMarker::None,
                    ));
                    field_names.push(quote! { #field_name });
                    field_types.push(field_type);
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

                    field_indices.push(
                        syn::Ident::new(&format!("v{}", field_idx), proc_macro2::Span::call_site())
                            .to_token_stream(),
                    );

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
                        FieldMarker::None,
                    ));
                    field_types.push(field_type);
                    field_names_in_arm.push(field_name);
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

    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &all_repl_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &all_repl_params);
    let tag = (0..variant_arm.len()).collect::<Vec<_>>();

    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &all_fields_types);
    let force_repl_set: HashSet<&syn::Ident> = ctx.force_repl.iter().collect();
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(&all_fields_types, ctx.is_zero_copy, &force_repl_set);

    // Add user-specified bounds from #[epserde(bound(...))]
    ser_where_clause
        .predicates
        .extend(ctx.ser_bounds.iter().cloned());
    deser_where_clause
        .predicates
        .extend(ctx.deser_bounds.iter().cloned());

    // For force_repl parameters, add T: SerInner and T: DeserInner so
    // that the substituted forms SerType<T> and DeserType<'_, T> are
    // well-formed. See gen_epserde_struct_impl for the rationale.
    if !ctx.is_zero_copy {
        for ident in &ctx.force_repl {
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
            &all_repl_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );

        let name_str = name.to_string();

        quote! {
            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Deep;
            }
            #[automatically_derived]

            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = #name<#(#generics_for_ser_type,)*>;

                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    // Check whether the type could be zero-copy but it is not
                    // declared as such, and the attribute `epserde_deep_copy`
                    // is missing
                    const { assert!(!(! #is_deep_copy #(&& <#all_fields_types>::IS_ZERO_COPY)*), concat!("Enum ", #name_str, " could be zero-copy, but it has not been declared as such; use either #[epserde(zero_copy)] or #[epserde(deep_copy)] to silence this error")); }

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
/// of a replaceable type parameter, as the associated type needs to be pinned
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
/// # The `force_repl` attribute
///
/// `#[epserde(force_repl(T, U, ...))]` forces the listed type parameters to be
/// replaceable, even if they do not appear as a field type. See the [ε-serde
/// documentation] for the rationale behind this attribute.
///
/// [ε-serde documentation]:
/// https://docs.rs/epserde/latest/epserde/#example-forcing-transitive-replaceability-with-force_repl

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

    // Validate force_repl idents: each must be a declared type parameter.
    for ident in &attrs.force_repl {
        if !type_params.contains(ident) {
            return syn::Error::new_spanned(
                ident,
                format!("`{}` is not a generic type parameter of this item", ident),
            )
            .to_compile_error()
            .into();
        }
    }

    // force_repl is incompatible with zero-copy types.
    if attrs.is_zero_copy && !attrs.force_repl.is_empty() {
        return syn::Error::new_spanned(
            &attrs.force_repl[0],
            "`force_repl` cannot be used with zero-copy types",
        )
        .to_compile_error()
        .into();
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
        force_repl: attrs.force_repl,
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
    let mut all_repl_params = HashSet::new();

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
                                all_repl_params.insert(field_type_id);
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
                                all_repl_params.insert(field_type_id);
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
