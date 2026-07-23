/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Parsing of the `#[epserde(...)]` and `#[repr(...)]` attributes.

use quote::ToTokens;
use syn::{DeriveInput, WherePredicate, punctuated::Punctuated, token};

/// Returns true if the given field carries `#[epserde(force_full_copy)]`.
pub(crate) fn is_force_full_copy(field: &syn::Field) -> bool {
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

/// Parsed epserde attributes.
pub(crate) struct EpserdeAttrs {
    /// Whether the type has `#[repr(C)]`.
    pub(crate) is_repr_c: bool,
    /// Whether `#[epserde(zero_copy)]` was specified.
    pub(crate) is_zero_copy: bool,
    /// Whether `#[epserde(deep_copy)]` was specified.
    pub(crate) is_deep_copy: bool,
    /// Additional where-clause predicates for `DeserInner` impl.
    pub(crate) deser_bounds: Vec<WherePredicate>,
    /// Additional where-clause predicates for `SerInner` impl.
    pub(crate) ser_bounds: Vec<WherePredicate>,
    /// Type-parameter idents listed in `#[epserde(full_copy(...))]`. These are
    /// pinned to full-copy: removed from the `DeserType` substitution set, and
    /// kept verbatim in `DeserType<'a>`.
    pub(crate) full_copy_params: Vec<syn::Ident>,
    /// Type-parameter idents listed in `#[epserde(phantom(...))]`. These are
    /// declared phantom throughout the type and left completely untouched: no
    /// `SerType`/`DeserType` substitution and no `SerInner`/`DeserInner`
    /// bounds.
    pub(crate) phantom_params: Vec<syn::Ident>,
}

/// Collects the representation hints of all `repr` attributes of a type,
/// individually normalized (e.g., `align(16)`) and sorted.
///
/// The normalization guarantees that equivalent spellings such as
/// `#[repr(C, align(16))]` and `#[repr(align(16))] #[repr(C)]` yield the same
/// hints, and thus the same alignment hash.
pub(crate) fn repr_hints(attrs: &[syn::Attribute]) -> syn::Result<Vec<String>> {
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

/// Returns an error for the first `#[epserde(...)]` attribute in `attrs`.
///
/// Used to reject attributes in positions where they are syntactically
/// accepted (the derives register the `epserde` helper attribute, so the
/// compiler forwards them anywhere on the item) but have no effect, instead
/// of silently ignoring them.
pub(crate) fn reject_epserde_attrs(attrs: &[syn::Attribute], msg: &str) -> syn::Result<()> {
    for attr in attrs {
        if attr.meta.path().is_ident("epserde") {
            return Err(syn::Error::new_spanned(attr, msg));
        }
    }
    Ok(())
}

/// Parses the string value of a `bound(deser = "...")` or `bound(ser = "...")`
/// key into where-clause predicates, extending `out`.
fn parse_bound_predicates(
    inner: &syn::meta::ParseNestedMeta,
    out: &mut Vec<WherePredicate>,
) -> syn::Result<()> {
    let value = inner.value()?;
    let lit: syn::LitStr = value.parse()?;
    let preds = lit.parse_with(Punctuated::<WherePredicate, token::Comma>::parse_terminated)?;
    out.extend(preds);
    Ok(())
}

/// Parses the parenthesized type-parameter list of a type-level attribute such
/// as `full_copy(T, U)` or `phantom(T, U)`, extending `out` with the listed
/// identifiers.
fn parse_param_list(
    meta: &syn::meta::ParseNestedMeta,
    attr_name: &str,
    out: &mut Vec<syn::Ident>,
) -> syn::Result<()> {
    if !meta.input.peek(token::Paren) {
        return Err(meta.error(format!(
            "\"{attr_name}\" is a type-level attribute and requires a parenthesized \
             list of type parameters, e.g. #[epserde({attr_name}(T))]"
        )));
    }
    meta.parse_nested_meta(|inner| {
        if let Some(ident) = inner.path.get_ident() {
            out.push(ident.clone());
            Ok(())
        } else {
            Err(inner.error("expected a type-parameter identifier"))
        }
    })
}

/// Parses `#[epserde(...)]` attributes.
pub(crate) fn parse_epserde_attrs(input: &DeriveInput) -> syn::Result<EpserdeAttrs> {
    let is_repr_c = repr_hints(&input.attrs)?.iter().any(|hint| hint == "C");

    let mut is_zero_copy = false;
    let mut is_deep_copy = false;
    let mut deser_bounds = Vec::new();
    let mut ser_bounds = Vec::new();
    let mut full_copy_params = Vec::new();
    let mut phantom_params = Vec::new();

    for attr in &input.attrs {
        if attr.meta.path().is_ident("epserde") {
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
                            parse_bound_predicates(&inner, &mut deser_bounds)
                        } else if inner.path.is_ident("ser") {
                            parse_bound_predicates(&inner, &mut ser_bounds)
                        } else {
                            Err(inner.error("expected `deser` or `ser`"))
                        }
                    })
                } else if meta.path.is_ident("full_copy") {
                    parse_param_list(&meta, "full_copy", &mut full_copy_params)
                } else if meta.path.is_ident("phantom") {
                    parse_param_list(&meta, "phantom", &mut phantom_params)
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
    })
}
