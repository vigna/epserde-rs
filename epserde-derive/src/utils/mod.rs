/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Small [`syn`]-level utilities shared by the derive macros.

use quote::ToTokens;
use std::collections::HashSet;
use syn::{DeriveInput, WhereClause, punctuated::Punctuated, spanned::Spanned, token};

/// Returns an empty where clause.
pub(crate) fn empty_where_clause() -> WhereClause {
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
pub(crate) fn get_field_name(field: &syn::Field, field_idx: usize) -> proc_macro2::TokenStream {
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
pub(crate) fn type_diag_span(ty: &syn::Type) -> proc_macro2::Span {
    if let syn::Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            return last.ident.span();
        }
    }
    ty.span()
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
pub(crate) fn get_type_const_params(
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
