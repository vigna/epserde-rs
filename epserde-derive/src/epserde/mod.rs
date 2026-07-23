/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Implementation of the [`Epserde`] derive macro.
//!
//! [`Epserde`]: derive@crate::Epserde

pub(crate) mod bounds;
pub(crate) mod checks;
pub(crate) mod classify;
pub(crate) mod enums;
pub(crate) mod helpers;
pub(crate) mod structs;

use std::collections::HashSet;
use syn::{
    Data, DeriveInput, ImplGenerics, TypeGenerics, WhereClause, WherePredicate, parse_macro_input,
};

use crate::attrs::{parse_epserde_attrs, reject_epserde_attrs};
use crate::type_info::type_info_derive_impl;
use crate::utils::get_type_const_params;
use enums::gen_epserde_enum_impl;
use structs::gen_epserde_struct_impl;

/// Context structure for the [`Epserde`] derive macro.
///
/// [`Epserde`]: derive@crate::Epserde
pub(crate) struct EpserdeContext<'a> {
    /// The original derive input.
    pub(crate) derive_input: &'a DeriveInput,
    /// Identifiers of type and const parameters, in order of appearance.
    pub(crate) type_const_params: Vec<&'a syn::Ident>,
    /// Type parameters eligible for substitution: the declared type
    /// parameters minus those declared phantom by the type-level
    /// `#[epserde(phantom(...))]` attribute. The [replaceable]-parameter walk
    /// matches against this set only, so phantom parameters are left
    /// completely untouched (no substitution, no bounds).
    ///
    /// [replaceable]: crate::epserde::classify::collect_repl_param_occs
    pub(crate) repl_params: HashSet<&'a syn::Ident>,
    /// Type parameters pinned to full-copy deserialization by the type-level
    /// `#[epserde(full_copy(...))]` attribute, as a subset of the declared type
    /// parameters.
    pub(crate) forced_params: HashSet<&'a syn::Ident>,
    /// Generics for the `impl` clause as returned by [`split_for_impl`].
    ///
    /// [`split_for_impl`]: syn::Generics::split_for_impl
    pub(crate) generics_for_impl: ImplGenerics<'a>,
    /// Generics for the type as returned by [`split_for_impl`].
    ///
    /// [`split_for_impl`]: syn::Generics::split_for_impl
    pub(crate) generics_for_type: TypeGenerics<'a>,
    /// The where clause for the type being derived.
    pub(crate) where_clause: &'a WhereClause,
    /// Whether the type has `#[repr(C)]`
    pub(crate) is_repr_c: bool,
    /// Whether the type has `#[epserde(zero_copy)]`
    pub(crate) is_zero_copy: bool,
    /// Whether the type has `#[epserde(deep_copy)]`
    pub(crate) is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl from
    /// `#[epserde(bound(deser = "..."))]`.
    pub(crate) deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl from
    /// `#[epserde(bound(ser = "..."))]`.
    pub(crate) ser_bounds: Vec<WherePredicate>,
}

/// Resolves the type-parameter identifiers listed in a type-level attribute
/// (`full_copy(...)` or `phantom(...)`) against the declared parameters of the
/// type.
///
/// Returns, for each listed identifier, the corresponding declared parameter
/// paired with the listed occurrence, which carries the span diagnostics
/// should point at. Errors on a zero-copy type (with the given
/// `zero_copy_error` reason), on a const parameter, and on an identifier that
/// is not a declared type parameter.
fn resolve_param_list<'a, 'b>(
    listed: &'b [syn::Ident],
    attr_name: &str,
    is_zero_copy: bool,
    zero_copy_error: &str,
    type_params: &HashSet<&'a syn::Ident>,
    const_params: &[&'a syn::ConstParam],
    type_ident: &syn::Ident,
) -> syn::Result<Vec<(&'a syn::Ident, &'b syn::Ident)>> {
    let mut out = Vec::new();
    for ident in listed {
        if is_zero_copy {
            return Err(syn::Error::new_spanned(ident, zero_copy_error));
        }
        if let Some(decl) = type_params.iter().copied().find(|p| **p == *ident) {
            out.push((decl, ident));
        } else if const_params.iter().any(|p| p.ident == *ident) {
            return Err(syn::Error::new_spanned(
                ident,
                format!("{attr_name} expects a type parameter, but `{ident}` is a const parameter"),
            ));
        } else {
            return Err(syn::Error::new_spanned(
                ident,
                format!("`{ident}` is not a type parameter of `{type_ident}`"),
            ));
        }
    }
    Ok(out)
}

/// Implements the [`Epserde`] derive macro.
///
/// [`Epserde`]: derive@crate::Epserde
pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
        Data::Enum(e) => e.variants.iter().try_for_each(|v| {
            reject_epserde_attrs(
                &v.attrs,
                "#[epserde(...)] attributes are not supported on enum variants; \
                 place field-level markers such as force_full_copy on the fields of the variant",
            )?;
            validate_fields(&v.fields)
        }),
        Data::Union(_) => Ok(()),
    } {
        return e.to_compile_error().into();
    }

    // Validate the type-level #[epserde(full_copy(...))] attribute and build
    // the set of forced parameters, referencing the declared type parameters.
    let mut forced_params: HashSet<&syn::Ident> = HashSet::new();
    match resolve_param_list(
        &attrs.full_copy_params,
        "full_copy",
        attrs.is_zero_copy,
        "full_copy(...) cannot be used on a zero-copy type, as its deserialization type is a reference",
        &type_params,
        &const_params,
        &derive_input.ident,
    ) {
        Ok(resolved) => forced_params.extend(resolved.into_iter().map(|(decl, _)| decl)),
        Err(e) => return e.to_compile_error().into(),
    }

    // Validate the type-level #[epserde(phantom(...))] attribute and build the
    // set of substitutable parameters: the declared type parameters minus the
    // phantom ones, which must be left completely untouched.
    let mut repl_params = type_params.clone();
    match resolve_param_list(
        &attrs.phantom_params,
        "phantom",
        attrs.is_zero_copy,
        "phantom(...) cannot be used on a zero-copy type, as it performs no substitution",
        &type_params,
        &const_params,
        &derive_input.ident,
    ) {
        Ok(resolved) => {
            for (decl, ident) in resolved {
                if forced_params.contains(decl) {
                    return syn::Error::new_spanned(
                        ident,
                        format!(
                            "`{ident}` cannot be listed both in phantom(...) and full_copy(...)"
                        ),
                    )
                    .to_compile_error()
                    .into();
                }
                repl_params.remove(decl);
            }
        }
        Err(e) => return e.to_compile_error().into(),
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
