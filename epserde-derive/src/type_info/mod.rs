/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Implementation of the [`TypeInfo`] derive macro.
//!
//! [`TypeInfo`]: derive@crate::TypeInfo

pub(crate) mod enums;
pub(crate) mod structs;

use quote::quote;
use syn::{
    Data, DeriveInput, ImplGenerics, PredicateType, TypeGenerics, TypeParamBound, WhereClause,
    WherePredicate, parse_macro_input,
    punctuated::Punctuated,
    token::{self, Plus},
};

use crate::attrs::{parse_epserde_attrs, reject_epserde_attrs, repr_hints};
use crate::utils::get_type_const_params;
use enums::gen_enum_type_info_impl;
use structs::gen_struct_type_info_impl;

/// Context structure for the [`TypeInfo`] derive macro.
///
/// [`TypeInfo`]: derive@crate::TypeInfo
pub(crate) struct TypeInfoContext<'a> {
    /// The name of the type
    pub(crate) name: &'a syn::Ident,
    /// Const parameters, in order of appearance. Both the identifier and the
    /// type are hashed by the `TypeHash` implementation.
    pub(crate) const_params: Vec<&'a syn::ConstParam>,
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
    /// Whether the type is zero-copy
    pub(crate) is_zero_copy: bool,
    /// `repr` attributes
    pub(crate) repr_attrs: Vec<String>,
}

/// Generates the `TypeHash` implementation body.
pub(crate) fn gen_type_hash_body(
    ctx: &TypeInfoContext,
    field_hashes: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let copy_type = if ctx.is_zero_copy {
        "ZeroCopy"
    } else {
        "DeepCopy"
    };
    let name = &ctx.name;
    let const_idents = ctx
        .const_params
        .iter()
        .map(|c| &c.ident)
        .collect::<Vec<_>>();
    let const_types = ctx.const_params.iter().map(|c| &c.ty).collect::<Vec<_>>();
    // Zero-copy field hashes use bare field types, so the import would be
    // dead there.
    let ser_type_import = if ctx.is_zero_copy {
        quote! {}
    } else {
        quote! { use ::epserde::ser::SerType; }
    };

    quote! {
        use ::core::hash::Hash;
        use ::epserde::traits::TypeHash;
        #ser_type_import

        // Hash in copy type
        Hash::hash(#copy_type, hasher);

        // Hash in the name, type, and value of each const parameter. The type
        // is hashed so that parameters differing only in type (for instance
        // const N: u8 versus const N: u32) do not collide. The value is
        // preceded by the size of its type, which length-delimits it: values
        // are written in native endianness with no intrinsic length, so
        // without a delimiter the concatenated values of several parameters
        // would form an ambiguous byte stream.
        #(
            Hash::hash(stringify!(#const_idents), hasher);
            <#const_types as TypeHash>::type_hash(hasher);
            Hash::hash(&::core::mem::size_of::<#const_types>(), hasher);
            Hash::hash(&#const_idents, hasher);
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

/// Generates the where clauses for `TypeHash`, `AlignHash`, and `PadTo`.
///
/// The where clauses bound with the trait being implemented; the bound is
/// applied to the field types for zero-copy types, and to the associated
/// serialization types of field types for deep-copy types,
pub(crate) fn gen_type_info_where_clauses(
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

    let mut bound_pad_to = Punctuated::new();
    bound_pad_to.push(syn::parse_quote!(::epserde::traits::PadTo));
    let pad_to = gen_type_info_where_clause(bound_pad_to);

    (type_hash, align_hash, pad_to)
}

/// Generates the implementations for `TypeHash`, `AlignHash`, and
/// optionally `PadTo`.
pub(crate) fn gen_type_info_traits(
    ctx: TypeInfoContext,
    type_hash_where_clause: syn::WhereClause,
    align_hash_where_clause: syn::WhereClause,
    pad_to_where_clause: syn::WhereClause,
    type_hash_body: proc_macro2::TokenStream,
    align_hash_body: proc_macro2::TokenStream,
    pad_to_body: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let name = &ctx.name;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;

    let pad_to_impl = if let Some(pad_to_body) = pad_to_body {
        quote! {
            #[automatically_derived]
            impl #generics_for_impl ::epserde::traits::PadTo for #name #generics_for_type #pad_to_where_clause {
                fn pad_to() -> usize {
                    #pad_to_body
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

        #pad_to_impl
    }
}

/// Implements the [`TypeInfo`] derive macro.
///
/// [`TypeInfo`]: derive@crate::TypeInfo
pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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

    if let Some(ident) = attrs.full_copy_params.first() {
        return syn::Error::new_spanned(
            ident,
            "\"full_copy\" has no effect on derive(TypeInfo); it only affects the \
             deserialization type generated by derive(Epserde)",
        )
        .to_compile_error()
        .into();
    }
    if let Some(ident) = attrs.phantom_params.first() {
        return syn::Error::new_spanned(
            ident,
            "\"phantom\" has no effect on derive(TypeInfo); it only affects the \
             deserialization type generated by derive(Epserde)",
        )
        .to_compile_error()
        .into();
    }
    if let Some(pred) = attrs.deser_bounds.first().or(attrs.ser_bounds.first()) {
        return syn::Error::new_spanned(
            pred,
            "\"bound\" has no effect on derive(TypeInfo); it only affects the \
             impls generated by derive(Epserde)",
        )
        .to_compile_error()
        .into();
    }
    const FIELD_ATTR_MSG: &str = "field-level #[epserde(...)] attributes have no effect on \
         derive(TypeInfo); they only affect derive(Epserde)";
    const VARIANT_ATTR_MSG: &str = "#[epserde(...)] attributes are not supported on enum \
         variants";
    if let Err(e) = match &derive_input.data {
        Data::Struct(s) => s
            .fields
            .iter()
            .try_for_each(|f| reject_epserde_attrs(&f.attrs, FIELD_ATTR_MSG)),
        Data::Enum(e) => e.variants.iter().try_for_each(|v| {
            reject_epserde_attrs(&v.attrs, VARIANT_ATTR_MSG)?;
            v.fields
                .iter()
                .try_for_each(|f| reject_epserde_attrs(&f.attrs, FIELD_ATTR_MSG))
        }),
        // Unions are rejected as unsupported in type_info_derive_impl.
        Data::Union(_) => Ok(()),
    } {
        return e.to_compile_error().into();
    }
    let (_, _type_params, const_params) = match get_type_const_params(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

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
///
/// [`Epserde`]: derive@crate::Epserde
/// [`TypeInfo`]: derive@crate::TypeInfo
pub(crate) fn type_info_derive_impl(
    derive_input: &DeriveInput,
    const_params: Vec<&syn::ConstParam>,
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
