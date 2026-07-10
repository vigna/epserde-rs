/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Generation of the where clauses and substituted generics of the derived
//! impls.

use quote::quote;
use std::collections::HashSet;
use syn::{
    BoundLifetimes, DeriveInput, GenericParam, LifetimeParam, PredicateType, TypeParamBound,
    WhereClause, WherePredicate,
    punctuated::Punctuated,
    token::{self, Plus},
};

use super::EpserdeContext;
use crate::utils::empty_where_clause;

/// For each bounded type parameter that is substituted in an associated
/// (de)serialization type, bounds that substituted form with the same trait
/// bounds as the parameter.
///
/// The two substitution sets differ: `SerType` substitutes every [replaceable]
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
/// * `params` - All [replaceable] parameters; for each, a `SerType<T>` bound is
///   added to `ser_where_clause`.
///
/// * `eps_params` - All ε-copy type parameters appearing at a variable
///   position; for each, a `DeserType<'a, T>` bound is added to
///   `deser_where_clause`.
///
/// * `ser_where_clause` - The `SerInner` where clause, extended in place.
///
/// * `deser_where_clause` - The `DeserInner` where clause, extended in place.
///
/// [replaceable]: crate::epserde::classify::collect_repl_param_occs
pub(crate) fn bound_ser_deser_types(
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
                        "'__ඞඞඞepserdeඞඞඞ_desertype",
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
                            ::epserde::deser::DeserType<'__ඞඞඞepserdeඞඞඞ_desertype, #ident>
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
pub(crate) fn add_ser_deser_trait_bounds(
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
pub(crate) fn gen_generics_for_deser_type(
    ctx: &EpserdeContext,
    eps_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if eps_params.contains(ident) {
                quote!(::epserde::deser::DeserType<'__ඞඞඞepserdeඞඞඞ_desertype, #ident>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates generics for the serialization type by replacing every
/// [replaceable] parameter with its associated serialization type.
///
/// [replaceable]: crate::epserde::classify::collect_repl_param_occs
pub(crate) fn gen_generics_for_ser_type(
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
pub(crate) fn gen_ser_deser_where_clauses(
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
