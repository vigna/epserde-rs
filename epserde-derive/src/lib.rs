/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Derive procedural macros for the [`epserde`](https://crates.io/crates/epserde) crate.

use quote::{quote, ToTokens};
use std::{collections::HashSet, vec};
use syn::{
    parse_macro_input,
    punctuated::Punctuated,
    token::{self, Plus},
    BoundLifetimes, Data, DeriveInput, GenericParam, ImplGenerics, LifetimeParam, PredicateType,
    TypeGenerics, TypeParamBound, WhereClause, WherePredicate,
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

/// Adds a trait bound for a type to a where clause.
fn add_trait_bound(where_clause: &mut WhereClause, ty: &syn::Type, trait_path: syn::Path) {
    let mut bounds = Punctuated::new();
    bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
        paren_token: None,
        modifier: syn::TraitBoundModifier::None,
        lifetimes: None,
        path: trait_path,
    }));

    where_clause
        .predicates
        .push(WherePredicate::Type(PredicateType {
            lifetimes: None,
            bounded_ty: ty.clone(),
            colon_token: token::Colon::default(),
            bounds,
        }));
}

/// Returns a field name as a token stream.
///
/// This method takes care transparently of unnamed fields (i.e., fields tuple
/// structs), and for this reason it can only return a `TokenStream` instead of
/// a more specific type such as `Ident`.
fn get_field_name(field: &syn::Field, field_idx: usize) -> proc_macro2::TokenStream {
    field
        .ident
        .to_owned()
        .map(|x| x.to_token_stream())
        .unwrap_or_else(|| syn::Index::from(field_idx).to_token_stream())
}

/// Returns true if the given type is just given by the given identifier.
fn type_equals_ident(ty: &syn::Type, ident: &syn::Ident) -> bool {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    {
        if segments.len() == 1 && segments[0].ident == *ident {
            return true;
        }
    }

    false
}

/// Generates a method call for field deserialization.
///
/// This methods takes care of choosing `_deserialize_eps_inner` or
/// `_deserialize_full_inner` depending on whether the field type is a generic
/// type or not, and to use the special method `_deserialize_eps_inner_special`
/// for `PhantomDeserData`.
///
/// The type of `field_name` is `TokenStream` because it can be either an
/// identifier (for named fields) or an index (for unnamed fields).
fn gen_method_call(
    field_name: &proc_macro2::TokenStream,
    ty: &syn::Type,
    field_type_params: &HashSet<&syn::Ident>,
) -> proc_macro2::TokenStream {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    {
        // This is a pretty weak check, as a user could define its own PhantomDeserData,
        // but it should be good enough in practice
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: <#ty>::_deserialize_eps_inner_special);
            }
        }

        // It's just a type, and it's the type of a field: ε-copy deserialization
        if segments.len() == 1 && field_type_params.contains(&segments[0].ident) {
            return syn::parse_quote!(#field_name: <#ty as DeserializeInner>::_deserialize_eps_inner);
        }
    }

    syn::parse_quote!(#field_name: <#ty as DeserializeInner>::_deserialize_full_inner)
}

/// Generates the `IS_ZERO_COPY` expression.
fn gen_is_zero_copy_expr(is_repr_c: bool, fields_types: &[&syn::Type]) -> proc_macro2::TokenStream {
    if fields_types.is_empty() {
        quote!(#is_repr_c)
    } else {
        quote!(#is_repr_c #(&& <#fields_types>::IS_ZERO_COPY)*)
    }
}

/// Returns the identifiers of type and const parameters in order of appearance,
/// and the identifiers of const parameters only, also in order of appearance.
fn get_type_const_params(input: &DeriveInput) -> (Vec<syn::Ident>, Vec<syn::Ident>) {
    let mut type_const_params = vec![];
    let mut const_params = vec![];

    input.generics.params.iter().for_each(|x| {
        match x {
            syn::GenericParam::Type(t) => {
                type_const_params.push(t.ident.clone());
            }
            syn::GenericParam::Const(c) => {
                const_params.push(c.ident.clone());
                type_const_params.push(c.ident.clone());
            }
            syn::GenericParam::Lifetime(_) => {}
        };
    });

    (type_const_params, const_params)
}

/// Returns whether the struct has attributes `repr(C)`, `zero_copy`, and `deep_copy`.
///
/// # Panics
///
/// This method will panic if coherence checks fail (e.g., to be `zero_copy` the
/// struct must be `repr(C)`)
fn check_attrs(input: &DeriveInput) -> (bool, bool, bool) {
    let is_repr_c = input.attrs.iter().any(|x| {
        x.meta.path().is_ident("repr") && x.meta.require_list().unwrap().tokens.to_string() == "C"
    });
    let is_zero_copy = input
        .attrs
        .iter()
        .any(|x| x.meta.path().is_ident("zero_copy"));
    let is_deep_copy = input
        .attrs
        .iter()
        .any(|x| x.meta.path().is_ident("deep_copy"));
    if is_zero_copy && !is_repr_c {
        panic!(
            "Type {} is declared as zero copy, but it is not repr(C)",
            input.ident
        );
    }
    if is_zero_copy && is_deep_copy {
        panic!(
            "Type {} is declared as both zero copy and deep copy",
            input.ident
        );
    }

    (is_repr_c, is_zero_copy, is_deep_copy)
}

/// Adds trait bounds for associated (de)serialization types based on bounds on
/// type parameters that are the type of some fields.
fn add_ser_deser_bounds<'a>(
    derive_input: &'a DeriveInput,
    field_type_params: &HashSet<&syn::Ident>,
    where_clause_ser: &mut WhereClause,
    where_clause_des: &mut WhereClause,
) {
    // If there are bounded type parameters which are fields of the struct, we
    // need to impose the same bounds on the associated SerType/DeserType.
    for param in &derive_input.generics.params {
        if let syn::GenericParam::Type(t) = param {
            let ident = &t.ident;

            // We are just interested in type parameters that are types of
            // fields and that have trait bounds
            if !t.bounds.is_empty() && field_type_params.contains(ident) {
                // The lifetime of the DeserType
                let mut lifetimes = Punctuated::new();
                lifetimes.push(GenericParam::Lifetime(LifetimeParam {
                    attrs: vec![],
                    lifetime: syn::Lifetime::new(
                        "'epserde_desertype",
                        proc_macro2::Span::call_site(),
                    ),
                    colon_token: None,
                    bounds: Punctuated::new(),
                }));

                // Add the trait bounds of the type to the DeserType
                where_clause_des
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: Some(BoundLifetimes {
                            for_token: token::For::default(),
                            lt_token: token::Lt::default(),
                            lifetimes,
                            gt_token: token::Gt::default(),
                        }),
                        bounded_ty: syn::parse_quote!(
                            <#ident as ::epserde::deser::DeserializeInner>::DeserType<'epserde_desertype>
                        ),
                        colon_token: token::Colon::default(),
                        bounds: t.bounds.clone(),
                    }));

                // Add the trait bounds of the type to the SerType
                where_clause_ser
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: syn::parse_quote!(
                            <#ident as ::epserde::ser::SerializeInner>::SerType
                        ),
                        colon_token: token::Colon::default(),
                        bounds: t.bounds.clone(),
                    }));
            }
        }
    }
}

/// Adds SerializeInner and DeserializeInner trait bounds to a type
/// in the serialization/deserialization where clauses.
fn add_ser_deser_trait_bounds(
    where_clause_ser: &mut syn::WhereClause,
    where_clause_des: &mut syn::WhereClause,
    ty: &syn::Type,
) {
    add_trait_bound(
        where_clause_ser,
        ty,
        syn::parse_quote!(::epserde::ser::SerializeInner),
    );
    add_trait_bound(
        where_clause_des,
        ty,
        syn::parse_quote!(::epserde::deser::DeserializeInner),
    );
}

/// Generates deserialization type generics by replacing type parameters
/// that are types of fields with their associated DeserType.
fn gen_deser_type_generics<'a>(
    ctx: &EpserdeContext,
    field_type_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if field_type_params.contains(ident)
            {
                quote!(<#ident as ::epserde::deser::DeserializeInner>::DeserType<'epserde_desertype>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates serialization type generics by replacing type parameters
/// that are types of fields with their associated SerType.
fn gen_ser_type_generics<'a>(
    ctx: &EpserdeContext,
    field_type_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if field_type_params.contains(ident) {
                quote!(<#ident as ::epserde::ser::SerializeInner>::SerType)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates where clauses for SerializeInner and DeserializeInner
/// implementation.
///
/// The where clauses bound all field types with the trait being implemented,
/// thus propagating recursively (de)serializability.
fn gen_where_clauses(field_types: &[&syn::Type]) -> (WhereClause, WhereClause) {
    let mut where_clause_ser = empty_where_clause();
    let mut where_clause_des = empty_where_clause();

    // Add trait bounds for all field types
    for ty in field_types {
        add_ser_deser_trait_bounds(&mut where_clause_ser, &mut where_clause_des, ty);
    }

    (where_clause_ser, where_clause_des)
}

/// Set of where clauses for traits handled by the [`TypeInfo`] derive macro.
struct TypeInfoWhereClauses {
    /// Where clause for `TypeHash` trait.
    type_hash: WhereClause,
    /// Where clause for `AlignHash` trait.
    align_hash: WhereClause,
    /// Where clause for `MaxSizeOf` trait.
    max_size_of: WhereClause,
}

/// Generates all clauses in [`TypeInfoWhereClauses`].
fn gen_type_info_where_clauses(
    base_clause: &WhereClause,
    field_types: &[&syn::Type],
) -> TypeInfoWhereClauses {
    /// Generates one of the clauses in [`TypeInfoWhereClauses`]
    /// by adding the given trait bound for all types of fields.
    fn gen_type_info_where_clause(
        base_clause: &WhereClause,
        field_types: &[&syn::Type],
        trait_bound: Punctuated<TypeParamBound, Plus>,
    ) -> WhereClause {
        let mut where_clause = base_clause.clone();
        for &ty in field_types {
            where_clause
                .predicates
                .push(WherePredicate::Type(PredicateType {
                    lifetimes: None,
                    bounded_ty: ty.clone(),
                    colon_token: token::Colon::default(),
                    bounds: trait_bound.clone(),
                }));
        }

        where_clause
    }
    let mut bound_type_hash = Punctuated::new();
    bound_type_hash.push(syn::parse_quote!(::epserde::traits::TypeHash));
    let type_hash = gen_type_info_where_clause(base_clause, field_types, bound_type_hash);

    let mut bound_align_hash = Punctuated::new();
    bound_align_hash.push(syn::parse_quote!(::epserde::traits::AlignHash));
    let align_hash = gen_type_info_where_clause(base_clause, field_types, bound_align_hash);

    let mut bound_max_size_of = Punctuated::new();
    bound_max_size_of.push(syn::parse_quote!(::epserde::traits::MaxSizeOf));
    let max_size_of = gen_type_info_where_clause(base_clause, field_types, bound_max_size_of);

    TypeInfoWhereClauses {
        type_hash,
        align_hash,
        max_size_of,
    }
}

/// Context structure for the [`Epserde`] derive macro.
struct EpserdeContext<'a> {
    /// The original derive input.
    derive_input: &'a DeriveInput,
    /// The name of the type being derived
    name: syn::Ident,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_params: Vec<syn::Ident>,
    /// Generics for the type as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics: TypeGenerics<'a>,
    /// Generics for the `ìmpl` clause as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    impl_generics: ImplGenerics<'a>,
    /// The where clause for the type being derived.
    where_clause: &'a WhereClause,
    /// Whether the type has `#[repr(C)]`
    is_repr_c: bool,
    /// Whether the type has `#[zero_copy]`
    is_zero_copy: bool,
    /// Whether the type has `#[deep_copy]`
    is_deep_copy: bool,
}

/// Generate implementation for struct types
fn gen_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    let mut fields_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];
    let mut field_type_params = HashSet::new();

    s.fields.iter().enumerate().for_each(|(field_idx, field)| {
        let field_type = &field.ty;
        let field_name = get_field_name(field, field_idx);

        // We look for type parameters that are types of fields
        for id in &ctx.type_const_params {
            if type_equals_ident(field_type, id) {
                field_type_params.insert(id);
                break;
            }
        }

        method_calls.push(gen_method_call(&field_name, field_type, &field_type_params));
        fields_names.push(field_name);
        field_types.push(field_type);
    });

    // Gather deserialization types of fields, as they are necessary to
    // derive the deserialization type.
    let deser_type_generics = gen_deser_type_generics(&ctx, &field_type_params);
    let ser_type_generics = gen_ser_type_generics(&ctx, &field_type_params);

    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &field_types);
    let (mut where_clause_ser, mut where_clause_des) = gen_where_clauses(&field_types);
    let impl_generics = &ctx.impl_generics;
    let generics = &ctx.generics;
    let where_clause = &ctx.where_clause;
    let name = &ctx.name;

    if ctx.is_zero_copy {
        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.
        quote! {
            #[automatically_derived]
            impl #impl_generics ::epserde::traits::CopyType for #name #generics #where_clause {
                type Copy = ::epserde::traits::Zero;
            }

            #[automatically_derived]
            impl #impl_generics ::epserde::ser::SerializeInner for #name #generics #where_clause_ser {
                type SerType = Self;
                // Compute whether the type could be zero copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                // The type is declared as zero copy, so a fortiori there is no mismatch.
                const ZERO_COPY_MISMATCH: bool = false;

                #[inline(always)]
                unsafe fn _serialize_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::traits::ZeroCopy;
                    use ::epserde::ser::helpers;

                    // No-op code that however checks that all fields are zero-copy.
                    fn test<T: ZeroCopy>() {}
                    #(
                        test::<#field_types>();
                    )*
                    helpers::serialize_zero(backend, self)
                }
            }

            #[automatically_derived]
            impl #impl_generics ::epserde::deser::DeserializeInner for #name #generics #where_clause_des
            {
                unsafe fn _deserialize_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::helpers;

                    helpers::deserialize_full_zero::<Self>(backend)
                }

                type DeserType<'epserde_desertype> = &'epserde_desertype Self;

                unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::helpers;

                    helpers::deserialize_eps_zero::<Self>(backend)
                }
            }
        }
    } else {
        add_ser_deser_bounds(
            &ctx.derive_input,
            &field_type_params,
            &mut where_clause_ser,
            &mut where_clause_des,
        );

        let is_deep_copy = ctx.is_deep_copy;

        quote! {
            #[automatically_derived]
            impl #impl_generics ::epserde::traits::CopyType for #name #generics #where_clause {
                type Copy = ::epserde::traits::Deep;
            }

            #[automatically_derived]
            impl #impl_generics ::epserde::ser::SerializeInner for #name #generics #where_clause_ser {
                type SerType = #name<#(#ser_type_generics,)*>;
                // Compute whether the type could be zero copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                // Compute whether the type could be zero copy but it is not declared as such,
                // and the attribute `deep_copy` is missing.
                const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#field_types>::IS_ZERO_COPY)*;

                #[inline(always)]
                unsafe fn _serialize_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::helpers;
                    use ::epserde::ser::WriteWithNames;

                    helpers::check_mismatch::<Self>();
                    #(
                        WriteWithNames::write(backend, stringify!(#fields_names), &self.#fields_names)?;
                    )*
                    Ok(())
                }
            }

            #[automatically_derived]
            impl #impl_generics ::epserde::deser::DeserializeInner for #name #generics #where_clause_des {
                unsafe fn _deserialize_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserializeInner;

                    Ok(#name{
                        #(
                            #fields_names: unsafe { <#field_types as ::epserde::deser::DeserializeInner>::_deserialize_full_inner(backend)? },
                        )*
                    })
                }

                type DeserType<'epserde_desertype> = #name<#(#deser_type_generics,)*>;

                unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserializeInner;

                    Ok(#name{
                        #(
                            #method_calls(backend)?,
                        )*
                    })
                }
            }
        }
    }
}

/// Generate implementation for enum types
fn gen_enum_impl(ctx: &EpserdeContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut variants_names = vec![];
    let mut variants = vec![];
    let mut variant_ser = vec![];
    let mut variant_full_des = vec![];
    let mut variant_eps_des = vec![];
    let mut field_type_params = HashSet::new();
    let mut fields_types = vec![];
    let type_const_params = ctx
        .type_const_params
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    e.variants.iter().enumerate().for_each(|(variant_id, variant)| {
        variants_names.push(variant.ident.to_token_stream());
        match &variant.fields {
        syn::Fields::Unit => {
            variants.push(variant.ident.to_token_stream());
            variant_ser.push(quote! {{
                WriteWithNames::write(backend, "tag", &#variant_id)?;
            }});
            variant_full_des.push(quote! {});
            variant_eps_des.push(quote! {});
        }
        syn::Fields::Named(fields) => {
            let mut var_fields_names = vec![];
            let mut var_fields_types = vec![];
            let mut method_calls: Vec<proc_macro2::TokenStream> = vec![];
            fields
                .named
                .iter()
                .map(|named| (named.ident.as_ref().unwrap(), &named.ty))
                .for_each(|(name, ty)| {
                    for id in &type_const_params {
                        if type_equals_ident(ty, id) {
                            field_type_params.insert(id);
                            break;
                        }
                    }

                    method_calls.push(gen_method_call(&name.to_token_stream(), ty, &field_type_params));

                    var_fields_names.push(name.to_token_stream());
                    var_fields_types.push(ty);
                });
            let ident = variant.ident.clone();
            variants.push(quote! {
                #ident{ #( #var_fields_names, )* }
            });
            fields_types.extend(&var_fields_types);
            variant_ser.push(quote! {
                WriteWithNames::write(backend, "tag", &#variant_id)?;
                #(
                    WriteWithNames::write(backend, stringify!(#var_fields_names), #var_fields_names)?;
                )*
            });
            variant_full_des.push(quote! {
                #(
                    #var_fields_names: unsafe { <#var_fields_types as DeserializeInner>::_deserialize_full_inner(backend)? },
                )*
            });
            variant_eps_des.push(quote! {
                #(
                   #method_calls(backend)?,
                )*
            });
        }
        syn::Fields::Unnamed(fields) => {
            let mut var_fields_names = vec![];
            let mut var_fields_vars = vec![];
            let mut var_fields_types = vec![];
            let mut method_calls: Vec<proc_macro2::TokenStream> = vec![];

            fields
                .unnamed
                .iter()
                .enumerate()
                .for_each(|(field_idx, unnamed)| {
                    let ty = &unnamed.ty;
                    let name = syn::Index::from(field_idx);
                    for id in &type_const_params {
                        if type_equals_ident(ty, id) {
                            field_type_params.insert(&id);
                            break;
                        }
                    }

                    var_fields_names.push(syn::Ident::new(
                        &format!("v{}", field_idx),
                        proc_macro2::Span::call_site(),
                    )
                    .to_token_stream());

                    method_calls.push(gen_method_call(&name.to_token_stream(), ty, &field_type_params));

                    var_fields_vars.push(syn::Index::from(field_idx));
                    var_fields_types.push(ty);
                });

            let ident = variant.ident.clone();
            variants.push(quote! {
                #ident( #( #var_fields_names, )* )
            });
            fields_types.extend(&var_fields_types);

            variant_ser.push(quote! {
                WriteWithNames::write(backend, "tag", &#variant_id)?;
                #(
                    WriteWithNames::write(backend, stringify!(#var_fields_names), #var_fields_names)?;
                )*
            });
            variant_full_des.push(quote! {
                #(
                    #var_fields_vars : unsafe { <#var_fields_types as DeserializeInner>::_deserialize_full_inner(backend)? },
                )*
            });
            variant_eps_des.push(quote! {
                #(
                    #method_calls(backend)?,
                )*
            });
        }
        }
    });

    // Gather deserialization types of fields,
    // which are necessary to derive the deserialization type.
    let deser_type_generics = gen_deser_type_generics(&ctx, &field_type_params);
    let ser_type_generics = gen_ser_type_generics(&ctx, &field_type_params);
    let tag = (0..variants.len()).collect::<Vec<_>>();

    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &fields_types);
    let (mut where_clause_ser, mut where_clause_des) = gen_where_clauses(&fields_types);
    let is_deep_copy = ctx.is_deep_copy;
    let impl_generics = &ctx.impl_generics;
    let generics = &ctx.generics;
    let where_clause = &ctx.where_clause;
    let name = &ctx.name;

    if ctx.is_zero_copy {
        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.

        quote! {
            #[automatically_derived]
            impl #impl_generics ::epserde::traits::CopyType for #name #generics #where_clause {
                type Copy = ::epserde::traits::Zero;
            }
            #[automatically_derived]
            impl #impl_generics ::epserde::ser::SerializeInner for #name #generics #where_clause_ser {
                type SerType = Self;

                // Compute whether the type could be zero copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                // The type is declared as zero copy, so a fortiori there is no mismatch.
                const ZERO_COPY_MISMATCH: bool = false;
                #[inline(always)]
                unsafe fn _serialize_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::traits::ZeroCopy;
                    use ::epserde::ser::helpers;

                    // No-op code that however checks that all fields are zero-copy.
                    fn test<T: ZeroCopy>() {}
                    #(
                        test::<#fields_types>();
                    )*
                    helpers::serialize_zero(backend, self)
                }
            }

            #[automatically_derived]
            impl #impl_generics ::epserde::deser::DeserializeInner for #name #generics #where_clause_des {
                unsafe fn _deserialize_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::helpers;

                    helpers::deserialize_full_zero::<Self>(backend)
                }

                type DeserType<'epserde_desertype> = &'epserde_desertype Self;

                unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::helpers;

                    helpers::deserialize_eps_zero::<Self>(backend)
                }
            }
        }
    } else {
        add_ser_deser_bounds(
            &ctx.derive_input,
            &field_type_params,
            &mut where_clause_ser,
            &mut where_clause_des,
        );

        let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &fields_types);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::epserde::traits::CopyType for #name #generics #where_clause {
                type Copy = ::epserde::traits::Deep;
            }
            #[automatically_derived]

            impl #impl_generics ::epserde::ser::SerializeInner for #name #generics #where_clause_ser {
                type SerType = #name<#(#ser_type_generics,)*>;

                // Compute whether the type could be zero copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                // Compute whether the type could be zero copy but it is not declared as such,
                // and the attribute `deep_copy` is missing.
                const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#fields_types>::IS_ZERO_COPY)*;
                #[inline(always)]
                unsafe fn _serialize_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::helpers;
                    use ::epserde::ser::WriteWithNames;

                    helpers::check_mismatch::<Self>();
                    match self {
                        #(
                           Self::#variants => { #variant_ser }
                        )*
                    }
                    Ok(())
                }
            }
            #[automatically_derived]
            impl #impl_generics ::epserde::deser::DeserializeInner for #name #generics #where_clause_des {
                unsafe fn _deserialize_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserializeInner;
                    use ::epserde::deser::Error;

                    match unsafe { <usize as DeserializeInner>::_deserialize_full_inner(backend)? } {
                        #(
                            #tag => Ok(Self::#variants_names{ #variant_full_des }),
                        )*
                        tag => Err(Error::InvalidTag(tag)),
                    }
                }

                type DeserType<'epserde_desertype> = #name<#(#deser_type_generics,)*>;

                unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserializeInner;
                    use ::epserde::deser::Error;

                    match unsafe { <usize as DeserializeInner>::_deserialize_full_inner(backend)? } {
                        #(
                            #tag => Ok(Self::DeserType::<'_>::#variants_names{ #variant_eps_des }),
                        )*
                        tag => Err(Error::InvalidTag(tag)),
                    }
                }
            }
        }
    }
}

/// Generate an ε-serde implementation for custom types.
///
/// It generates implementations for the traits `CopyType`,
/// `MaxSizeOf`, `TypeHash`, `AlignHash`, `SerializeInner`,
/// and `DeserializeInner`.
///
/// Presently we do not support unions.
///
/// The attribute `zero_copy` can be used to generate an implementation for a zero-copy
/// type, but the type must be `repr(C)` and all fields must be zero-copy.
///
/// If you do not specify `zero_copy`, the macro assumes your structure is deep-copy.
/// However, if you have a structure that could be zero-copy, but has no attribute,
/// a warning will be issued every time you serialize. The warning can be silenced adding
/// the explicit attribute `deep_copy`.
#[proc_macro_derive(Epserde, attributes(zero_copy, deep_copy))]
pub fn epserde_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // This part is in common with type_info_derive
    let mut derive_input = parse_macro_input!(input as DeriveInput);

    if derive_input.generics.where_clause.is_some() {
        panic!("The derive macros do not support where clauses on the original type.");
    }

    derive_input.generics.make_where_clause();
    let (impl_generics, generics, where_clause) = derive_input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let (is_repr_c, is_zero_copy, is_deep_copy) = check_attrs(&derive_input);
    let (type_const_params, const_params) = get_type_const_params(&derive_input);
    let name = derive_input.ident.clone();

    let ctx = EpserdeContext {
        derive_input: &derive_input,
        name,
        type_const_params,
        generics,
        impl_generics,
        where_clause,
        is_repr_c,
        is_zero_copy,
        is_deep_copy,
    };

    let mut out: proc_macro::TokenStream = match &derive_input.data {
        Data::Struct(s) => gen_struct_impl(&ctx, s),
        Data::Enum(e) => gen_enum_impl(&ctx, e),
        _ => todo!("Union types are not currently supported"),
    }
    .into();

    // Automatically derive type info
    out.extend(_type_info_derive(
        &derive_input,
        ctx.type_const_params,
        const_params,
        ctx.generics,
        ctx.impl_generics,
        ctx.where_clause,
        ctx.is_zero_copy,
    ));

    out
}

//
// `TypeInfo` derive macro implementation
//

/// Context structure for the `TypeInfo` derive macro
struct TypeInfoContext<'a> {
    /// The original derive input
    derive_input: &'a DeriveInput,
    /// The name of the type
    name: syn::Ident,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_params: Vec<syn::Ident>,
    /// Identifiers of const parameters, in order of appearance.
    const_params: Vec<syn::Ident>,
    /// Generics for the type as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    generics: TypeGenerics<'a>,
    /// Generics for the `ìmpl` clause as returned by
    /// [`split_for_impl`](syn::Generics::split_for_impl).
    impl_generics: ImplGenerics<'a>,
    /// The where clause for the type being derived.
    where_clause: &'a WhereClause,
    /// Whether the type is zero-copy
    is_zero_copy: bool,
    /// `repr` attributes
    repr: Vec<String>,
}

/// Generate TypeHash implementation body
fn gen_type_info_body(
    ctx: &TypeInfoContext,
    field_hashes: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let copy_type = if ctx.is_zero_copy {
        "ZeroCopy"
    } else {
        "DeepCopy"
    };
    let const_params = &ctx.const_params;
    let name = &ctx.name;

    quote! {
        use ::core::hash::Hash;
        use ::epserde::traits::TypeHash;
        // Hash in copy type
        Hash::hash(#copy_type, hasher);
        // Hash the values of generic constants
        #(
            Hash::hash(&#const_params, hasher);
        )*
        // Hash the identifiers of generic constants
        #(
            Hash::hash(stringify!(#const_params), hasher);
        )*
        // Hash in struct and field names.
        Hash::hash(stringify!(#name), hasher);
        // Hash field information
        #(
            #field_hashes
        )*
    }
}

/// Generate AlignHash implementation body for structs
fn gen_struct_align_hash_body(
    ctx: &TypeInfoContext,
    fields_types: &[&syn::Type],
) -> proc_macro2::TokenStream {
    let repr = &ctx.repr;
    if ctx.is_zero_copy {
        quote! {
            use ::core::hash::Hash;
            use ::core::mem;
            use ::epserde::traits::AlignHash;
            // Hash in size, as padding is given by MaxSizeOf.
            // and it is independent of the architecture.
            Hash::hash(&mem::size_of::<Self>(), hasher);
            // Hash in representation data.
            #(
                Hash::hash(#repr, hasher);
            )*
            // Recurse on all fields.
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
            // Recurse on all variants starting at offset 0
            #(
                <#fields_types as AlignHash>::align_hash(hasher, &mut 0);
            )*
        }
    }
}

/// Generates `MaxSizeOf` implementation body.
fn gen_max_size_of_body(fields_types: &[&syn::Type]) -> proc_macro2::TokenStream {
    quote! {
        use ::std::mem;
        use ::epserde::traits::MaxSizeOf;

        let mut max_size_of = mem::align_of::<Self>();
        // Recurse on all fields.
        #(
            if max_size_of < <#fields_types as MaxSizeOf>::max_size_of() {
                max_size_of = <#fields_types as MaxSizeOf>::max_size_of();
            }
        )*
        max_size_of
    }
}

/// Generates the implementations for `TypeHash`, `AlignHash`, and optionally
/// `MaxSizeOf`.
fn gen_type_info_traits(
    ctx: &TypeInfoContext,
    where_clause_type_hash: &syn::WhereClause,
    where_clause_align_hash: &syn::WhereClause,
    where_clause_max_size_of: &syn::WhereClause,
    type_hash_body: proc_macro2::TokenStream,
    align_hash_body: proc_macro2::TokenStream,
    max_size_of_body: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let name = &ctx.name;
    let generics = &ctx.generics;
    let impl_generics = &ctx.impl_generics;

    let max_size_of_impl = if let Some(max_size_of_body) = max_size_of_body {
        quote! {
            #[automatically_derived]
            impl #impl_generics ::epserde::traits::MaxSizeOf for #name #generics #where_clause_max_size_of {
                #[inline(always)]
                fn max_size_of() -> usize {
                    #max_size_of_body
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics ::epserde::traits::TypeHash for #name #generics #where_clause_type_hash {
            #[inline(always)]
            fn type_hash(hasher: &mut impl ::core::hash::Hasher) {
                #type_hash_body
            }
        }
        #[automatically_derived]
        impl #impl_generics ::epserde::traits::AlignHash for #name #generics #where_clause_align_hash {
            #[inline(always)]
            fn align_hash(
                hasher: &mut impl ::core::hash::Hasher,
                offset_of: &mut usize,
            ) {
                #align_hash_body
            }
        }
        #max_size_of_impl
    }
}

/// Generate TypeHash implementation for struct types
fn gen_struct_type_info(ctx: TypeInfoContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    let mut generic_types = vec![];
    let mut fields_names = vec![];
    let mut fields_types = vec![];

    // Extract field information
    s.fields.iter().enumerate().for_each(|(field_idx, field)| {
        let ty = &field.ty;
        fields_names.push(get_field_name(field, field_idx));
        fields_types.push(ty);

        if ctx
            .type_const_params
            .iter()
            .any(|ident| type_equals_ident(&ty, ident))
        {
            generic_types.push(ty.clone());
        }
    });

    let TypeInfoWhereClauses {
        type_hash: where_clause_type_hash,
        align_hash: where_clause_align_hash,
        max_size_of: where_clause_max_size_of,
    } = gen_type_info_where_clauses(&ctx.where_clause, &fields_types);

    // Generate field hashes for TypeHash
    let mut field_hashes: Vec<_> = fields_names
        .iter()
        .map(|name| quote! {Hash::hash(stringify!(#name), hasher);})
        .collect();

    field_hashes.extend(
        fields_types
            .iter()
            .map(|ty| quote! {<#ty as TypeHash>::type_hash(hasher);}),
    );

    // Generate implementation bodies
    let type_hash_body = gen_type_info_body(&ctx, &field_hashes);
    let align_hash_body = gen_struct_align_hash_body(&ctx, &fields_types);
    let max_size_of_body = if ctx.is_zero_copy {
        Some(gen_max_size_of_body(&fields_types))
    } else {
        None
    };

    gen_type_info_traits(
        &ctx,
        &where_clause_type_hash,
        &where_clause_align_hash,
        &where_clause_max_size_of,
        type_hash_body,
        align_hash_body,
        max_size_of_body,
    )
}

/// Generate TypeHash implementation for enum types
fn gen_enum_type_info(ctx: TypeInfoContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut var_type_hashes = vec![];
    let mut var_align_hashes = vec![];
    let mut var_max_size_ofs = vec![];
    let mut generic_types = vec![];

    // Process each variant
    e.variants.iter().for_each(|variant| {
        let ident = variant.ident.to_owned();
        let mut var_type_hash = quote! { Hash::hash(stringify!(#ident), hasher); };
        let mut var_align_hash = quote! {};
        let mut var_max_size_of = quote! {};

        match &variant.fields {
            syn::Fields::Unit => {}
            syn::Fields::Named(fields) => {
                fields.named.iter().for_each(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let ty = &field.ty;

                    var_type_hash.extend([quote! {
                        Hash::hash(stringify!(#ident), hasher);
                        <#ty as TypeHash>::type_hash(hasher);
                    }]);
                    var_align_hash.extend([quote! {
                        <#ty as AlignHash>::align_hash(hasher, offset_of);
                    }]);
                    var_max_size_of.extend([quote! {
                        if max_size_of < <#ty as MaxSizeOf>::max_size_of() {
                            max_size_of = <#ty as MaxSizeOf>::max_size_of();
                        }
                    }]);

                    if ctx
                        .type_const_params
                        .iter()
                        .any(|ident| type_equals_ident(ty, ident))
                    {
                        generic_types.push(ty);
                    }
                });
            }
            syn::Fields::Unnamed(fields) => {
                fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .for_each(|(field_idx, field)| {
                        let ty = &field.ty;
                        let field_name = field_idx.to_string();

                        var_type_hash.extend([quote! {
                            Hash::hash(#field_name, hasher);
                            <#ty as TypeHash>::type_hash(hasher);
                        }]);
                        var_align_hash.extend([quote! {
                            <#ty as AlignHash>::align_hash(hasher, offset_of);
                        }]);
                        var_max_size_of.extend([quote! {
                            if max_size_of < <#ty as MaxSizeOf>::max_size_of() {
                                max_size_of = <#ty as MaxSizeOf>::max_size_of();
                            }
                        }]);

                        if ctx
                            .type_const_params
                            .iter()
                            .any(|ident| type_equals_ident(ty, ident))
                        {
                            generic_types.push(ty);
                        }
                    });
            }
        }

        var_type_hashes.push(var_type_hash);
        var_align_hashes.push(var_align_hash);
        var_max_size_ofs.push(var_max_size_of);
    });

    // Generate where clauses
    let where_clause = ctx
        .derive_input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(empty_where_clause);

    let TypeInfoWhereClauses {
        type_hash: where_clause_type_hash,
        align_hash: where_clause_align_hash,
        max_size_of: where_clause_max_size_of,
    } = gen_type_info_where_clauses(&where_clause, &generic_types);

    // Generate implementation bodies
    let type_hash_body = gen_type_info_body(&ctx, &var_type_hashes);

    let repr = &ctx.repr;
    let align_hash_body = if ctx.is_zero_copy {
        quote! {
            use ::core::hash::Hash;
            // Hash in size, as padding is given by MaxSizeOf.
            // and it is independent of the architecture.
            Hash::hash(&::core::mem::size_of::<Self>(), hasher);
            // Hash in representation data.
            #(
                Hash::hash(#repr, hasher);
            )*
            // Recurse on all fields.
            let old_offset_of = *offset_of;
            #(
                *offset_of = old_offset_of;
                #var_align_hashes
            )*
        }
    } else {
        quote! {
            // Recurse on all variants starting at offset 0
            // Note that we share var_align_hashes with the
            // zero-copy case, so we cannot pass &mut 0.
            #(
                *offset_of = 0;
                #var_align_hashes
            )*
        }
    };

    let max_size_of_body = quote! {
        let mut max_size_of = std::mem::align_of::<Self>();
        #(
            #var_max_size_ofs
        )*
        max_size_of
    };

    let max_size_of_body = if ctx.is_zero_copy {
        Some(max_size_of_body)
    } else {
        None
    };

    gen_type_info_traits(
        &ctx,
        &where_clause_type_hash,
        &where_clause_align_hash,
        &where_clause_max_size_of,
        type_hash_body,
        align_hash_body,
        max_size_of_body,
    )
}

/// Generate a partial ε-serde implementation for custom types.
///
/// It generates implementations just for the traits
/// `MaxSizeOf`, `TypeHash`, and `AlignHash`. See the documentation
/// of [`epserde_derive`] for more information.
#[proc_macro_derive(TypeInfo, attributes(zero_copy, deep_copy))]
pub fn type_info_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // This part is in common with epserde_derive.
    let mut derive_input = parse_macro_input!(input as DeriveInput);

    if derive_input.generics.where_clause.is_some() {
        panic!("The derive macros do not support where clauses on the original type.");
    }

    derive_input.generics.make_where_clause();
    let (impl_generics, generics, where_clause) = derive_input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let (_, is_zero_copy, _) = check_attrs(&derive_input);
    let (type_const_params, const_params) = get_type_const_params(&derive_input);

    _type_info_derive(
        &derive_input,
        type_const_params,
        const_params,
        generics,
        impl_generics,
        where_clause,
        is_zero_copy,
    )
}

/// Completes the `TypeInfo` derive macro using precomuted data.
///
/// This method is used by the `Epserde` derive macro to
/// avoid recomputing the same data twice.
fn _type_info_derive(
    derive_input: &DeriveInput,
    type_const_params: Vec<syn::Ident>,
    const_params: Vec<syn::Ident>,
    generics: TypeGenerics<'_>,
    impl_generics: ImplGenerics<'_>,
    where_clause: &WhereClause,
    is_zero_copy: bool,
) -> proc_macro::TokenStream {
    // Add reprs
    let repr = derive_input
        .attrs
        .iter()
        .filter(|x| x.meta.path().is_ident("repr"))
        .map(|x| x.meta.require_list().unwrap().tokens.to_string())
        .collect::<Vec<_>>();

    let name = derive_input.ident.clone();
    let ctx = TypeInfoContext {
        derive_input,
        name,
        type_const_params,
        const_params,
        generics,
        impl_generics,
        where_clause,
        is_zero_copy,
        repr,
    };

    match &derive_input.data {
        Data::Struct(s) => gen_struct_type_info(ctx, s),
        Data::Enum(e) => gen_enum_type_info(ctx, e),
        _ => todo!("Union types are not currently supported"),
    }
    .into()
}
