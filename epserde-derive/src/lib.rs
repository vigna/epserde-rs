/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Derive procedural macros for the [`epserde`](https://crates.io/crates/epserde) crate.

*/

use std::collections::HashSet;

use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, token, BoundLifetimes, Data, DeriveInput,
    GenericParam, LifetimeParam, PredicateType, WhereClause, WherePredicate,
};

/// Returns an empty where clause.
fn empty_where_clause() -> WhereClause {
    WhereClause {
        where_token: token::Where::default(),
        predicates: Punctuated::new(),
    }
}

/// Adds a trait bound to a where clause.
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

/// Returns true if the given type just made of the given identifier.
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
fn generate_method_call(
    field_name: &proc_macro2::TokenStream,
    ty: &syn::Type,
    generic_fields_ids: &HashSet<syn::Ident>,
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

        if segments.len() == 1 && generic_fields_ids.contains(&segments[0].ident) {
            return syn::parse_quote!(#field_name: <#ty as DeserializeInner>::_deserialize_eps_inner);
        }
    }

    syn::parse_quote!(#field_name: <#ty as DeserializeInner>::_deserialize_full_inner)
}

/// Generates the `IS_ZERO_COPY` expression.
fn generate_is_zero_copy_expr(
    is_repr_c: bool,
    fields_types: &[syn::Type],
) -> proc_macro2::TokenStream {
    if fields_types.is_empty() {
        quote!(#is_repr_c)
    } else {
        quote!(#is_repr_c #(&& <#fields_types>::IS_ZERO_COPY)*)
    }
}

/// Pre-parsed information for the derive macros.
struct CommonDeriveInput {
    /// The identifier of the type.
    name: syn::Ident,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_ids: Vec<syn::Ident>,
    /// Identifiers of const parameters, in order of appearance.
    const_ids: Vec<syn::Ident>,
    /// All generics (lifetimes and type parameters) concatenated and separated
    /// by commas, in order of appearance. It can be put between `<` and `>`
    /// after the structure name.
    concat_generics: proc_macro2::TokenStream,
    /// Same as `concat_generics`, but with all necessary trait
    /// bounds. It can be put between `<` and `>` after
    /// the `impl` keyword.
    impl_generics: proc_macro2::TokenStream,
}

impl CommonDeriveInput {
    /// Create a new `CommonDeriveInput` from a `DeriveInput`.
    /// Additionally, one can specify traits and lifetimes to
    /// be added to the generic types.
    fn new(input: DeriveInput, traits_to_add: Vec<syn::Path>) -> Self {
        let name = input.ident;
        let mut type_const_ids = vec![];
        let mut const_ids = vec![];
        let mut concat_generics = quote!();
        let mut impl_generics = quote!();

        input.generics.params.into_iter().for_each(|x| {
            match x {
                syn::GenericParam::Type(mut t) => {
                    type_const_ids.push(t.ident.clone());
                    concat_generics.extend(t.ident.to_token_stream());

                    // Remove default and add traits
                    t.default = None;
                    for trait_to_add in traits_to_add.iter() {
                        t.bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
                            paren_token: None,
                            modifier: syn::TraitBoundModifier::None,
                            lifetimes: None,
                            path: trait_to_add.clone(),
                        }));
                    }

                    impl_generics.extend(quote!(#t,));
                }
                syn::GenericParam::Lifetime(l) => {
                    concat_generics.extend(l.lifetime.to_token_stream());
                    impl_generics.extend(quote!(#l,));
                }
                syn::GenericParam::Const(mut c) => {
                    const_ids.push(c.ident.clone());
                    type_const_ids.push(c.ident.clone());
                    concat_generics.extend(c.ident.to_token_stream());

                    // Remove default
                    c.default = None;
                    impl_generics.extend(quote!(#c,));
                }
            };
            concat_generics.extend(quote!(,))
        });

        Self {
            name,
            const_ids,
            type_const_ids,
            impl_generics,
            concat_generics,
        }
    }
}

/// Return whether the struct has attributes `repr(C)`, `zero_copy`, and `deep_copy`.
///
/// Performs coherence checks (e.g., to be `zero_copy` the struct must be `repr(C)`).
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

/// Add trait bounds for associated (de)serialization types based on bounds on
/// type parameters that are types of some field.
fn add_ser_deser_bounds(
    derive_input: &DeriveInput,
    generic_fields_ids: &HashSet<syn::Ident>,
    where_clause_ser: &mut WhereClause,
    where_clause_des: &mut WhereClause,
) {
    // If there are bounded type parameters which are fields of the
    // struct, we need to impose the same bounds on the SerType and on
    // the DeserType.
    derive_input.generics.params.iter().for_each(|param| {
        if let syn::GenericParam::Type(t) = param {
            let ident = &t.ident;

            // We are just interested in types with bounds that are
            // types of fields of the struct.
            if !t.bounds.is_empty()
                && generic_fields_ids.contains(ident)
            {
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


                // Add the type bounds to the DeserType
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

                // Add the type bounds to the SerType
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
    });
}

/// Add SerializeInner and DeserializeInner trait bounds for a field type
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

/// Generate deserialization type generics
fn generate_deser_type_generics<'a>(
    ctx: &CodegenContext,
    generic_fields_ids: &HashSet<syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_ids
        .iter()
        .map(|ident| {
            if generic_fields_ids.contains(ident)
            {
                quote!(<#ident as ::epserde::deser::DeserializeInner>::DeserType<'epserde_desertype>)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Generate serialization type generics
fn generate_ser_type_generics<'a>(
    ctx: &CodegenContext,
    generic_fields_ids: &HashSet<syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    ctx.type_const_ids
        .iter()
        .map(|ident| {
            if generic_fields_ids.contains(ident) {
                quote!(<#ident as ::epserde::ser::SerializeInner>::SerType)
            } else {
                quote!(#ident)
            }
        })
        .collect()
}

/// Where clauses for `SerializeInner` and `DeserializeInner`
struct WhereClauses {
    /// Where clause for `SerializeInner`.
    serialize: WhereClause,
    /// Where clause for `DeserializeInner`.
    deserialize: WhereClause,
}

/// Generate where clauses for main derive traits (SerializeInner, DeserializeInner)
fn generate_where_clauses(base_clause: &WhereClause, field_types: &[syn::Type]) -> WhereClauses {
    let mut where_clause_ser = base_clause.clone();
    let mut where_clause_des = base_clause.clone();

    // Add trait bounds for all field types
    for ty in field_types {
        add_ser_deser_trait_bounds(&mut where_clause_ser, &mut where_clause_des, ty);
    }

    WhereClauses {
        serialize: where_clause_ser,
        deserialize: where_clause_des,
    }
}

/// Set of where clauses for TypeHash traits
struct TypeHashWhereClausesSet {
    /// Where clause for TypeHash trait
    type_hash: WhereClause,
    /// Where clause for AlignHash trait
    align_hash: WhereClause,
    /// Where clause for MaxSizeOf trait
    max_size_of: WhereClause,
}

/// Generate where clauses for TypeHash traits (TypeHash, AlignHash, MaxSizeOf)
fn generate_type_hash_where_clauses(
    base_clause: &WhereClause,
    field_types: &[&syn::Type],
) -> TypeHashWhereClausesSet {
    let mut bounds_type_hash = Punctuated::new();
    bounds_type_hash.push(syn::parse_quote!(::epserde::traits::TypeHash));
    let mut where_clause_type_hash = base_clause.clone();

    let mut bounds_align_hash = Punctuated::new();
    bounds_align_hash.push(syn::parse_quote!(::epserde::traits::AlignHash));
    let mut where_clause_align_hash = base_clause.clone();

    let mut bounds_max_size_of = Punctuated::new();
    bounds_max_size_of.push(syn::parse_quote!(::epserde::traits::MaxSizeOf));
    let mut where_clause_max_size_of = base_clause.clone();

    // Add trait bounds for all field types
    for &ty in field_types {
        where_clause_type_hash
            .predicates
            .push(WherePredicate::Type(PredicateType {
                lifetimes: None,
                bounded_ty: ty.clone(),
                colon_token: token::Colon::default(),
                bounds: bounds_type_hash.clone(),
            }));
        where_clause_align_hash
            .predicates
            .push(WherePredicate::Type(PredicateType {
                lifetimes: None,
                bounded_ty: ty.clone(),
                colon_token: token::Colon::default(),
                bounds: bounds_align_hash.clone(),
            }));
        where_clause_max_size_of
            .predicates
            .push(WherePredicate::Type(PredicateType {
                lifetimes: None,
                bounded_ty: ty.clone(),
                colon_token: token::Colon::default(),
                bounds: bounds_max_size_of.clone(),
            }));
    }

    TypeHashWhereClausesSet {
        type_hash: where_clause_type_hash,
        align_hash: where_clause_align_hash,
        max_size_of: where_clause_max_size_of,
    }
}

/// Context structure containing all the common parameters needed for code generation
struct CodegenContext {
    /// The original derive input containing all metadata
    derive_input: DeriveInput,
    /// The name of the type being derived
    name: syn::Ident,
    /// Concatenated generics for type declarations (e.g., `<T, U>`)
    concat_generics: proc_macro2::TokenStream,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_ids: Vec<syn::Ident>,
    /// Implementation generics for trait bounds
    impl_generics: proc_macro2::TokenStream,
    /// Serialization generics with bounds
    generics_serialize: proc_macro2::TokenStream,
    /// Deserialization generics with bounds
    generics_deserialize: proc_macro2::TokenStream,
    /// Whether the type has `#[repr(C)]`
    is_repr_c: bool,
    /// Whether the type has `#[zero_copy]`
    is_zero_copy: bool,
    /// Whether the type has `#[deep_copy]`
    is_deep_copy: bool,
}

/// Common initialization data for trait implementation generation
struct TraitImplInit {
    /// Base where clause from the derive input
    base_where_clause: WhereClause,
    /// Where clause for serialization traits
    where_clause_ser: WhereClause,
    /// Where clause for deserialization traits
    where_clause_des: WhereClause,
    /// Expression for IS_ZERO_COPY constant
    is_zero_copy_expr: proc_macro2::TokenStream,
    /// Type name
    name: syn::Ident,
    /// Implementation generics
    impl_generics: proc_macro2::TokenStream,
    /// Concatenated generics
    concat_generics: proc_macro2::TokenStream,
    /// Serialization generics
    generics_serialize: proc_macro2::TokenStream,
    /// Deserialization generics
    generics_deserialize: proc_macro2::TokenStream,
}

/// Initialize common trait implementation data
fn initialize_trait_impl(ctx: &CodegenContext, fields_types: &[syn::Type]) -> TraitImplInit {
    let base_where_clause = ctx
        .derive_input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(empty_where_clause);

    // Generate where clauses for trait implementations
    let where_clauses = generate_where_clauses(&base_where_clause, fields_types);
    let where_clause_ser = where_clauses.serialize;
    let where_clause_des = where_clauses.deserialize;

    let is_zero_copy_expr = generate_is_zero_copy_expr(ctx.is_repr_c, fields_types);

    TraitImplInit {
        base_where_clause,
        where_clause_ser,
        where_clause_des,
        is_zero_copy_expr,
        name: ctx.name.clone(),
        impl_generics: ctx.impl_generics.clone(),
        concat_generics: ctx.concat_generics.clone(),
        generics_serialize: ctx.generics_serialize.clone(),
        generics_deserialize: ctx.generics_deserialize.clone(),
    }
}

/// Generate implementation for enum types
fn generate_enum_impl(ctx: CodegenContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut variants_names = Vec::new();
    let mut variants = Vec::new();
    let mut variant_ser = Vec::new();
    let mut variant_full_des = Vec::new();
    let mut variant_eps_des = Vec::new();
    let mut generic_fields_ids = HashSet::new();
    let mut fields_types = Vec::new();
    let type_const_ids = ctx.type_const_ids.iter().cloned().collect::<HashSet<_>>();

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
            let mut var_fields_names = Vec::new();
            let mut var_fields_types = Vec::new();
            let mut method_calls: Vec<proc_macro2::TokenStream> = vec![];
            fields
                .named
                .iter()
                .map(|named| (named.ident.as_ref().unwrap(), &named.ty))
                .for_each(|(name, ty)| {
                    for id in &type_const_ids {
                        if type_equals_ident(ty, id) {
                            generic_fields_ids.insert(id.clone());
                            break;
                        }
                    }

                    method_calls.push(generate_method_call(&name.to_token_stream(), ty, &generic_fields_ids));

                    var_fields_names.push(name.to_token_stream());
                    var_fields_types.push(ty.clone());
                });
            let ident = variant.ident.clone();
            variants.push(quote! {
                #ident{ #( #var_fields_names, )* }
            });
            fields_types.extend(var_fields_types.clone());
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
            let mut var_fields_names = Vec::new();
            let mut var_fields_vars = Vec::new();
            let mut var_fields_types = Vec::new();
            let mut method_calls: Vec<proc_macro2::TokenStream> = vec![];

            fields
                .unnamed
                .iter()
                .enumerate()
                .for_each(|(field_idx, unnamed)| {
                    let ty = &unnamed.ty;
                    let name = syn::Index::from(field_idx);
                    for id in &type_const_ids {
                        if type_equals_ident(ty, id) {
                            generic_fields_ids.insert(id.clone());
                            break;
                        }
                    }

                    var_fields_names.push(syn::Ident::new(
                        &format!("v{}", field_idx),
                        proc_macro2::Span::call_site(),
                    )
                    .to_token_stream());

                    method_calls.push(generate_method_call(&name.to_token_stream(), ty, &generic_fields_ids));

                    var_fields_vars.push(syn::Index::from(field_idx));
                    var_fields_types.push(ty.clone());
                });

            let ident = variant.ident.clone();
            variants.push(quote! {
                #ident( #( #var_fields_names, )* )
            });
            fields_types.extend(var_fields_types.clone());

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
    let deser_type_generics = generate_deser_type_generics(&ctx, &generic_fields_ids);
    let ser_type_generics = generate_ser_type_generics(&ctx, &generic_fields_ids);
    let tag = (0..variants.len()).collect::<Vec<_>>();

    // Initialize common trait implementation data
    let TraitImplInit {
        base_where_clause: where_clause,
        mut where_clause_ser,
        mut where_clause_des,
        is_zero_copy_expr,
        name,
        impl_generics,
        concat_generics,
        generics_serialize,
        generics_deserialize,
    } = initialize_trait_impl(&ctx, &fields_types);
    let is_deep_copy = ctx.is_deep_copy;

    if ctx.is_zero_copy {
        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.

        quote! {
            #[automatically_derived]
            impl<#impl_generics> ::epserde::traits::CopyType for #name<#concat_generics> #where_clause {
                type Copy = ::epserde::traits::Zero;
            }
            #[automatically_derived]
            impl<#generics_serialize> ::epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
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
            impl<#generics_deserialize> ::epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des {
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
            &generic_fields_ids,
            &mut where_clause_ser,
            &mut where_clause_des,
        );

        let is_zero_copy_expr = generate_is_zero_copy_expr(ctx.is_repr_c, &fields_types);

        quote! {
            #[automatically_derived]
            impl<#impl_generics> ::epserde::traits::CopyType for #name<#concat_generics> #where_clause {
                type Copy = ::epserde::traits::Deep;
            }
            #[automatically_derived]

            impl<#generics_serialize> ::epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
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
            impl<#generics_deserialize> ::epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des {
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

/// Generate implementation for struct types
fn generate_struct_impl(ctx: CodegenContext, s: &syn::DataStruct) -> proc_macro2::TokenStream {
    let mut fields_types = vec![];
    let mut fields_names = vec![];
    let mut generic_fields_ids = HashSet::new();
    let mut method_calls: Vec<proc_macro2::TokenStream> = vec![];
    let type_const_ids = ctx.type_const_ids.iter().cloned().collect::<HashSet<_>>();

    // Scan the struct to find which fields contain a generic
    // type (i.e., they are themselves of a generic type,
    // or of a type containing a generic type as a parameter).
    s.fields.iter().enumerate().for_each(|(field_idx, field)| {
        let field_type = &field.ty;
        let field_name = get_field_name(field, field_idx);

        for id in &type_const_ids {
            if type_equals_ident(field_type, id) {
                generic_fields_ids.insert(id.clone());
                break;
            }
        }

        method_calls.push(generate_method_call(
            &field_name,
            field_type,
            &generic_fields_ids,
        ));
        fields_types.push(field_type.clone());
        fields_names.push(field_name);
    });

    // Gather deserialization types of fields, as they are necessary to
    // derive the deserialization type.
    let deser_type_generics = generate_deser_type_generics(&ctx, &generic_fields_ids);
    let ser_type_generics = generate_ser_type_generics(&ctx, &generic_fields_ids);

    // Initialize common trait implementation data
    let TraitImplInit {
        base_where_clause: where_clause,
        mut where_clause_ser,
        mut where_clause_des,
        is_zero_copy_expr,
        name,
        impl_generics,
        concat_generics,
        generics_serialize,
        generics_deserialize,
    } = initialize_trait_impl(&ctx, &fields_types);

    if ctx.is_zero_copy {
        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.
        quote! {
            #[automatically_derived]
            impl<#impl_generics> ::epserde::traits::CopyType for #name<#concat_generics> #where_clause {
                type Copy = ::epserde::traits::Zero;
            }

            #[automatically_derived]
            impl<#generics_serialize> ::epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
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
            impl<#generics_deserialize> ::epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des
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
            &generic_fields_ids,
            &mut where_clause_ser,
            &mut where_clause_des,
        );

        let is_deep_copy = ctx.is_deep_copy;

        quote! {
            #[automatically_derived]
            impl<#impl_generics> ::epserde::traits::CopyType for #name<#concat_generics> #where_clause {
                type Copy = ::epserde::traits::Deep;
            }

            #[automatically_derived]
            impl<#generics_serialize> ::epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
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
                    #(
                        WriteWithNames::write(backend, stringify!(#fields_names), &self.#fields_names)?;
                    )*
                    Ok(())
                }
            }

            #[automatically_derived]
            impl<#generics_deserialize> ::epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des {
                unsafe fn _deserialize_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserializeInner;

                    Ok(#name{
                        #(
                            #fields_names: unsafe { <#fields_types as ::epserde::deser::DeserializeInner>::_deserialize_full_inner(backend)? },
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

/// Context structure for TypeHash code generation
struct TypeHashContext {
    /// The original derive input
    input: DeriveInput,
    /// The name of the type
    name: syn::Ident,
    /// Implementation generics
    impl_generics: proc_macro2::TokenStream,
    /// Concatenated generics
    concat_generics: proc_macro2::TokenStream,
    /// Identifiers of type and const parameters, in order of appearance.
    type_const_ids: Vec<syn::Ident>,
    /// Identifiers of const parameters, in order of appearance.
    const_ids: Vec<syn::Ident>,
    /// Whether the type is zero-copy
    is_zero_copy: bool,
    /// Type name as string literal
    name_literal: String,
    /// `repr` attributes
    repr: Vec<String>,
}

/// Generate TypeHash implementation body
fn generate_type_hash_body(
    ctx: &TypeHashContext,
    field_hashes: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let copy_type = if ctx.is_zero_copy {
        "ZeroCopy"
    } else {
        "DeepCopy"
    };
    let const_ids = &ctx.const_ids;
    let name_literal = &ctx.name_literal;

    quote! {
        use ::core::hash::Hash;
        use ::epserde::traits::TypeHash;
        // Hash in copy type
        Hash::hash(#copy_type, hasher);
        // Hash the values of generic constants
        #(
            Hash::hash(&#const_ids, hasher);
        )*
        // Hash the identifiers of generic constants
        #(
            Hash::hash(stringify!(#const_ids), hasher);
        )*
        // Hash in struct and field names.
        Hash::hash(#name_literal, hasher);
        // Hash field information
        #(
            #field_hashes
        )*
    }
}

/// Generate AlignHash implementation body for structs
fn generate_struct_align_hash_body(
    ctx: &TypeHashContext,
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

/// Generate MaxSizeOf implementation body
fn generate_max_size_of_body(fields_types: &[&syn::Type]) -> proc_macro2::TokenStream {
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

/// Generates the implementations for TypeHash, AlignHash, and optionally MaxSizeOf.
fn generate_type_hash_traits(
    ctx: &TypeHashContext,
    where_clause_type_hash: &syn::WhereClause,
    where_clause_align_hash: &syn::WhereClause,
    where_clause_max_size_of: &syn::WhereClause,
    type_hash_body: proc_macro2::TokenStream,
    align_hash_body: proc_macro2::TokenStream,
    max_size_of_body: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let name = &ctx.name;
    let impl_generics = &ctx.impl_generics;
    let concat_generics = &ctx.concat_generics;

    let max_size_of_impl = if let Some(max_size_of_body) = max_size_of_body {
        quote! {
            #[automatically_derived]
            impl<#impl_generics> ::epserde::traits::MaxSizeOf for #name<#concat_generics> #where_clause_max_size_of {
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
        impl<#impl_generics> ::epserde::traits::TypeHash for #name<#concat_generics> #where_clause_type_hash {
            #[inline(always)]
            fn type_hash(hasher: &mut impl ::core::hash::Hasher) {
                #type_hash_body
            }
        }
        #[automatically_derived]
        impl<#impl_generics> ::epserde::traits::AlignHash for #name<#concat_generics> #where_clause_align_hash {
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
fn generate_struct_type_hash(
    ctx: TypeHashContext,
    s: &syn::DataStruct,
) -> proc_macro2::TokenStream {
    let mut generic_types = vec![];
    let mut fields_names = vec![];
    let mut fields_types = vec![];

    // Extract field information
    s.fields.iter().enumerate().for_each(|(field_idx, field)| {
        let ty = &field.ty;
        fields_names.push(get_field_name(field, field_idx));
        fields_types.push(ty);

        if ctx
            .type_const_ids
            .iter()
            .any(|ident| type_equals_ident(&ty, ident))
        {
            generic_types.push(ty.clone());
        }
    });

    // Generate where clauses
    let where_clause = ctx
        .input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(empty_where_clause);

    let TypeHashWhereClausesSet {
        type_hash: where_clause_type_hash,
        align_hash: where_clause_align_hash,
        max_size_of: where_clause_max_size_of,
    } = generate_type_hash_where_clauses(&where_clause, &fields_types);

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
    let type_hash_body = generate_type_hash_body(&ctx, &field_hashes);
    let align_hash_body = generate_struct_align_hash_body(&ctx, &fields_types);
    let max_size_of_body = if ctx.is_zero_copy {
        Some(generate_max_size_of_body(&fields_types))
    } else {
        None
    };

    generate_type_hash_traits(
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
fn generate_enum_type_hash(ctx: TypeHashContext, e: &syn::DataEnum) -> proc_macro2::TokenStream {
    let mut var_type_hashes = Vec::new();
    let mut var_align_hashes = Vec::new();
    let mut var_max_size_ofs = Vec::new();
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
                        .type_const_ids
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
                            .type_const_ids
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
        .input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(empty_where_clause);

    let TypeHashWhereClausesSet {
        type_hash: where_clause_type_hash,
        align_hash: where_clause_align_hash,
        max_size_of: where_clause_max_size_of,
    } = generate_type_hash_where_clauses(&where_clause, &generic_types);

    // Generate implementation bodies
    let type_hash_body = generate_type_hash_body(&ctx, &var_type_hashes);

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

    generate_type_hash_traits(
        &ctx,
        &where_clause_type_hash,
        &where_clause_align_hash,
        &where_clause_max_size_of,
        type_hash_body,
        align_hash_body,
        max_size_of_body,
    )
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
    // Cloning input for type hash
    let input_for_type_hash = input.clone();
    let derive_input = parse_macro_input!(input as DeriveInput);
    let (is_repr_c, is_zero_copy, is_deep_copy) = check_attrs(&derive_input);

    let CommonDeriveInput {
        name,
        type_const_ids,
        concat_generics,
        impl_generics,
        ..
    } = CommonDeriveInput::new(derive_input.clone(), vec![]);

    // Values for serialize (we add serialization bounds to generics)
    let CommonDeriveInput {
        impl_generics: generics_serialize,
        ..
    } = CommonDeriveInput::new(derive_input.clone(), vec![]);

    // Values for deserialize (we add deserialization bounds to generics)
    let CommonDeriveInput {
        impl_generics: generics_deserialize,
        ..
    } = CommonDeriveInput::new(derive_input.clone(), vec![]);

    let data = derive_input.data.to_owned();
    let ctx = CodegenContext {
        derive_input,
        name,
        concat_generics,
        type_const_ids,
        impl_generics,
        generics_serialize,
        generics_deserialize,
        is_repr_c,
        is_zero_copy,
        is_deep_copy,
    };

    let mut out: proc_macro::TokenStream = match &data {
        Data::Struct(s) => generate_struct_impl(ctx, s),
        Data::Enum(e) => generate_enum_impl(ctx, e),
        _ => todo!("Union types are not currently supported"),
    }
    .into();

    // automatically derive type hash
    out.extend(epserde_type_hash(input_for_type_hash));
    out
}

/// Generate a partial ε-serde implementation for custom types.
///
/// It generates implementations just for the traits
/// `MaxSizeOf`, `TypeHash`, and `AlignHash`. See the documentation
/// of [`epserde_derive`] for more information.
#[proc_macro_derive(TypeInfo, attributes(zero_copy, deep_copy))]
pub fn epserde_type_hash(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let (_, is_zero_copy, _) = check_attrs(&input);

    let CommonDeriveInput {
        name,
        type_const_ids,
        const_ids,
        impl_generics,
        concat_generics,
        ..
    } = CommonDeriveInput::new(input.clone(), vec![]);

    // Build type name
    let name_literal = name.to_string();

    // Add reprs
    let repr = input
        .attrs
        .iter()
        .filter(|x| x.meta.path().is_ident("repr"))
        .map(|x| x.meta.require_list().unwrap().tokens.to_string())
        .collect::<Vec<_>>();

    let data = input.data.to_owned();
    let ctx = TypeHashContext {
        input,
        name,
        type_const_ids,
        const_ids,
        impl_generics,
        concat_generics,
        is_zero_copy,
        name_literal,
        repr,
    };

    match &data {
        Data::Struct(s) => generate_struct_type_hash(ctx, s),
        Data::Enum(e) => generate_enum_type_hash(ctx, e),
        _ => todo!("Union types are not currently supported"),
    }
    .into()
}
