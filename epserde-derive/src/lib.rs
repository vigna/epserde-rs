/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![allow(clippy::collapsible_if)]

//! Derive procedural macros for the [`epserde`](https://crates.io/crates/epserde) crate.

use quote::{ToTokens, quote};
use std::{
    collections::{HashMap, HashSet},
    vec,
};
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

/// Returns the most meaningful span to underline for a field type in a
/// diagnostic.
///
/// For a path type such as `std::ops::ControlFlow<F, E>` this is the span of
/// the final segment (the type constructor `ControlFlow`) rather than the
/// leading qualifier (`std`). The latter is what [`Spanned::span`] falls back
/// to on stable Rust, where multi-token spans cannot be joined
/// ([`proc_macro2::Span::join`] is unstable), so an unelaborated
/// `field_type.span()` underlines only the first token. Non-path types use
/// their natural span.
fn type_diag_span(ty: &syn::Type) -> proc_macro2::Span {
    if let syn::Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            return last.ident.span();
        }
    }
    ty.span()
}

/// Returns true if the given field carries `#[epserde(force_full_copy)]`.
fn is_force_full_copy(field: &syn::Field) -> bool {
    let mut found = false;
    for attr in &field.attrs {
        if !attr.meta.path().is_ident("epserde") {
            continue;
        }
        // Parse errors are intentionally swallowed; the per-field validator
        // runs the same walk with proper error propagation.
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("force_full_copy") {
                found = true;
            }
            Ok(())
        });
    }
    found
}

/// Records into `out` every *replaceable parameter* of `type_params` that
/// occurs in `ty`.
///
/// A type parameter is *replaceable* when it occurs in `ty` in its bare form
/// (as a single-segment path with no arguments, i.e. `T` itself) found by
/// descending through the supported type constructors (generic arguments,
/// tuples, arrays, slices). It is not made replaceable by an occurrence nested
/// inside `PhantomData` nor by one that is only a qualified projection such as
/// `T::Assoc` (opaque, not a bare `T`). The name reflects that such a parameter
/// is a candidate for replacement by `SerType`/`DeserType`.
fn collect_repl_param_occs<'a>(
    ty: &syn::Type,
    type_params: &HashSet<&'a syn::Ident>,
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
                if let Some(p) = type_params.get(id) {
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
                                collect_repl_param_occs(
                                    t,
                                    type_params,
                                    out,
                                    descend_inside_phantom,
                                );
                            }
                            syn::GenericArgument::AssocType(a) => {
                                collect_repl_param_occs(
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
                collect_repl_param_occs(e, type_params, out, inside_phantom);
            }
        }
        syn::Type::Array(a) => collect_repl_param_occs(&a.elem, type_params, out, inside_phantom),
        syn::Type::Slice(s) => collect_repl_param_occs(&s.elem, type_params, out, inside_phantom),
        syn::Type::Paren(p) => collect_repl_param_occs(&p.elem, type_params, out, inside_phantom),
        syn::Type::Group(g) => collect_repl_param_occs(&g.elem, type_params, out, inside_phantom),
        _ => {}
    }
}

/// Returns true if `ty` contains a replaceable parameter from `type_params`.
/// Used to decide whether an unmarked field is ε-copy (a replaceable parameter
/// present) or full-copy (none: nothing to substitute).
fn has_repl_param(ty: &syn::Type, type_params: &HashSet<&syn::Ident>) -> bool {
    let mut out: HashSet<&syn::Ident> = HashSet::new();
    collect_repl_param_occs(ty, type_params, &mut out, false);
    !out.is_empty()
}

/// Examines one field, recording its replaceable parameters into the running
/// sets the caller uses to build the (de)serialization substitution sets and
/// bounds, and returns whether the field is full-copy.
///
/// A field is full-copy when it carries `#[epserde(force_full_copy)]`, or when
/// all its replaceable parameters are listed in `#[epserde(full_copy(...))]`
/// (in particular, when it has none).
///
/// # Arguments
///
/// * `field_type` - The type of the field.
///
/// * `force_full_copy_field` - Whether the field carries the field-level
///   `#[epserde(force_full_copy)]` marker, which makes it a full-copy field.
///
/// * `type_params` - The type parameters of the item that are eligible for
///   substitution. Const parameters are never replaceable, so they must not
///   be included: a bare occurrence of a const parameter in a field type
///   (e.g., as a forwarded generic argument) is indistinguishable from a type
///   at the syntactic level, but must be left untouched by the substitution.
///   Parameters declared with the type-level `#[epserde(phantom(...))]`
///   attribute must be excluded as well, as they are left completely
///   untouched (no substitution, no bounds).
///
/// * `forced_params` - The type parameters pinned to full-copy deserialization
///   by the type-level `#[epserde(full_copy(...))]` attribute.
///
/// * `eps_params` - The ε-copy parameters: the non-force-full replaceable
///   parameters of an ε-copy field. This is the `DeserType` substitution set,
///   used directly.
///
/// * `full_params` - The full-copy parameters: the replaceable parameters of a
///   force-full field, plus the `full_copy(...)`-listed ones. Used for the
///   ε/full conflict diagnostic; its union with `eps_params` is the `SerType`
///   substitution set (all replaceable parameters).
///
/// * `deser_inner_params` - The replaceable parameters (force-full or not) of
///   an ε-copy field. For ε-copy fields the field-type bound is
///   suppressed (it would [shadow the `DeserType<'_>` projection]), so the caller
///   emits an explicit `T: DeserInner` bound for each of these. Parameters of
///   full-copy fields instead obtain `DeserInner` from their field-type bound,
///   so they are not collected here.
///
/// * `eps_field_spans` - Maps each ε-copy parameter to the span of the first
///   ε-copy field using it, so the conflict diagnostic can point at that field.
///
/// * `seq_deep_idents` - The ε-copy parameters occurring as a sequence element
///   in an ε-copy field, each re-spanned to that field.
///
/// * `full_copy_check_fields` - The types of ε-copy fields that also contain a
///   `#[epserde(full_copy(...))]`-pinned parameter. Such a field is sound only
///   when its type holds the pinned parameter full-copy; the caller emits a
///   [consistency assertion] for each, so a field that instead ε-copy
///   deserializes the pinned parameter (e.g. `ControlFlow<F, E>`) gets an
///   readable diagnostic rather than a raw slot mismatch.
///
/// [shadow the `DeserType<'_>` projection]: https://github.com/rust-lang/rust/issues/152409
/// [consistency assertion]: gen_full_copy_consistency_check
#[allow(clippy::too_many_arguments)]
fn classify_field<'a>(
    field_type: &'a syn::Type,
    force_full_copy_field: bool,
    type_params: &HashSet<&'a syn::Ident>,
    forced_params: &HashSet<&'a syn::Ident>,
    eps_params: &mut HashSet<&'a syn::Ident>,
    full_params: &mut HashSet<&'a syn::Ident>,
    deser_inner_params: &mut HashSet<&'a syn::Ident>,
    eps_field_spans: &mut HashMap<&'a syn::Ident, proc_macro2::Span>,
    seq_deep_idents: &mut Vec<syn::Ident>,
    full_copy_check_fields: &mut Vec<&'a syn::Type>,
) -> bool {
    let mut field_occ = HashSet::new();
    collect_repl_param_occs(field_type, type_params, &mut field_occ, false);

    if force_full_copy_field {
        // Every parameter of a force-full field is a full-copy parameter.
        full_params.extend(&field_occ);
        return true;
    }

    // Unmarked field: a force-full parameter is full-copy, otherwise it is an
    // ε-copy parameter. The field is full-copy iff it has no ε-copy parameter.
    let mut has_eps = false;
    let mut has_forced = false;
    for p in &field_occ {
        if forced_params.contains(p) {
            full_params.insert(*p);
            has_forced = true;
        } else {
            eps_params.insert(*p);
            eps_field_spans
                .entry(*p)
                .or_insert_with(|| type_diag_span(field_type));
            has_eps = true;
        }
    }

    if has_eps {
        // The field-type DeserInner bound is suppressed for ε-copy fields, so
        // each of this field parameters (force-full ones included) needs an
        // explicit DeserInner bound emitted by the caller.
        deser_inner_params.extend(&field_occ);
        let mut field_seq_deep = HashSet::new();
        collect_seq_forced_deep_params(field_type, type_params, &mut field_seq_deep, false);
        push_seq_deep_idents(&field_seq_deep, type_diag_span(field_type), seq_deep_idents);

        // An ε-copy field that also pins a parameter to full-copy keeps that
        // parameter verbatim in its DeserType slot while the field's own
        // `_deser_eps_inner` may substitute it: the caller asserts the two
        // agree.
        if has_forced {
            full_copy_check_fields.push(field_type);
        }
    }

    !has_eps
}

/// Records into `out` every type parameter that occurs as the direct element of
/// a literal `Vec<…>`, boxed/bare slice `[…]`, or array `[…; N]` anywhere within
/// `ty`. Such a parameter is forced to be deep-copy for ε-copy stability: were
/// it zero-copy, the containing sequence would ε-copy deserialize to a slice
/// reference, a type not expressible as the original sequence.
///
/// An occurrence nested inside `PhantomData<…>` is ignored: a phantom slot is
/// zero-sized and never serialized, so it imposes no ε-copy-stability
/// requirement (mirroring [`collect_repl_param_occs`], which excludes phantom
/// occurrences from the replaceable set).
fn collect_seq_forced_deep_params<'a>(
    ty: &syn::Type,
    type_params: &HashSet<&'a syn::Ident>,
    out: &mut HashSet<&'a syn::Ident>,
    inside_phantom: bool,
) {
    fn record_if_bare<'a>(
        ty: &syn::Type,
        type_params: &HashSet<&'a syn::Ident>,
        out: &mut HashSet<&'a syn::Ident>,
    ) {
        if let syn::Type::Path(syn::TypePath { qself: None, path }) = ty {
            if path.leading_colon.is_none()
                && path.segments.len() == 1
                && path.segments[0].arguments.is_empty()
            {
                let id = &path.segments[0].ident;
                if let Some(p) = type_params.get(id) {
                    out.insert(*p);
                }
            }
        }
    }

    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            for segment in &path.segments {
                let segment_is_phantom = segment.ident == "PhantomData";
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let is_vec = segment.ident == "Vec";
                    let descend_inside_phantom = inside_phantom || segment_is_phantom;
                    for arg in &ab.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            if is_vec && !descend_inside_phantom {
                                record_if_bare(t, type_params, out);
                            }
                            collect_seq_forced_deep_params(
                                t,
                                type_params,
                                out,
                                descend_inside_phantom,
                            );
                        }
                    }
                }
            }
        }
        syn::Type::Slice(s) => {
            if !inside_phantom {
                record_if_bare(&s.elem, type_params, out);
            }
            collect_seq_forced_deep_params(&s.elem, type_params, out, inside_phantom);
        }
        syn::Type::Array(a) => {
            if !inside_phantom {
                record_if_bare(&a.elem, type_params, out);
            }
            collect_seq_forced_deep_params(&a.elem, type_params, out, inside_phantom);
        }
        syn::Type::Tuple(t) => {
            for e in &t.elems {
                collect_seq_forced_deep_params(e, type_params, out, inside_phantom);
            }
        }
        syn::Type::Reference(r) => {
            collect_seq_forced_deep_params(&r.elem, type_params, out, inside_phantom)
        }
        syn::Type::Paren(p) => {
            collect_seq_forced_deep_params(&p.elem, type_params, out, inside_phantom)
        }
        syn::Type::Group(g) => {
            collect_seq_forced_deep_params(&g.elem, type_params, out, inside_phantom)
        }
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
/// `_deser_eps_inner` when one or more type parameters are ε-copy yet are also
/// replaceable parameters of a field marked `#[epserde(force_full_copy)]`.
///
/// For each conflicting parameter the assertion requires the bound `for<'a> <T
/// as DeserInner>::DeserType<'a>: EitherFullOrEpsCopy<T>`. The blanket impl
/// `impl<T> EitherFullOrEpsCopy<T> for T` makes the bound trivially hold when
/// `DeserType<'a> = T` (the fixed-point condition the user can supply through
/// `bound(deser = ...)`); otherwise the impl does not apply and the
/// `#[diagnostic::on_unimplemented]` message on `EitherFullOrEpsCopy` show a hint
/// alongside rustc's slot-mismatch error.
///
/// Each ident in `conflict_params` is expected to be re-spanned to the ε-copy
/// field that forces the constraint (see [`push_conflict_idents`]), so that the
/// diagnostic points at that field rather than at the derive invocation. The
/// ε-copy field is chosen over the `#[epserde(force_full_copy)]` one because the
/// hint recommends adding `force_full_copy`, so underlining a field that already
/// has it would be contradictory.
///
/// Returns an empty token stream when there are no conflicts.
fn gen_fixed_point_check(conflict_params: &[syn::Ident]) -> proc_macro2::TokenStream {
    if conflict_params.is_empty() {
        return quote!();
    }
    // The failing bound is on __Slot (the deserialization type), not on the
    // bare parameter, so re-spanning the parameter alone leaves the error on the
    // derive invocation. Emitting each call with quote_spanned! at the field
    // span makes the whole call expression carry that span, so the diagnostic
    // points at the ε-copy field.
    let checks = conflict_params.iter().map(|param| {
        quote::quote_spanned! {param.span()=>
            __epserde_fixed_point_check::<
                #param,
                <#param as ::epserde::deser::DeserInner>::DeserType<'_>,
            >();
        }
    });
    quote! {
        fn __epserde_fixed_point_check<__Outer, __Slot: ?Sized>()
        where
            __Slot: ::epserde::deser::EitherFullOrEpsCopy<__Outer>,
        {}
        #(#checks)*
    }
}

/// Substitutes each ε-copy type parameter `P` of `type_params` in a cloned type
/// with its deserialization type `::epserde::deser::DeserType<'lifetime, P>`,
/// reproducing the slot the derive forms for a field in
/// `Self::DeserType<'lifetime>`.
///
/// The fold is total: it descends through every type constructor (via the
/// default [`syn::fold`] recursion), so the result matches exactly what the
/// compiler produces when instantiating the type's `DeserType`, including
/// occurrences inside [`PhantomData`] and qualified projections such as
/// `P::Assoc`. This exactness is what lets the consistency assertion built from
/// the result avoid false positives on a field that legitimately holds `P`
/// full-copy. Forced, phantom, and const parameters, and every other type, are
/// left verbatim.
///
/// [`PhantomData`]: https://doc.rust-lang.org/core/marker/struct.PhantomData.html
struct EpsParamSubst<'a> {
    eps_params: &'a HashSet<&'a syn::Ident>,
    lifetime: &'a syn::Lifetime,
}

impl syn::fold::Fold for EpsParamSubst<'_> {
    fn fold_type(&mut self, ty: syn::Type) -> syn::Type {
        if let syn::Type::Path(tp) = &ty {
            if tp.qself.is_none() && tp.path.leading_colon.is_none() {
                let first = &tp.path.segments[0];
                if first.arguments.is_empty() && self.eps_params.contains(&first.ident) {
                    let lt = self.lifetime;
                    let p = &first.ident;
                    let deser: syn::Type = syn::parse_quote!(::epserde::deser::DeserType<#lt, #p>);
                    if tp.path.segments.len() == 1 {
                        // Bare `P` becomes `DeserType<'lifetime, P>`.
                        return deser;
                    }
                    // Projection `P::Assoc[::…]` becomes `<DeserType<'lifetime,
                    // P>>::Assoc[::…]`, mirroring instantiation of the leading
                    // parameter.
                    let mut rest = tp.path.clone();
                    rest.segments = rest.segments.into_iter().skip(1).collect();
                    return syn::Type::Path(syn::TypePath {
                        qself: Some(syn::QSelf {
                            lt_token: Default::default(),
                            ty: Box::new(deser),
                            position: 0,
                            as_token: None,
                            gt_token: Default::default(),
                        }),
                        path: rest,
                    });
                }
            }
        }
        syn::fold::fold_type(self, ty)
    }
}

/// Generates the consistency assertion injected into `_deser_eps_inner` for each
/// ε-copy field that carries a `#[epserde(full_copy(...))]`-pinned parameter
/// (collected in `check_fields`).
///
/// For each such field the assertion requires `<Field as
/// DeserInner>::DeserType<'_>: FullCopyConsistent<Slot>`, where `Slot` is the
/// field's slot in `Self::DeserType` (the field type with the ε-copy parameters
/// in `eps_params` substituted and the pinned ones left verbatim, built by
/// [`EpsParamSubst`]). The blanket impl `impl<T> FullCopyConsistent<T> for T`
/// makes the bound hold exactly when the field's real deserialization type
/// coincides with the slot — so a field that genuinely holds the pinned
/// parameter full-copy is silent; otherwise, the
/// `#[diagnostic::on_unimplemented]` message on `FullCopyConsistent` surfaces
/// alongside rustc's slot mismatch.
///
/// Each call is emitted with `quote_spanned!` at the field span so the
/// diagnostic points at the offending field. `lifetime` is the lifetime of the
/// enclosing `_deser_eps_inner`, shared by both type arguments.
///
/// Returns an empty token stream when there are no such fields.
fn gen_full_copy_consistency_check(
    check_fields: &[&syn::Type],
    eps_params: &HashSet<&syn::Ident>,
    lifetime: &syn::Lifetime,
) -> proc_macro2::TokenStream {
    if check_fields.is_empty() {
        return quote!();
    }
    let checks = check_fields.iter().map(|field_ty| {
        let mut subst = EpsParamSubst {
            eps_params,
            lifetime,
        };
        let slot_ty = syn::fold::Fold::fold_type(&mut subst, (*field_ty).clone());
        quote::quote_spanned! {type_diag_span(field_ty)=>
            __epserde_full_copy_consistency::<
                <#field_ty as ::epserde::deser::DeserInner>::DeserType<#lifetime>,
                #slot_ty,
            >();
        }
    });
    quote! {
        fn __epserde_full_copy_consistency<__A: ?Sized, __B: ?Sized>()
        where
            __A: ::epserde::deser::FullCopyConsistent<__B>,
        {}
        #(#checks)*
    }
}

/// Generates the ε-copy stability assertion emitted, as a standalone item, for
/// each type parameter that occurs as the direct element of a literal `Vec<…>`,
/// boxed slice, or array in an ε-copy field.
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

/// Computes the conflict parameters, that is, the intersection of `eps_params`
/// (the ε-copy parameters) with `full_params` (the full-copy parameters), and pushes
/// each into `out`, re-spanned to the ε-copy
/// field that uses it (recorded in `eps_field_spans`), so the fixed-point
/// diagnostic points at that field.
///
/// The `full_copy(...)`-listed members of `full_params` never cause an output:
/// being force-full, they are absent from `eps_params`, so the intersection
/// keeps only ε-copy parameters that also occur in a full-copy field.
fn push_conflict_idents(
    eps_params: &HashSet<&syn::Ident>,
    full_params: &HashSet<&syn::Ident>,
    eps_field_spans: &HashMap<&syn::Ident, proc_macro2::Span>,
    out: &mut Vec<syn::Ident>,
) {
    for p in eps_params.intersection(full_params) {
        let mut id = (*p).clone();
        if let Some(span) = eps_field_spans.get(*p) {
            id.set_span(*span);
        }
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
    /// Whether `#[epserde(zero_copy)]` or `#[epserde_zero_copy]` was specified.
    is_zero_copy: bool,
    /// Whether `#[epserde(deep_copy)]` or `#[epserde_deep_copy]` was specified.
    is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl.
    deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl.
    ser_bounds: Vec<WherePredicate>,
    /// Type-parameter idents listed in `#[epserde(full_copy(...))]`. These are
    /// pinned to full-copy: removed from the `DeserType` substitution set, and
    /// kept verbatim in `DeserType<'a>`.
    full_copy_params: Vec<syn::Ident>,
    /// Type-parameter idents listed in `#[epserde(phantom(...))]`. These are
    /// declared phantom throughout the type and left completely untouched: no
    /// `SerType`/`DeserType` substitution and no `SerInner`/`DeserInner`
    /// bounds.
    phantom_params: Vec<syn::Ident>,
    /// Whether old-style `#[epserde(zero_copy)]` was used.
    deprecated_zero_copy: bool,
    /// Whether old-style `#[epserde_deep_copy]` was used.
    deprecated_deep_copy: bool,
}

/// Collects the representation hints of all `repr` attributes of a type,
/// individually normalized (e.g., `align(16)`) and sorted.
///
/// The normalization guarantees that equivalent spellings such as
/// `#[repr(C, align(16))]` and `#[repr(align(16))] #[repr(C)]` yield the same
/// hints, and thus the same alignment hash.
fn repr_hints(attrs: &[syn::Attribute]) -> syn::Result<Vec<String>> {
    let mut hints = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("repr") {
            // A repr attribute may combine several hints, as in
            // #[repr(C, align(16))]
            attr.parse_nested_meta(|meta| {
                let mut hint = meta.path.to_token_stream().to_string();
                // Append the argument of hints such as align(16) or packed(2)
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let args: proc_macro2::TokenStream = content.parse()?;
                    hint = format!("{hint}({args})");
                }
                hints.push(hint);
                Ok(())
            })?;
        }
    }
    hints.sort();
    Ok(hints)
}

/// Parses epserde attributes from `#[epserde(...)]`, `#[epserde(zero_copy)]`,
/// and `#[epserde_(deep_copy)]`.
fn parse_epserde_attrs(input: &DeriveInput) -> syn::Result<EpserdeAttrs> {
    let is_repr_c = repr_hints(&input.attrs)?.iter().any(|hint| hint == "C");

    let mut is_zero_copy = false;
    let mut is_deep_copy = false;
    let mut deser_bounds = Vec::new();
    let mut ser_bounds = Vec::new();
    let mut full_copy_params = Vec::new();
    let mut phantom_params = Vec::new();
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
                } else if meta.path.is_ident("full_copy") {
                    if !meta.input.peek(token::Paren) {
                        return Err(meta.error(
                            "\"full_copy\" is a type-level attribute and requires a parenthesized \
                             list of type parameters, e.g. #[epserde(full_copy(T))]",
                        ));
                    }
                    meta.parse_nested_meta(|inner| {
                        if let Some(ident) = inner.path.get_ident() {
                            full_copy_params.push(ident.clone());
                            Ok(())
                        } else {
                            Err(inner.error("expected a type-parameter identifier"))
                        }
                    })
                } else if meta.path.is_ident("phantom") {
                    if !meta.input.peek(token::Paren) {
                        return Err(meta.error(
                            "\"phantom\" is a type-level attribute and requires a parenthesized \
                             list of type parameters, e.g. #[epserde(phantom(T))]",
                        ));
                    }
                    meta.parse_nested_meta(|inner| {
                        if let Some(ident) = inner.path.get_ident() {
                            phantom_params.push(ident.clone());
                            Ok(())
                        } else {
                            Err(inner.error("expected a type-parameter identifier"))
                        }
                    })
                } else {
                    Err(meta.error(
                        "expected \"zero_copy\", \"deep_copy\", \"bound\", \"full_copy\", or \"phantom\"",
                    ))
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
        full_copy_params,
        phantom_params,
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

/// For each bounded type parameter that is substituted in an associated
/// (de)serialization type, bounds that substituted form with the same trait
/// bounds as the parameter.
///
/// The two substitution sets differ: `SerType` substitutes every replaceable
/// parameter, whereas `DeserType` omits the force-full ones
/// and those occurring only in full-copy fields. A parameter substituted in
/// `SerType` but not in `DeserType` appears as `SerType<T>` (so its bounds
/// propagate on the ser side) but is kept verbatim as `T` in `DeserType<'a>`
/// (so it carries its own declared bounds through the impl generics, and needs
/// no `DeserType<'a, T>` bound).
///
/// # Arguments
///
/// * `derive_input` - The item being derived, whose generic parameters carry
///   the trait bounds to propagate.
///
/// * `params` - All replaceable parameters; for each, a `SerType<T>` bound is
///   added to `ser_where_clause`.
///
/// * `eps_params` - All ε-copy type parameters appearing at a variable
///   position; for each, a `DeserType<'a, T>` bound is added to
///   `deser_where_clause`.
///
/// * `ser_where_clause` - The `SerInner` where clause, extended in place.
///
/// * `deser_where_clause` - The `DeserInner` where clause, extended in place.
fn bound_ser_deser_types(
    derive_input: &DeriveInput,
    params: &HashSet<&syn::Ident>,
    eps_params: &HashSet<&syn::Ident>,
    ser_where_clause: &mut WhereClause,
    deser_where_clause: &mut WhereClause,
) {
    // If there are bounded type parameters which are substituted, we
    // need to impose the same bounds on the associated SerType/DeserType.
    for param in &derive_input.generics.params {
        if let syn::GenericParam::Type(t) = param {
            let ident = &t.ident;

            // Relaxed bounds (?Sized) cannot be transplanted: Rust permits
            // them only on the type parameters of the item itself, not on
            // arbitrary bounded types in a where clause. Dropping them is
            // sound, as the substituted forms are used as arguments of the
            // very type that declares the parameter ?Sized.
            let bounds: Punctuated<TypeParamBound, Plus> = t
                .bounds
                .iter()
                .filter(|b| {
                    !matches!(b, TypeParamBound::Trait(tb)
                        if matches!(tb.modifier, syn::TraitBoundModifier::Maybe(_)))
                })
                .cloned()
                .collect();

            if bounds.is_empty() {
                continue;
            }

            // Add the trait bounds of the type to the DeserType, but only for
            // parameters actually substituted on the deser side.
            if eps_params.contains(ident) {
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
                        bounds: bounds.clone(),
                    }));
            }

            // Add the trait bounds of the type to the SerType.
            if params.contains(ident) {
                ser_where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: syn::parse_quote!(
                            ::epserde::ser::SerType<#ident>
                        ),
                        colon_token: token::Colon::default(),
                        bounds,
                    }));
            }
        }
    }
}

/// Adds to the (de)serialization where clauses the bounds for the given field
/// type.
///
/// The deser side always gets `ty: DeserInner`. The ser side gets `ty: SerInner`
/// for a deep-copy type, or `ty: ZeroCopy` for a zero-copy type (every field of
/// a zero-copy type must itself be zero-copy). The deser side stays at
/// `DeserInner` even for zero-copy types: we cannot impose `DeserType<'_> =
/// &Self` because of primitive types.
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

/// Generates generics for the serialization type by replacing every
/// replaceable parameter with its associated serialization type.
fn gen_generics_for_ser_type(
    ctx: &EpserdeContext,
    params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if params.contains(ident) {
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
/// `full_deser_fields[i]` is `true` when field `i` is full-copy (either because
/// it carries `#[epserde(force_full_copy)]` or because its type contains no variable
/// position to substitute).
///
/// For ε-copy fields the field-type bound is suppressed: it would shadow the
/// impl's `DeserType<'_>` projection (Rust issue #152409), making the derived
/// `_deser_eps_inner` body fail to type-check. The per-parameter `T:
/// SerInner`/`T: DeserInner` bounds emitted by the caller are sufficient for
/// Rust to resolve impls of wrappers whose `DeserType<'_>` is uniform across
/// kinds (`Box<T>`, `Rc<T>`, `Arc<T>`, `Option<T>`, `Range<T>`, tuples).
///
/// For wrappers whose resolution depends on `T`'s kind (`Vec<T>`, `Box<[T]>`,
/// `[T; N]`), the user must additionally bound `T: ZeroCopy` or `T:
/// DeepCopy`; the derive does not emit those bounds because the choice is not
/// derivable from the field type alone.
///
/// Note that only the `DeserInner` bound must be suppressed, as only
/// `_deser_eps_inner` produces a value of the deserialization type. `SerType`
/// is never materialized: it appears only at the type level, as a projection
/// whose required capabilities are asserted in place (e.g., `FieldType:
/// SerInner<SerType: TypeHash>` in the type-info where clauses), so a
/// `SerInner` field-type bound would be harmless. The two bounds are dropped
/// together for simplicity, and to keep the `SerInner`/`DeserInner` requirement
/// sets symmetric.
///
/// Moreover, only a bare field-type bound is impossible. A bound carrying the
/// GAT equality (`for<'a> FieldType: DeserInner<DeserType<'a> = σ(FieldType)>`,
/// where σ substitutes the ε-copy parameters) compiles and even subsumes the
/// user-supplied kind bounds (issue #152409's own workaround: the shadowing
/// where-clause itself supplies the equality the body needs). We prefer the
/// current solution because the right-hand side is the same syntactic
/// substitution the current encoding relies on, so nothing is gained in
/// expressiveness, while a wrong substitution would surface at use sites
/// instead of inside the derived impl, and field types (possibly private) would
/// leak into the impl's public where clause, which downstream generic code
/// would have to repeat per field.
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
    /// Type parameters eligible for substitution: the declared type
    /// parameters minus those declared phantom by the type-level
    /// `#[epserde(phantom(...))]` attribute. The replaceable-parameter walk
    /// matches against this set only, so phantom parameters are left
    /// completely untouched (no substitution, no bounds).
    repl_params: HashSet<&'a syn::Ident>,
    /// Type parameters pinned to full-copy deserialization by the type-level
    /// `#[epserde(full_copy(...))]` attribute, as a subset of the declared type
    /// parameters.
    forced_params: HashSet<&'a syn::Ident>,
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
    // The ε-copy parameters: the non-force-full replaceable parameters of an
    // ε-copy field. This is the DeserType substitution set.
    let mut eps_params = HashSet::new();
    // The full-copy parameters: the replaceable parameters of a force-full field,
    // plus the full_copy(...)-listed ones. Used for the ε/full conflict
    // diagnostic; its union with eps_params is the SerType substitution set.
    let mut full_params = HashSet::new();
    // Parameters in an ε-copy field, whose suppressed field-type bound must be
    // replaced by an explicit T: DeserInner (force-full ones included).
    let mut deser_inner_params = HashSet::new();
    // Parameters forced to be deep-copy because they occur as a sequence
    // element in an ε-copy field, each re-spanned to the field that forces it,
    // so that the stability diagnostic points at the offending field.
    let mut seq_deep_idents: Vec<syn::Ident> = vec![];
    // For each ε-copy parameter, the span of the first ε-copy field that uses
    // it, so that a parameter that is also full-copy can have its conflict
    // diagnostic point at that field.
    let mut eps_field_spans: HashMap<&syn::Ident, proc_macro2::Span> = HashMap::new();
    let mut full_deser_fields = vec![];
    // Types of ε-copy fields that also pin a full_copy(...) parameter, for the
    // full-copy consistency assertion.
    let mut full_copy_check_fields: Vec<&syn::Type> = vec![];

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;
        let force_full_copy = is_force_full_copy(field);

        if force_full_copy && (ctx.is_zero_copy || !has_repl_param(field_type, &ctx.repl_params)) {
            let type_name = &ctx.derive_input.ident;
            eprintln!(
                "warning: #[epserde(force_full_copy)] on field {field_name} of type {type_name} has no effect; consider removing the marker"
            );
        }

        let deser_full = classify_field(
            field_type,
            force_full_copy,
            &ctx.repl_params,
            &ctx.forced_params,
            &mut eps_params,
            &mut full_params,
            &mut deser_inner_params,
            &mut eps_field_spans,
            &mut seq_deep_idents,
            &mut full_copy_check_fields,
        );

        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            deser_full,
        ));

        field_names.push(field_name);
        field_types.push(field_type);
        full_deser_fields.push(deser_full);
    }

    // SerType substitutes every replaceable parameter
    // uniformly): the union of the ε-copy and full-copy parameters.
    let params: HashSet<&syn::Ident> = eps_params.union(&full_params).copied().collect();
    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &eps_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &params);
    // A type parameter that is both ε-copy and full-copy produces
    // a slot mismatch in the generated _deser_eps_inner: one occurrence
    // becomes <T as DeserInner>::DeserType<'_>, the other stays as T. The
    // user can resolve the conflict with a bound that forces DeserType<'_>
    // = T (automatic for ZeroCopy types). The assertion below requests
    // EitherFullOrEpsCopy for each conflicting parameter so that, when the
    // bound is missing, the on_unimplemented message points at the fix
    // instead of leaving the user with rustc's raw slot mismatch. Each
    // conflicting parameter is re-spanned to the ε-copy field that uses it,
    // so the diagnostic points at that field.
    let mut conflict_params: Vec<syn::Ident> = vec![];
    push_conflict_idents(
        &eps_params,
        &full_params,
        &eps_field_spans,
        &mut conflict_params,
    );
    let fixed_point_check = gen_fixed_point_check(&conflict_params);
    let deser_eps_lifetime: syn::Lifetime = syn::parse_quote!('deser_eps_inner_lifetime);
    let full_copy_consistency_check =
        gen_full_copy_consistency_check(&full_copy_check_fields, &eps_params, &deser_eps_lifetime);
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

    // Emit T: SerInner for every SerType-substituted parameter, and
    // T: DeserInner for every parameter occurring in an ε-copy field.
    // The field-type bound was skipped above for ε-copy fields;
    // these per-parameter bounds let Rust resolve kind-uniform wrapper
    // impls (Box, Rc, Arc, Option, Range, tuples). For wrappers whose
    // resolution depends on T's kind (Vec, Box<[…]>, [T; N]),
    // the user must additionally bound T: ZeroCopy or T: DeepCopy.
    if !ctx.is_zero_copy {
        for ident in &params {
            ser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::ser::SerInner
            ));
        }
        for ident in &deser_inner_params {
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
            &params,
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
                    // declared as such, and the attribute epserde_deep_copy
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
                    #full_copy_consistency_check

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
    // The ε-copy parameters: the non-force-full replaceable parameters of some
    // ε-copy field. This is the DeserType substitution set.
    let mut eps_params = HashSet::new();
    // The full-copy parameters: the replaceable parameters of some force-full
    // field, plus the full_copy(...)-listed ones. Used for the ε/full conflict
    // diagnostic; its union with eps_params is the SerType set.
    let mut all_full_params = HashSet::new();
    // Parameters in an ε-copy field, whose suppressed field-type bound must be
    // replaced by an explicit T: DeserInner (force-full ones included).
    let mut deser_inner_params = HashSet::new();
    // For each ε-copy parameter, the span of the first ε-copy field that uses
    // it, so that a parameter that is also full-copy can have its conflict
    // diagnostic point at that field.
    let mut eps_field_spans: HashMap<&syn::Ident, proc_macro2::Span> = HashMap::new();
    // Parameters forced to be deep-copy because they occur as a sequence
    // element in an ε-copy field, each re-spanned to the offending field.
    let mut seq_deep_idents: Vec<syn::Ident> = vec![];
    // All field types for all variants
    let mut all_fields_types = vec![];
    // Whether each entry in all_fields_types is full-copy.
    let mut all_full_deser_fields = vec![];
    // Types of ε-copy fields that also pin a full_copy(...) parameter, for the
    // full-copy consistency assertion.
    let mut full_copy_check_fields: Vec<&syn::Type> = vec![];

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
                    let force_full_copy = is_force_full_copy(field);

                    if force_full_copy
                        && (ctx.is_zero_copy || !has_repl_param(field_type, &ctx.repl_params))
                    {
                        let type_name = &ctx.derive_input.ident;
                        eprintln!(
                            "warning: #[epserde(force_full_copy)] on field {ident}::{field_name} of type {type_name} has no effect; consider removing the marker"
                        );
                    }

                    let deser_full = classify_field(
                        field_type,
                        force_full_copy,
                        &ctx.repl_params,
                        &ctx.forced_params,
                        &mut eps_params,
                        &mut all_full_params,
                        &mut deser_inner_params,
                        &mut eps_field_spans,
                        &mut seq_deep_idents,
                        &mut full_copy_check_fields,
                    );

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
                    let force_full_copy = is_force_full_copy(field);

                    if force_full_copy
                        && (ctx.is_zero_copy || !has_repl_param(field_type, &ctx.repl_params))
                    {
                        let type_name = &ctx.derive_input.ident;
                        let idx = syn::Index::from(field_idx);
                        eprintln!(
                            "warning: #[epserde(force_full_copy)] on field {ident}::{idx_index} of type {type_name} has no effect; consider removing the marker",
                            idx_index = idx.index,
                        );
                    }

                    let deser_full = classify_field(
                        field_type,
                        force_full_copy,
                        &ctx.repl_params,
                        &ctx.forced_params,
                        &mut eps_params,
                        &mut all_full_params,
                        &mut deser_inner_params,
                        &mut eps_field_spans,
                        &mut seq_deep_idents,
                        &mut full_copy_check_fields,
                    );

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

    // SerType substitutes every replaceable parameter
    let params: HashSet<&syn::Ident> = eps_params.union(&all_full_params).copied().collect();
    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &eps_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &params);
    // See the struct branch for the rationale: a parameter that appears in both
    // ε-copy and full-copy fields needs DeserType<'_> = T to make the generated
    // body type-check. Each conflicting parameter is re-spanned to the ε-copy
    // field that uses it, so the diagnostic points at that field.
    let mut conflict_params: Vec<syn::Ident> = vec![];
    push_conflict_idents(
        &eps_params,
        &all_full_params,
        &eps_field_spans,
        &mut conflict_params,
    );
    let fixed_point_check = gen_fixed_point_check(&conflict_params);
    let deser_eps_lifetime: syn::Lifetime = syn::parse_quote!('deser_eps_inner_lifetime);
    let full_copy_consistency_check =
        gen_full_copy_consistency_check(&full_copy_check_fields, &eps_params, &deser_eps_lifetime);
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

    // Emit T: SerInner for every SerType-substituted parameter, and
    // T: DeserInner for every parameter occurring in an ε-copy field.
    // The field-type bound was skipped above for ε-copy fields;
    // these per-parameter bounds let Rust resolve kind-uniform wrapper
    // impls (Box, Rc, Arc, Option, Range, tuples). For wrappers whose
    // resolution depends on T's kind (Vec, Box<[…]>, [T; N]),
    // the user must additionally bound T: ZeroCopy or T: DeepCopy.
    if !ctx.is_zero_copy {
        for ident in &params {
            ser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::ser::SerInner
            ));
        }
        for ident in &deser_inner_params {
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
            &params,
            &eps_params,
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
                    // declared as such, and the attribute epserde_deep_copy
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
                    #full_copy_consistency_check

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
/// It generates implementations for the traits `CopyType`, `TypeHash`,
/// `AlignHash`, `SerInner`, and `DeserInner` (and `AlignTo` for zero-copy
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
/// - its occurrences of type paramters do not contribute to the ε-copy parameters.
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

    // Validate per-field epserde attributes: the only valid field-level key is
    // force_full_copy.
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
                if meta.path.is_ident("force_full_copy") {
                    if meta.input.peek(syn::token::Paren) {
                        return Err(meta.error(
                            "\"force_full_copy\" is a field-level marker and takes no arguments; \
                             use #[epserde(force_full_copy)]",
                        ));
                    }
                    if attrs.is_zero_copy {
                        return Err(meta.error(
                            "\"force_full_copy\" cannot appear inside a zero-copy type",
                        ));
                    }
                    if is_phantom_deser_data {
                        return Err(meta.error(
                            "\"force_full_copy\" has no operational effect on a PhantomDeserData<T> field; \
                             remove the marker, or migrate to PhantomData<T>",
                        ));
                    }
                    return Ok(());
                }
                Err(meta.error("expected \"force_full_copy\""))
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

    // Validate the type-level #[epserde(full_copy(...))] attribute and build
    // the set of forced parameters, referencing the declared type parameters.
    let mut forced_params: HashSet<&syn::Ident> = HashSet::new();
    for ident in &attrs.full_copy_params {
        if attrs.is_zero_copy {
            return syn::Error::new_spanned(
                ident,
                "full_copy(...) cannot be used on a zero-copy type, as its deserialization type is a reference",
            )
            .to_compile_error()
            .into();
        }
        if let Some(decl) = type_params.iter().copied().find(|p| **p == *ident) {
            forced_params.insert(decl);
        } else if const_params.iter().any(|p| **p == *ident) {
            return syn::Error::new_spanned(
                ident,
                format!("full_copy expects a type parameter, but `{ident}` is a const parameter"),
            )
            .to_compile_error()
            .into();
        } else {
            return syn::Error::new_spanned(
                ident,
                format!(
                    "`{ident}` is not a type parameter of `{}`",
                    derive_input.ident
                ),
            )
            .to_compile_error()
            .into();
        }
    }

    // Validate the type-level #[epserde(phantom(...))] attribute and build the
    // set of substitutable parameters: the declared type parameters minus the
    // phantom ones, which must be left completely untouched.
    let mut repl_params = type_params.clone();
    for ident in &attrs.phantom_params {
        if attrs.is_zero_copy {
            return syn::Error::new_spanned(
                ident,
                "phantom(...) cannot be used on a zero-copy type, as it performs no substitution",
            )
            .to_compile_error()
            .into();
        }
        if let Some(decl) = type_params.iter().copied().find(|p| **p == *ident) {
            if forced_params.contains(decl) {
                return syn::Error::new_spanned(
                    ident,
                    format!("`{ident}` cannot be listed both in phantom(...) and full_copy(...)"),
                )
                .to_compile_error()
                .into();
            }
            repl_params.remove(decl);
        } else if const_params.iter().any(|p| **p == *ident) {
            return syn::Error::new_spanned(
                ident,
                format!("phantom expects a type parameter, but `{ident}` is a const parameter"),
            )
            .to_compile_error()
            .into();
        } else {
            return syn::Error::new_spanned(
                ident,
                format!(
                    "`{ident}` is not a type parameter of `{}`",
                    derive_input.ident
                ),
            )
            .to_compile_error()
            .into();
        }
    }

    let ctx = EpserdeContext {
        derive_input: &derive_input,
        type_const_params,
        repl_params,
        forced_params,
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
    out.extend(type_info_derive_impl(
        &derive_input,
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

        // Hash in the fully qualified struct name (module path + name),
        // so that two structs with the same short name in different modules
        // do not collide.
        Hash::hash(::core::module_path!(), hasher);
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

    // Process each variant
    for variant in &e.variants {
        let ident = &variant.ident;
        let mut type_hash = quote! { Hash::hash(stringify!(#ident), hasher); };
        // For zero-copy enums the discriminant is stored verbatim in the
        // serialized bytes (they are (de)serialized as raw memory), so
        // re-numbering variants changes the encoding. We hash any explicit
        // discriminant so such a change is detected as a type-hash mismatch
        // rather than silently mis-decoding. Deep-copy enums instead write a
        // positional tag, so their discriminant values are irrelevant and are
        // deliberately not hashed (hashing them would break compatibility with
        // data serialized before an unrelated discriminant edit). Hashing only
        // explicit discriminants keeps enums with purely implicit discriminants
        // backward-compatible; two distinct value mappings cannot collide, as
        // identical explicit tokens at identical positions imply identical
        // resolved values.
        if ctx.is_zero_copy {
            if let Some((_, discriminant)) = &variant.discriminant {
                type_hash.extend([quote! {
                    Hash::hash("=", hasher);
                    Hash::hash(stringify!(#discriminant), hasher);
                }]);
            }
        }
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
        let mut align_to = ::core::mem::align_of::<Self>();
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
/// It generates implementations just for the traits `TypeHash` and `AlignHash`
/// (plus `AlignTo` for zero-copy types), but not for `CopyType`, `SerInner`, or
/// `DeserInner`. See the documentation of [`Epserde`] for more information.
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
    let (_, _type_params, const_params) = match get_type_const_params(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    emit_deprecation_warnings(&attrs, &derive_input.ident);

    type_info_derive_impl(
        &derive_input,
        const_params,
        generics_for_impl,
        generics_for_type,
        where_clause,
        attrs.is_zero_copy,
    )
}

/// Completes the [`TypeInfo`] derive macro using precomputed data.
///
/// This function is used by the [`Epserde`] derive macro to
/// avoid recomputing the same data twice.
fn type_info_derive_impl(
    derive_input: &DeriveInput,
    const_params: Vec<&syn::Ident>,
    generics_for_impl: ImplGenerics<'_>,
    generics_for_type: TypeGenerics<'_>,
    where_clause: &WhereClause,
    is_zero_copy: bool,
) -> proc_macro::TokenStream {
    // Add reprs, normalized and sorted (order does not matter)
    let repr_attrs = match repr_hints(&derive_input.attrs) {
        Ok(repr_attrs) => repr_attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    let ctx = TypeInfoContext {
        name: &derive_input.ident,
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
