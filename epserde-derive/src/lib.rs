/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//!
//! Derive procedural macros for the [`epserde`](https://crates.io/crates/epserde) crate.
//!

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput};
use syn::{
    punctuated::Punctuated, token, BoundLifetimes, GenericParam, LifetimeParam, PredicateType,
    WhereClause, WherePredicate,
};

/// Pre-parsed information for the derive macros.
struct CommonDeriveInput {
    /// The identifier of the struct.
    name: syn::Ident,
    /// The token stream to be used after `impl` in angle brackets. It contains
    /// the generics, lifetimes, and consts, with their trait bounds.
    generics: proc_macro2::TokenStream,
    /// A vector containing the identifiers of the generics.
    generics_name_vec: Vec<proc_macro2::TokenStream>,
    /// Same as `generics_name_vec`, but names are concatenated
    /// and separated by commans.
    generics_names: proc_macro2::TokenStream,
    /// A vector containing the name of generics types, represented as strings.
    generics_names_raw: Vec<String>,
    /// A vector containing the identifier of the constants, represented as strings.
    /// Used to include the const values into the [`TypeHash`].
    consts_names_raw: Vec<String>,
    /// The where clause.
    where_clause: proc_macro2::TokenStream,
}

impl CommonDeriveInput {
    /// Create a new `CommonDeriveInput` from a `DeriveInput`.
    /// Additionally, one can specify traits and lifetimes to
    /// be added to the generic types.
    fn new(input: DeriveInput, traits_to_add: Vec<syn::Path>) -> Self {
        let name = input.ident;
        let mut generics = quote!();
        let mut generics_names_raw = vec![];
        let mut consts_names_raw = vec![];
        let mut generics_name_vec = vec![];
        let mut generics_names = quote!();
        if !input.generics.params.is_empty() {
            input.generics.params.into_iter().for_each(|x| {
                match x {
                    syn::GenericParam::Type(mut t) => {
                        generics_names.extend(t.ident.to_token_stream());
                        generics_names_raw.push(t.ident.to_string());

                        t.default = None;
                        for trait_to_add in traits_to_add.iter() {
                            t.bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
                                paren_token: None,
                                modifier: syn::TraitBoundModifier::None,
                                lifetimes: None,
                                path: trait_to_add.clone(),
                            }));
                        }
                        generics.extend(quote!(#t,));
                        generics_name_vec.push(t.ident.to_token_stream());
                    }
                    syn::GenericParam::Lifetime(l) => {
                        generics_names.extend(l.lifetime.to_token_stream());

                        generics.extend(quote!(#l,));
                        generics_name_vec.push(l.lifetime.to_token_stream());
                    }
                    syn::GenericParam::Const(mut c) => {
                        generics_names.extend(c.ident.to_token_stream());
                        consts_names_raw.push(c.ident.to_string());

                        c.default = None; // remove the defaults from the const generics
                                          // otherwise we can't use them in the impl generics
                        generics.extend(quote!(#c,));
                        generics_name_vec.push(c.ident.to_token_stream());
                    }
                };
                generics_names.extend(quote!(,))
            });
        }

        // We add a where keyword in case we need to add clauses
        let where_clause = input
            .generics
            .where_clause
            .map(|x| x.to_token_stream())
            .unwrap_or(quote!(where));

        Self {
            name,
            generics,
            generics_names,
            where_clause,
            generics_names_raw,
            consts_names_raw,
            generics_name_vec,
        }
    }
}

/// Return whether the struct has attributes `repr(C)`, `zero_copy`, and `deep_copy`.
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

#[proc_macro_derive(Epserde, attributes(zero_copy, deep_copy))]
pub fn epserde_derive(input: TokenStream) -> TokenStream {
    // Cloning input for type hash
    let input_for_typehash = input.clone();
    let derive_input = parse_macro_input!(input as DeriveInput);
    let (is_repr_c, is_zero_copy, is_deep_copy) = check_attrs(&derive_input);

    // Common values between serialize and deserialize
    let CommonDeriveInput {
        name,
        generics_names,
        generics_names_raw,
        generics_name_vec,
        generics,
        ..
    } = CommonDeriveInput::new(derive_input.clone(), vec![]);

    // Values for serialize (we add serialization bounds to generics)
    let CommonDeriveInput {
        generics: generics_serialize,
        ..
    } = CommonDeriveInput::new(
        derive_input.clone(),
        vec![syn::parse_quote!(epserde::ser::SerializeInner)],
    );

    // Values for deserialize (we add deserialization bounds to generics)
    let CommonDeriveInput {
        generics: generics_deserialize,
        ..
    } = CommonDeriveInput::new(
        derive_input.clone(),
        vec![syn::parse_quote!(epserde::des::DeserializeInner)],
    );

    let out = match derive_input.data {
        Data::Struct(s) => {
            let mut fields_types = vec![];
            let mut fields_names = vec![];
            let mut non_generic_fields = vec![];
            let mut non_generic_types = vec![];
            let mut generic_fields = vec![];
            let mut generic_types = vec![];

            // Scan the struct to find which fields are generics, and which are not.
            s.fields.iter().for_each(|field| {
                let ty = &field.ty;
                let field_name = field.ident.clone().unwrap();
                if generics_names_raw.contains(&ty.to_token_stream().to_string()) {
                    generic_fields.push(field_name.clone());
                    generic_types.push(ty);
                } else {
                    non_generic_fields.push(field_name.clone());
                    non_generic_types.push(ty);
                }
                fields_types.push(ty);
                fields_names.push(field_name);
            });

            // Assign  ε-copy deserialization or full deserialization to
            // fields depending whether they are generic or not.
            let mut methods: Vec<proc_macro2::TokenStream> = vec![];

            s.fields.iter().for_each(|field| {
                let ty = &field.ty;
                if generics_names_raw.contains(&ty.to_token_stream().to_string()) {
                    methods.push(syn::parse_quote!(_deserialize_eps_copy_inner));
                } else {
                    methods.push(syn::parse_quote!(_deserialize_full_copy_inner));
                }
            });

            // Gather deserialization types of fields,
            // which are necessary to derive the deserialization type.
            let desser_type_generics = generics_name_vec
                .iter()
                .map(|ty| {
                    if generic_types
                        .iter()
                        .any(|x| x.to_token_stream().to_string() == ty.to_string())
                    {
                        quote!(<#ty as epserde::des::DeserializeInner>::DeserType<'epserde_desertype>)
                    } else {
                        ty.clone()
                    }
                })
                .collect::<Vec<_>>();

            let where_clause = derive_input
                .generics
                .where_clause
                .clone()
                .unwrap_or_else(|| WhereClause {
                    where_token: token::Where::default(),
                    predicates: Punctuated::new(),
                });

            let mut where_clause_des = where_clause.clone();

            // We add to the deserialization where clause the bounds on the deserialization
            // types of the fields derived from the bounds of the original types of the fields.
            // TODO: we presently handle only inlined bounds, and not bounds in a where clause.
            derive_input.generics.params.iter().for_each(|param| {
                if let GenericParam::Type(t) = param {
                    let ty = &t.ident;
                    if t.bounds.is_empty() {
                        return;
                    }
                    let mut lifetimes = Punctuated::new();
                    lifetimes.push(GenericParam::Lifetime(LifetimeParam {
                        attrs: vec![],
                        lifetime: syn::Lifetime::new("'epserde_desertype", proc_macro2::Span::call_site()),
                        colon_token: None,
                        bounds: Punctuated::new(),
                    }));
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
                                <#ty as epserde::des::DeserializeInner>::DeserType<'epserde_desertype>
                            ),
                            colon_token: token::Colon::default(),
                            bounds: t.bounds.clone(),
                        }));
                }
            });

            if is_zero_copy {
                quote! {
                    #[automatically_derived]
                    impl<#generics> epserde::traits::CopyType for  #name<#generics_names> #where_clause {
                        type Copy = epserde::traits::Zero;
                    }

                    #[automatically_derived]
                    impl<#generics_serialize> epserde::ser::SerializeInner for #name<#generics_names> #where_clause {
                        // Compute whether the type could be zero copy
                        const IS_ZERO_COPY: bool = #is_repr_c #(
                            && <#fields_types>::IS_ZERO_COPY
                        )*;

                        // The type is declared as zero copy, so a fortiori there is no mismatch.
                        const ZERO_COPY_MISMATCH: bool = false;

                        #[inline(always)]
                        fn _serialize_inner<F: epserde::ser::FieldWrite>(&self, mut backend: F) -> epserde::ser::Result<F> {
                            backend.write_field_zero("zero", self)
                        }
                    }

                    #[automatically_derived]
                    impl<#generics_deserialize> epserde::des::DeserializeInner for #name<#generics_names> #where_clause_des
                    {
                        fn _deserialize_full_copy_inner<R: epserde::des::ReadWithPos>(
                            mut backend: R,
                        ) -> core::result::Result<(Self, R), epserde::des::Error> {
                            use epserde::des::DeserializeInner;
                            backend.deserialize_full_zero::<Self>()
                        }

                        type DeserType<'epserde_desertype> = &'epserde_desertype #name<#(#desser_type_generics,)*>;

                        fn _deserialize_eps_copy_inner(
                            backend: epserde::des::SliceWithPos,
                        ) -> core::result::Result<(Self::DeserType<'_>, epserde::des::SliceWithPos), epserde::des::Error>
                        {
                            backend.deserialize_eps_zero::<Self>()
                        }
                    }
                }
            } else {
                quote! {
                    #[automatically_derived]
                    impl<#generics> epserde::traits::CopyType for  #name<#generics_names> #where_clause {
                        type Copy = epserde::traits::Deep;
                    }

                    #[automatically_derived]
                    impl<#generics_serialize> epserde::ser::SerializeInner for #name<#generics_names> #where_clause {
                        // Compute whether the type could be zero copy
                        const IS_ZERO_COPY: bool = #is_repr_c #(
                            && <#fields_types>::IS_ZERO_COPY
                        )*;

                        // Compute whether the type could be zero copy but it is not declared as such,
                        // and the attribute `deep_copy` is missing.
                        const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#fields_types>::IS_ZERO_COPY)*;

                        #[inline(always)]
                        fn _serialize_inner<F: epserde::ser::FieldWrite>(&self, mut backend: F) -> epserde::ser::Result<F> {
                            if Self::ZERO_COPY_MISMATCH {
                                eprintln!("Type {} is zero copy, but it has not declared as such; use the #deep_copy attribute to silence this warning", core::any::type_name::<Self>());
                            }
                            #(
                                backend = backend.write_field(stringify!(#fields_names), &self.#fields_names)?;
                            )*
                            Ok(backend)
                        }
                    }

                    #[automatically_derived]
                    impl<#generics_deserialize> epserde::des::DeserializeInner for #name<#generics_names> #where_clause_des {
                        fn _deserialize_full_copy_inner<R: epserde::des::ReadWithPos>(
                            backend: R,
                        ) -> core::result::Result<(Self, R), epserde::des::Error> {
                            use epserde::des::DeserializeInner;
                            #(
                                let (#fields_names, backend) = <#fields_types>::_deserialize_full_copy_inner(backend)?;
                            )*
                            Ok((#name{
                                #(#fields_names),*
                            }, backend))
                        }

                        type DeserType<'epserde_desertype> = #name<#(#desser_type_generics,)*>;

                        fn _deserialize_eps_copy_inner(
                            backend: epserde::des::SliceWithPos,
                        ) -> core::result::Result<(Self::DeserType<'_>, epserde::des::SliceWithPos), epserde::des::Error>
                        {
                            use epserde::des::DeserializeInner;
                            #(
                                let (#fields_names, backend) = <#fields_types>::#methods(backend)?;
                            )*
                            Ok((#name{
                                #(#fields_names),*
                            }, backend))
                        }
                    }
                }
            }
        }
        _ => todo!("Missing implementation for union, enum and tuple types"),
    };

    let mut out: TokenStream = out.into();
    // automatically derive type hash
    out.extend(epserde_type_hash(input_for_typehash));
    out
}

#[proc_macro_derive(TypeHash)]
pub fn epserde_type_hash(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let (_, is_zero_copy, _) = check_attrs(&input);

    let CommonDeriveInput {
        name,
        generics,
        generics_names,
        where_clause,
        generics_names_raw,
        consts_names_raw,
        ..
    } = CommonDeriveInput::new(
        input.clone(),
        vec![syn::parse_quote!(epserde::traits::TypeHash)],
    );

    let out = match input.data {
        Data::Struct(s) => {
            let fields_names = s
                .fields
                .iter()
                .map(|field| field.ident.to_owned().unwrap().to_string())
                .collect::<Vec<_>>();

            let fields_types = s
                .fields
                .iter()
                .map(|field| field.ty.to_owned())
                .collect::<Vec<_>>();

            // Build type name
            let type_name: proc_macro2::TokenStream = if generics.is_empty() {
                format!("\"{}\".into()", name)
            } else {
                let mut res = "format!(\"".to_string();
                res += &name.to_string();
                res += "<";
                for _ in 0..generics_names_raw.len() + consts_names_raw.len() {
                    res += "{}, ";
                }
                res.pop();
                res.pop();
                res += ">\",";

                for gn in generics_names_raw.iter() {
                    res += &format!("{}::type_name()", gn);
                    res += ",";
                }
                res.pop();
                res += ")";
                res
            }
            .parse()
            .unwrap();

            let name_literal = format!("\"{}\"", type_name);

            // Add reprs
            let repr = input
                .attrs
                .iter()
                .filter(|x| x.meta.path().is_ident("repr"))
                .map(|x| x.meta.require_list().unwrap().tokens.to_string())
                .collect::<Vec<_>>();

            if is_zero_copy {
                quote! {
                    #[automatically_derived]
                    impl<#generics> epserde::traits::PaddingOf for #name<#generics_names> #where_clause{}

                    impl<#generics> epserde::traits::TypeHash for #name<#generics_names> #where_clause{

                        fn type_hash(
                            type_hasher: &mut impl core::hash::Hasher,
                            repr_hasher: &mut impl core::hash::Hasher,
                        ) {
                            use core::hash::Hash;
                            use epserde::traits::type_info::PaddingOf;
                            // Hash in size and padding.
                            core::mem::size_of::<Self>().hash(repr_hasher);
                            Self::padding_of().hash(repr_hasher);
                            // Hash in ZeroCopy
                            "ZeroCopy".hash(repr_hasher);
                            // Hash in representation data.
                            #(
                                #repr.hash(repr_hasher);
                            )*
                            // Hash in struct and field names.
                            #name_literal.hash(type_hasher);
                            #(
                                #fields_names.hash(type_hasher);
                            )*
                            // Hash in aligments of all fields.
                            /*#(
                               core::mem::align_of::<#fields_types>::hash(repr_hasher);
                            )*/
                            // Recurse on all fields.
                            #(
                                <#fields_types as epserde::traits::TypeHash>::type_hash(
                                    type_hasher,
                                    repr_hasher,
                                );
                            )*
                        }
                    }
                }
            } else {
                quote! {
                    #[automatically_derived]
                    impl<#generics> epserde::traits::PaddingOf for #name<#generics_names> #where_clause{}

                    impl<#generics> epserde::traits::TypeHash for #name<#generics_names> #where_clause{

                        #[inline(always)]
                        fn type_hash(
                            type_hasher: &mut impl core::hash::Hasher,
                            repr_hasher: &mut impl core::hash::Hasher,
                        ) {
                            use core::hash::Hash;
                            // No alignment, so we do not hash in anything.
                            // Hash in DeepCopy
                            "DeepCopy".hash(repr_hasher);
                            // Hash in struct and field names.
                            #name_literal.hash(type_hasher);
                            #(
                                #fields_names.hash(type_hasher);
                            )*
                            // Recurse on all fields.
                            #(
                                <#fields_types as epserde::traits::TypeHash>::type_hash(
                                    type_hasher,
                                    repr_hasher,
                                );
                            )*
                        }
                    }
                }
            }
        }
        _ => todo!(),
    };
    out.into()
}
