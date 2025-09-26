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

/// Adds a type trait bound to a where clause.
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

/// Generates a method call for field deserialization.
///
/// This methods takes care of choosing `_deser_eps_inner` or
/// `_deser_full_inner` depending on whether a field type is a type
/// parameter or not, and to use the special method
/// `_deser_eps_inner_special` for `PhantomDeserData`.
///
/// The type of `field_name` is [`proc_macro2::TokenStream`] because it can be
/// either an identifier (for named fields) or an index (for unnamed fields).
fn gen_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    type_params: &HashSet<&syn::Ident>,
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
        }

        // If it's a replaceable type parameter we proceed with ε-copy
        // deserialization
        if segments.len() == 1 && type_params.contains(&segments[0].ident) {
            return syn::parse_quote!(#field_name: unsafe  { <#field_type as DeserInner>::_deser_eps_inner(backend)? });
        }
    }

    syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_full_inner(backend)? })
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
) -> (Vec<&syn::Ident>, HashSet<&syn::Ident>, Vec<&syn::Ident>) {
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
            syn::GenericParam::Lifetime(_) => {
                panic!("Lifetime generics are not supported")
            }
        };
    }

    (type_const_params, type_params, const_params)
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
            "Type {} is declared as zero-copy, but it is not repr(C)",
            input.ident
        );
    }
    if is_zero_copy && is_deep_copy {
        panic!(
            "Type {} is declared as both zero-copy and deep-copy",
            input.ident
        );
    }

    (is_repr_c, is_zero_copy, is_deep_copy)
}

/// For each bounded type parameter that is the type of some field, binds the
/// associated (de)serialization types with the same trait bounds of the type.
fn bind_ser_deser_types(
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
                        "'epserde_desertype",
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
                            ::epserde::deser::DeserType<'epserde_desertype, #ident>
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

/// Adds to the given (de)serialization where clause a bound
/// binding the given type to `(De)SerInner`.
fn add_ser_deser_trait_bounds(
    ty: &syn::Type,
    is_zero_copy: bool,
    ser_where_clause: &mut syn::WhereClause,
    deser_where_clause: &mut syn::WhereClause,
) {
    if is_zero_copy {
        add_trait_bound(
            ser_where_clause,
            ty,
            syn::parse_quote!(::epserde::ser::SerInner<SerType = #ty>),
        );
        add_trait_bound(
            ser_where_clause,
            ty,
            syn::parse_quote!(::epserde::traits::AlignHash),
        );
        add_trait_bound(
            ser_where_clause,
            ty,
            syn::parse_quote!(::epserde::traits::TypeHash),
        );
        add_trait_bound(
            ser_where_clause,
            ty,
            syn::parse_quote!(::epserde::traits::AlignTo),
        );
        add_trait_bound(
            deser_where_clause,
            ty,
            syn::parse_quote!(::epserde::deser::DeserInner),
        );
    } else {
        add_trait_bound(
            ser_where_clause,
            ty,
            syn::parse_quote!(::epserde::ser::SerInner<SerType: ::epserde::traits::TypeHash + ::epserde::traits::AlignHash>),
        );
        add_trait_bound(
            deser_where_clause,
            ty,
            syn::parse_quote!(::epserde::deser::DeserInner),
        );
    }
}

/// Generates generics for the deserialization type by replacing type parameters
/// that are types of fields with their associated deserialization type.
fn gen_generics_for_deser_type(
    ctx: &EpserdeContext,
    repl_params: &HashSet<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_params
        .iter()
        .map(|ident| {
            if repl_params.contains(ident) {
                quote!(::epserde::deser::DeserType<'epserde_desertype, #ident>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generates generics for the serialization type by replacing type parameters
/// that are types of fields with their associated serialization type.
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
) -> (WhereClause, WhereClause) {
    let mut ser_where_clause = empty_where_clause();
    let mut deser_where_clause = empty_where_clause();

    // Add trait bounds for all field types
    for field_type in field_types {
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
/// The where clauses bound all field types with the trait being implemented,
/// thus propagating the trait recursively, with the proviso that in case of a
/// replaceable type parameter of a deep-copy type we bound the associated
/// serialization type instead.
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

    let mut bound_align_of = Punctuated::new();
    bound_align_of.push(syn::parse_quote!(::epserde::traits::AlignTo));
    let align_of = gen_type_info_where_clause(bound_align_of);

    (type_hash, align_hash, align_of)
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
    /// Whether the type has `#[zero_copy]`
    is_zero_copy: bool,
    /// Whether the type has `#[deep_copy]`
    is_deep_copy: bool,
}

/// [`Epserde`] derive code for struct types.
fn gen_epserde_struct_impl(ctx: &EpserdeContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];
    let mut repl_params = HashSet::new();

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;

        // We look for type parameters that are types of fields
        if let Some(field_type_id) = get_ident(field_type) {
            if ctx.type_params.contains(field_type_id) {
                repl_params.insert(field_type_id);
            }
        }

        method_calls.push(gen_deser_method_call(
            &field_name,
            field_type,
            &ctx.type_params,
        ));

        field_names.push(field_name);
        field_types.push(field_type);
    }

    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &repl_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &repl_params);
    let is_zero_copy_expr = gen_is_zero_copy_expr(ctx.is_repr_c, &field_types);
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(&field_types, ctx.is_zero_copy);

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

                // The type is declared as zero-copy, so a fortiori there is no mismatch.
                const ZERO_COPY_MISMATCH: bool = false;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    // No-op code that however checks that all fields are zero-copy.
                    fn test<T: ::epserde::traits::ZeroCopy>() {}
                    #(
                        test::<#field_types>();
                    )*
                    ::epserde::ser::helpers::ser_zero(backend, self)
                }
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause
            {
                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    unsafe { ::epserde::deser::helpers::deser_full_zero::<Self>(backend) }
                }

                type DeserType<'epserde_desertype> = &'epserde_desertype Self;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    unsafe { ::epserde::deser::helpers::deser_eps_zero::<Self>(backend) }
                }
            }
        }
    } else {
        bind_ser_deser_types(
            ctx.derive_input,
            &repl_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );

        let is_deep_copy = ctx.is_deep_copy;

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

                // Whether the type could be zero-copy but it is not
                // declared as such, and the attribute `deep_copy` is missing.
                const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#field_types>::IS_ZERO_COPY)*;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    #(
                        unsafe { WriteWithNames::write(backend, stringify!(#field_names), &self.#field_names)?; }
                    )*
                    Ok(())
                }
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause {
                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserInner;

                    Ok(#name{
                        #(
                            #field_names: unsafe { <#field_types as DeserInner>::_deser_full_inner(backend)? },
                        )*
                    })
                }

                type DeserType<'epserde_desertype> = #name<#(#generics_for_deser_type,)*>;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserInner;

                    Ok(#name{
                        #(
                            #method_calls,
                        )*
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
    // Type parameters that are types of some fields in some variant
    let mut all_repl_params = HashSet::new();
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

                    // We look for type parameters that are types of fields
                    if let Some(field_type_id) = get_ident(field_type) {
                        if ctx.type_params.contains(field_type_id) {
                            all_repl_params.insert(field_type_id);
                        }
                    }

                    method_calls.push(gen_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
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

                    // We look for type parameters that are types of fields
                    if let Some(field_type_id) = get_ident(field_type) {
                        if ctx.type_params.contains(field_type_id) {
                            all_repl_params.insert(field_type_id);
                        }
                    }

                    field_indices.push(
                        syn::Ident::new(&format!("v{}", field_idx), proc_macro2::Span::call_site())
                            .to_token_stream(),
                    );

                    method_calls.push(gen_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        &all_repl_params,
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
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(&all_fields_types, ctx.is_zero_copy);

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

                // The type is declared as zero-copy, so a fortiori there is no mismatch.
                const ZERO_COPY_MISMATCH: bool = false;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    // No-op code that however checks that all fields are zero-copy.
                    fn test<T: ::epserde::traits::ZeroCopy>() {}
                    #(
                        test::<#all_fields_types>();
                    )*

                    unsafe { ::epserde::ser::helpers::ser_zero(backend, self) }
                }
            }

            #[automatically_derived]
            impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause {
                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    unsafe { ::epserde::deser::helpers::deser_full_zero::<Self>(backend) }
                }

                type DeserType<'epserde_desertype> = &'epserde_desertype Self;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    unsafe { ::epserde::deser::helpers::deser_eps_zero::<Self>(backend) }
                }
            }
        }
    } else {
        bind_ser_deser_types(
            ctx.derive_input,
            &all_repl_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );

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

                // Whether the type could be zero-copy but it is not
                // declared as such, and the attribute `deep_copy` is missing.
                const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#all_fields_types>::IS_ZERO_COPY)*;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    ::epserde::ser::helpers::check_mismatch::<Self>();
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

                type DeserType<'epserde_desertype> = #name<#(#generics_for_deser_type,)*>;

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
/// The attribute `zero_copy` can be used to generate an implementation for a
/// zero-copy type, but the type must be `repr(C)` and all fields must be
/// zero-copy.
///
/// If you do not specify `zero_copy`, the macro assumes your structure is
/// deep-copy. However, if you have a structure that could be zero-copy, but has
/// no attribute, a warning will be issued every time you serialize an instance
/// of the type. The warning can be silenced adding the explicit attribute
/// `deep_copy`.
#[proc_macro_derive(Epserde, attributes(zero_copy, deep_copy))]
pub fn epserde_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // This part is in common with type_info_derive
    let mut derive_input = parse_macro_input!(input as DeriveInput);

    if derive_input.generics.where_clause.is_some() {
        panic!("The derive macros do not support where clauses on the original type.");
    }

    derive_input.generics.make_where_clause();
    let (generics_for_impl, generics_for_type, where_clause) =
        derive_input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let (is_repr_c, is_zero_copy, is_deep_copy) = check_attrs(&derive_input);
    let (type_const_params, type_params, const_params) = get_type_const_params(&derive_input);

    let ctx = EpserdeContext {
        derive_input: &derive_input,
        type_const_params,
        type_params,
        generics_for_impl,
        generics_for_type,
        where_clause,
        is_repr_c,
        is_zero_copy,
        is_deep_copy,
    };

    let mut out: proc_macro::TokenStream = match &derive_input.data {
        Data::Struct(s) => gen_epserde_struct_impl(&ctx, s),
        Data::Enum(e) => gen_epserde_enum_impl(&ctx, e),
        _ => todo!("Union types are not currently supported"),
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

            // Hash in size, as padding is given by AlignTo.
            // and it is independent of the architecture.
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
fn gen_struct_align_of_body(fields_types: &[&syn::Type]) -> proc_macro2::TokenStream {
    quote! {
        use ::epserde::traits::AlignTo;
        use ::epserde::ser::SerType;

        let mut align_of = ::core::mem::align_of::<Self>();

        #(
            if align_of < <#fields_types as AlignTo>::align_to() {
                align_of = <#fields_types as AlignTo>::align_to();
            }
        )*
        align_of
    }
}

/// Generates the implementations for `TypeHash`, `AlignHash`, and
/// optionally `AlignTo`.
fn gen_type_info_traits(
    ctx: TypeInfoContext,
    type_hash_where_clause: syn::WhereClause,
    align_hash_where_clause: syn::WhereClause,
    align_of_where_clause: syn::WhereClause,
    type_hash_body: proc_macro2::TokenStream,
    align_hash_body: proc_macro2::TokenStream,
    align_of_body: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let name = &ctx.name;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;

    let align_of_impl = if let Some(align_of_body) = align_of_body {
        quote! {
            #[automatically_derived]
            impl #generics_for_impl ::epserde::traits::AlignTo for #name #generics_for_type #align_of_where_clause {
                fn align_to() -> usize {
                    #align_of_body
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

        #align_of_impl
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

    let (type_hash_where_clause, align_hash_where_clause, align_of_where_clause) =
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
    let align_of_body = if ctx.is_zero_copy {
        Some(gen_struct_align_of_body(&field_types))
    } else {
        None
    };

    gen_type_info_traits(
        ctx,
        type_hash_where_clause,
        align_hash_where_clause,
        align_of_where_clause,
        type_hash_body,
        align_hash_body,
        align_of_body,
    )
}

/// [`TypeInfo`] derive code for enum types.
fn gen_enum_type_info_impl(ctx: TypeInfoContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut all_type_hashes = vec![];
    let mut all_align_hashes = vec![];
    let mut all_align_ofs = vec![];
    let mut all_field_types = vec![];
    let mut all_repl_params = HashSet::new();

    // Process each variant
    for variant in &e.variants {
        let ident = &variant.ident;
        let mut type_hash = quote! { Hash::hash(stringify!(#ident), hasher); };
        let mut field_types = vec![];
        let mut align_hash = quote! {};
        let mut align_of = quote! {};

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

                    align_of.extend([quote! {
                        if align_of < <#field_type as AlignTo>::align_to() {
                            align_of = <#field_type as AlignTo>::align_to();
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

                    align_of.extend([quote! {
                        if align_of < <#field_type as AlignTo>::align_to() {
                            align_of = <#field_type as AlignTo>::align_to();
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
        all_align_ofs.push(align_of);
        all_field_types.extend(field_types);
    }

    let (where_clause_type_hash, where_clause_align_hash, where_clause_align_of) =
        gen_type_info_where_clauses(ctx.where_clause, ctx.is_zero_copy, &all_field_types);

    let type_hash_body = gen_type_hash_body(&ctx, &all_type_hashes);
    let align_hash_body = gen_enum_align_hash_body(&ctx, &all_align_hashes);
    let align_of_body = quote! {
        let mut align_of = core::mem::align_of::<Self>();
        #(
            #all_align_ofs
        )*
        align_of
    };

    let align_of_body = if ctx.is_zero_copy {
        Some(align_of_body)
    } else {
        None
    };

    gen_type_info_traits(
        ctx,
        where_clause_type_hash,
        where_clause_align_hash,
        where_clause_align_of,
        type_hash_body,
        align_hash_body,
        align_of_body,
    )
}

/// Generates a [partial ε-serde](TypeInfo) implementation for custom types.
///
/// It generates implementations just for the traits `CopyType`, `AlignTo`,
/// `TypeHash`, and `AlignHash`. See the documentation of [`Epserde`] for
/// more information.
#[proc_macro_derive(TypeInfo, attributes(zero_copy, deep_copy))]
pub fn type_info_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut derive_input = parse_macro_input!(input as DeriveInput);

    if derive_input.generics.where_clause.is_some() {
        panic!("The derive macros do not support where clauses on the original type.");
    }

    derive_input.generics.make_where_clause();
    let (generics_for_impl, generics_for_type, where_clause) =
        derive_input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let (_, is_zero_copy, _) = check_attrs(&derive_input);
    let (_, type_params, const_params) = get_type_const_params(&derive_input);

    _type_info_derive(
        &derive_input,
        type_params,
        const_params,
        generics_for_impl,
        generics_for_type,
        where_clause,
        is_zero_copy,
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
        Data::Struct(s) => gen_struct_type_info_impl(ctx, s),
        Data::Enum(e) => gen_enum_type_info_impl(ctx, e),
        _ => todo!("Union types are not currently supported"),
    }
    .into()
}
