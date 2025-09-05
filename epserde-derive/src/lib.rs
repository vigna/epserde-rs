/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!

Derive procedural macros for the [`epserde`](https://crates.io/crates/epserde) crate.

*/

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, token, BoundLifetimes, Data, DeriveInput,
    GenericParam, LifetimeParam, PredicateType, WhereClause, WherePredicate,
};

/// Check if `sub_type` is part of `ty`. we use this function to detect which
/// field types contains generics and thus need to be bounded.
fn is_subtype(ty: &syn::Type, sub_type: &syn::Type) -> bool {
    // early stop if they perfectly match
    if ty == sub_type {
        return true;
    }

    match ty {
        syn::Type::Never(_) => false,
        syn::Type::Verbatim(token) => token.to_string() == sub_type.to_token_stream().to_string(),
        syn::Type::Array(ty) => is_subtype(ty.elem.as_ref(), sub_type),
        syn::Type::Tuple(ty) => ty.elems.iter().any(|x| is_subtype(x, sub_type)),
        syn::Type::Ptr(ty) => is_subtype(ty.elem.as_ref(), sub_type),
        syn::Type::Reference(ty) => is_subtype(ty.elem.as_ref(), sub_type),
        syn::Type::Slice(ty) => is_subtype(ty.elem.as_ref(), sub_type),
        syn::Type::Paren(ty) => is_subtype(ty.elem.as_ref(), sub_type),
        syn::Type::Group(ty) => is_subtype(ty.elem.as_ref(), sub_type),
        syn::Type::BareFn(ty) => {
            ty.inputs.iter().any(|x| is_subtype(&x.ty, sub_type))
                || match ty.output {
                    syn::ReturnType::Default => false,
                    syn::ReturnType::Type(_, ref ty) => is_subtype(ty.as_ref(), sub_type),
                }
        }
        syn::Type::ImplTrait(ty) => ty.bounds.iter().any(|x| match x {
            syn::TypeParamBound::Trait(_) => {
                unimplemented!("This shouldn't happen inside a struct")
            }
            _ => false,
        }),
        syn::Type::Path(ty) => ty.path.segments.iter().any(|x| {
            x.ident == sub_type.to_token_stream().to_string()
                || match x.arguments {
                    syn::PathArguments::None => false,
                    syn::PathArguments::AngleBracketed(ref args) => {
                        args.args.iter().any(|x| match x {
                            syn::GenericArgument::Type(ty) => is_subtype(ty, sub_type),
                            syn::GenericArgument::AssocType(ty) => is_subtype(&ty.ty, sub_type),
                            syn::GenericArgument::Const(_) => false,
                            syn::GenericArgument::Lifetime(_) => false,
                            syn::GenericArgument::AssocConst(_) => false,
                            syn::GenericArgument::Constraint(_) => todo!(),
                            _ => unimplemented!("Non exhaustive"),
                        })
                    }
                    syn::PathArguments::Parenthesized(_) => todo!(),
                }
        }),
        syn::Type::TraitObject(ty) => ty.bounds.iter().any(|x| match x {
            syn::TypeParamBound::Trait(_) => todo!(),
            syn::TypeParamBound::Lifetime(_) => false,
            syn::TypeParamBound::PreciseCapture(_) => false,
            syn::TypeParamBound::Verbatim(ty) => {
                ty.to_string() == sub_type.to_token_stream().to_string()
            }
            _ => unimplemented!("Non exhaustive"),
        }),
        syn::Type::Infer(_) => {
            unimplemented!("We cannot check the covariance of a type to be inferred")
        }
        syn::Type::Macro(_) => unimplemented!("We cannot check the covariance of a macro type"),
        _ => unimplemented!("Non exhaustive"),
    }
}

/// Pre-parsed information for the derive macros.
struct CommonDeriveInput {
    /// The identifier of the struct.
    name: syn::Ident,
    /// The token stream to be used after `impl` in angle brackets. It contains
    /// the generic types, lifetimes, and constants, with their trait bounds.
    impl_generics: proc_macro2::TokenStream,
    /// A vector containing the [types](syn::Type) of the generics.
    generics_types: Vec<syn::Type>,
    /// A vector containing the identifiers of the generics.
    generics_names: Vec<proc_macro2::TokenStream>,
    /// Same as `generics_names`, but names are concatenated
    /// and separated by commas.
    concat_generics: proc_macro2::TokenStream,
    /// A vector containing the name of generics types, represented as strings.
    /// Used to include the identifiers of generic types into the type hash.
    generics_names_raw: Vec<String>,
    /// A vector containing the identifiers of the generic constants.
    /// Used to include the generic constant values into the type hash.
    const_names: Vec<syn::Ident>,
    /// A vector containing the identifier of the generic constants, represented
    /// as strings. Used to include the identifiers of generic constants into
    /// the type hash.
    const_names_raw: Vec<String>,
}

impl CommonDeriveInput {
    /// Create a new `CommonDeriveInput` from a `DeriveInput`.
    /// Additionally, one can specify traits and lifetimes to
    /// be added to the generic types.
    fn new(input: DeriveInput, traits_to_add: Vec<syn::Path>) -> Self {
        let name = input.ident;
        let mut impl_generics = quote!();
        let mut generics_names_raw = vec![];
        let mut generics_names = vec![];
        let mut concat_generics = quote!();
        let mut generics_types: Vec<syn::Type> = vec![];

        let mut const_names = vec![];
        let mut const_names_raw = vec![];

        if !input.generics.params.is_empty() {
            input.generics.params.into_iter().for_each(|x| {
                match x {
                    syn::GenericParam::Type(mut t) => {
                        concat_generics.extend(t.ident.to_token_stream());
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
                        impl_generics.extend(quote!(#t,));
                        generics_names.push(t.ident.to_token_stream());
                        generics_types.push(syn::Type::Verbatim(t.ident.into_token_stream()));
                    }
                    syn::GenericParam::Lifetime(l) => {
                        concat_generics.extend(l.lifetime.to_token_stream());

                        impl_generics.extend(quote!(#l,));
                        generics_names.push(l.lifetime.to_token_stream());
                    }
                    syn::GenericParam::Const(mut c) => {
                        concat_generics.extend(c.ident.to_token_stream());
                        const_names_raw.push(c.ident.to_string());

                        c.default = None; // remove the defaults from the const generics
                                          // otherwise we can't use them in the impl generics
                        impl_generics.extend(quote!(#c,));
                        generics_names.push(c.ident.to_token_stream());
                        const_names.push(c.ident.clone());
                    }
                };
                concat_generics.extend(quote!(,))
            });
        }

        Self {
            name,
            impl_generics,
            generics_types,
            concat_generics,
            generics_names_raw,
            generics_names,
            const_names_raw,
            const_names,
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

/// Check if a type is PhantomDeserData
fn is_phantom_deser_data(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "PhantomDeserData";
        }
    }
    false
}

/// Add trait bounds for associated (de)serialization types based on bounds on
/// type parameters.
fn add_ser_deser_bounds<T: quote::ToTokens>(
    derive_input: &DeriveInput,
    types_with_generics: &[T],
    where_clause_ser: &mut WhereClause,
    where_clause_des: &mut WhereClause,
) {
    // If there are bounded type parameters which are fields of the
    // struct, we need to impose the same bounds on the SerType and on
    // the DeserType.
    derive_input.generics.params.iter().for_each(|param| {
        if let syn::GenericParam::Type(t) = param {
            let ty = &t.ident;

            // We are just interested in types with bounds that are
            // types of fields of the struct.
            //
            // Note that types_with_generics contains also field types
            // *containing* a type parameter, but that just slows down
            // the search.
            if !t.bounds.is_empty()
                && types_with_generics
                    .iter()
                    .any(|x| *ty == x.to_token_stream().to_string())
            {
                // Add a lifetime so we express bounds on DeserType
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
                            <#ty as epserde::deser::DeserializeInner>::DeserType<'epserde_desertype>
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
                            <#ty as epserde::ser::SerializeInner>::SerType
                        ),
                        colon_token: token::Colon::default(),
                        bounds: t.bounds.clone(),
                    }));
            }
        }
    });
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
pub fn epserde_derive(input: TokenStream) -> TokenStream {
    // Cloning input for type hash
    let input_for_type_hash = input.clone();
    let derive_input = parse_macro_input!(input as DeriveInput);
    let (is_repr_c, is_zero_copy, is_deep_copy) = check_attrs(&derive_input);

    // Common values between serialize and deserialize
    let CommonDeriveInput {
        name,
        concat_generics,
        generics_names_raw,
        generics_names,
        impl_generics,
        generics_types,
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

    let out = match &derive_input.data {
        Data::Struct(s) => {
            let mut fields_types = vec![];
            let mut fields_names = vec![];
            let mut fields_without_generics = vec![];
            let mut types_without_generics = vec![];
            let mut fields_with_generics = vec![];
            let mut types_with_generics = vec![];

            // Scan the struct to find which fields contain a generic
            // type (i.e., they are themselves of a generic type,
            // or of a type containing a generic type as a parameter).
            s.fields.iter().enumerate().for_each(|(field_idx, field)| {
                let ty = &field.ty;
                let field_name = field
                    .ident
                    .to_owned()
                    .map(|x| x.to_token_stream())
                    .unwrap_or_else(|| syn::Index::from(field_idx).to_token_stream());

                if generics_types.iter().any(|x| is_subtype(ty, x)) {
                    fields_with_generics.push(field_name.clone());
                    types_with_generics.push(ty);
                } else {
                    fields_without_generics.push(field_name.clone());
                    types_without_generics.push(ty);
                }
                fields_types.push(ty);
                fields_names.push(field_name);
            });

            // Assign ε-copy deserialization or full deserialization to
            // fields depending whether they are generic or not.
            let mut methods: Vec<proc_macro2::TokenStream> = vec![];

            s.fields.iter().for_each(|field| {
                if is_phantom_deser_data(&field.ty) {
                    methods.push(syn::parse_quote!(_deserialize_eps_inner_special));
                } else if generics_names_raw.contains(&field.ty.to_token_stream().to_string()) {
                    methods.push(syn::parse_quote!(_deserialize_eps_inner));
                } else {
                    methods.push(syn::parse_quote!(_deserialize_full_inner));
                }
            });

            // Gather deserialization types of fields, as they are necessary to
            // derive the deserialization type.
            let deser_type_generics = generics_names
                .iter()
                .map(|ty| {
                    if types_with_generics
                        .iter()
                        .any(|x| x.to_token_stream().to_string() == ty.to_string())
                    {
                        quote!(<#ty as epserde::deser::DeserializeInner>::DeserType<'epserde_desertype>)
                    } else {
                        ty.to_token_stream()
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
            let mut where_clause_ser = where_clause.clone();

            fields_types.iter().for_each(|ty| {
                // add that every struct field has to implement SerializeInner
                let mut bounds_ser = Punctuated::new();
                bounds_ser.push(syn::parse_quote!(epserde::ser::SerializeInner));
                where_clause_ser
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: (*ty).clone(),
                        colon_token: token::Colon::default(),
                        bounds: bounds_ser,
                    }));

                // add that every struct field has to implement DeserializeInner
                let mut bounds_des = Punctuated::new();
                bounds_des.push(syn::parse_quote!(epserde::deser::DeserializeInner));
                where_clause_des
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        lifetimes: None,
                        bounded_ty: (*ty).clone(),
                        colon_token: token::Colon::default(),
                        bounds: bounds_des,
                    }));
            });

            // Map recursively type parameters to their SerType to generate this
            // type's SerType
            let ser_type_generics = generics_names
                .iter()
                .map(|ty| {
                    if types_with_generics
                        .iter()
                        .any(|x| x.to_token_stream().to_string() == ty.to_string())
                    {
                        quote!(<#ty as epserde::ser::SerializeInner>::SerType)
                    } else {
                        ty.to_token_stream()
                    }
                })
                .collect::<Vec<_>>();

            if is_zero_copy {
                // In zero-copy types we do not need to add bounds to
                // the associated SerType/DeserType, as generics are not
                // replaced with their SerType/DeserType.

                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::CopyType for  #name<#concat_generics> #where_clause {
                        type Copy = epserde::traits::Zero;
                    }

                    #[automatically_derived]
                    impl<#generics_serialize> epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
                        type SerType = Self;
                        // Compute whether the type could be zero copy
                        const IS_ZERO_COPY: bool = #is_repr_c #(
                            && <#fields_types>::IS_ZERO_COPY
                        )*;

                        // The type is declared as zero copy, so a fortiori there is no mismatch.
                        const ZERO_COPY_MISMATCH: bool = false;

                        #[inline(always)]
                        unsafe fn _serialize_inner(&self, backend: &mut impl epserde::ser::WriteWithNames) -> epserde::ser::Result<()> {
                            // No-op code that however checks that all fields are zero-copy.
                            fn test<T: epserde::traits::ZeroCopy>() {}
                            #(
                                test::<#fields_types>();
                            )*
                            epserde::ser::helpers::serialize_zero(backend, self)
                        }
                    }

                    // SAFETY: &'epserde_desertype Self is covariant
                    #[automatically_derived]
                    impl<#generics_deserialize> epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des
                    {
                        unsafe fn _deserialize_full_inner(
                            backend: &mut impl epserde::deser::ReadWithPos,
                        ) -> core::result::Result<Self, epserde::deser::Error> {
                            use epserde::deser::DeserializeInner;
                            epserde::deser::helpers::deserialize_full_zero::<Self>(backend)
                        }

                        type DeserType<'epserde_desertype> = &'epserde_desertype Self;

                        unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                            backend: &mut epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                        ) -> core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, epserde::deser::Error>
                        {
                            epserde::deser::helpers::deserialize_eps_zero::<Self>(backend)
                        }
                    }
                }
            } else {
                add_ser_deser_bounds(
                    &derive_input,
                    &types_with_generics,
                    &mut where_clause_ser,
                    &mut where_clause_des,
                );

                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::CopyType for  #name<#concat_generics> #where_clause {
                        type Copy = epserde::traits::Deep;
                    }

                    #[automatically_derived]
                    impl<#generics_serialize> epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
                        type SerType =  #name<#(#ser_type_generics,)*>;
                        // Compute whether the type could be zero copy
                        const IS_ZERO_COPY: bool = #is_repr_c #(
                            && <#fields_types>::IS_ZERO_COPY
                        )*;

                        // Compute whether the type could be zero copy but it is not declared as such,
                        // and the attribute `deep_copy` is missing.
                        const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#fields_types>::IS_ZERO_COPY)*;

                        #[inline(always)]
                        unsafe fn _serialize_inner(&self, backend: &mut impl epserde::ser::WriteWithNames) -> epserde::ser::Result<()> {
                            epserde::ser::helpers::check_mismatch::<Self>();
                            #(
                                backend.write(stringify!(#fields_names), &self.#fields_names)?;
                            )*
                            Ok(())
                        }
                    }

                    // SAFETY: #name is a struct, so it is covariant
                    #[automatically_derived]
                    impl<#generics_deserialize> epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des {
                        unsafe fn _deserialize_full_inner(
                            backend: &mut impl epserde::deser::ReadWithPos,
                        ) -> core::result::Result<Self, epserde::deser::Error> {
                            use epserde::deser::DeserializeInner;
                            Ok(#name{
                                #(
                                    #fields_names: unsafe { <#fields_types>::_deserialize_full_inner(backend)? },
                                )*
                            })
                        }

                        type DeserType<'epserde_desertype> = #name<#(#deser_type_generics,)*>;

                        unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                            backend: &mut epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                        ) -> core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, epserde::deser::Error>
                        {
                            use epserde::deser::DeserializeInner;
                            Ok(#name{
                                #(
                                    #fields_names: <#fields_types>::#methods(backend)?,
                                )*
                            })
                        }
                    }
                }
            }
        }
        Data::Enum(e) => {
            let where_clause = derive_input
                .generics
                .where_clause
                .clone()
                .unwrap_or_else(|| WhereClause {
                    where_token: token::Where::default(),
                    predicates: Punctuated::new(),
                });

            let mut variants_names = Vec::new();
            let mut variants = Vec::new();
            let mut variant_ser = Vec::new();
            let mut where_clause_ser = where_clause.clone();
            let mut where_clause_des = where_clause.clone();
            let mut variant_full_des = Vec::new();
            let mut variant_eps_des = Vec::new();
            let mut types_with_generics = Vec::new();
            let mut fields_with_generics = Vec::new();
            let mut fields_without_generics = Vec::new();
            let mut types_without_generics = Vec::new();
            let mut fields_types = Vec::new();
            e.variants.iter().enumerate().for_each(|(variant_id, variant)| {
                variants_names.push(variant.ident.to_token_stream());
                match &variant.fields {
                syn::Fields::Unit => {
                    variants.push(variant.ident.to_token_stream());
                    variant_ser.push(quote! {{
                        backend.write("tag", &#variant_id)?;
                    }});
                    variant_full_des.push(quote! {});
                    variant_eps_des.push(quote! {});
                }
                syn::Fields::Named(fields) => {
                    let mut var_fields_names = Vec::new();
                    let mut var_fields_types = Vec::new();
                    let mut methods: Vec<proc_macro2::TokenStream> = vec![];
                    fields
                        .named
                        .iter()
                        .map(|named| (named.ident.as_ref().unwrap(), &named.ty))
                        .for_each(|(ident, ty)| {
                            if generics_types.iter().any(|x| is_subtype(ty, x)) {
                                fields_with_generics.push(ident.to_token_stream());
                                types_with_generics.push(ty.to_token_stream());
                            } else {
                                fields_without_generics.push(ident.to_token_stream());
                                types_without_generics.push(ty.to_token_stream());
                            }

                            var_fields_names.push(ident.to_token_stream());
                            var_fields_types.push(ty.to_token_stream());

                            // add that every struct field has to implement SerializeInner
                            let mut bounds_ser = Punctuated::new();
                            bounds_ser.push(syn::parse_quote!(epserde::ser::SerializeInner));
                            where_clause_ser
                                .predicates
                                .push(WherePredicate::Type(PredicateType {
                                    lifetimes: None,
                                    bounded_ty: ty.clone(),
                                    colon_token: token::Colon::default(),
                                    bounds: bounds_ser,
                            }));
                            // add that every struct field has to implement DeserializeInner
                            let mut bounds_des = Punctuated::new();
                            bounds_des.push(syn::parse_quote!(epserde::deser::DeserializeInner));
                            where_clause_des
                                .predicates
                                .push(WherePredicate::Type(PredicateType {
                                    lifetimes: None,
                                    bounded_ty: ty.clone(),
                                    colon_token: token::Colon::default(),
                                    bounds: bounds_des,
                            }));

                            if is_phantom_deser_data(ty) {
                                methods.push(syn::parse_quote!(_deserialize_eps_inner_special));
                            } else if generics_names_raw.contains(&ty.to_token_stream().to_string()) {
                                methods.push(syn::parse_quote!(_deserialize_eps_inner));
                            } else {
                                methods.push(syn::parse_quote!(_deserialize_full_inner));
                            }
                        });
                    let ident = variant.ident.clone();
                    variants.push(quote! {
                        #ident{ #( #var_fields_names, )* }
                    });
                    fields_types.extend(var_fields_types.clone());
                    variant_ser.push(quote! {
                        backend.write("tag", &#variant_id)?;
                        #(
                            backend.write(stringify!(#var_fields_names), #var_fields_names)?;
                        )*
                    });
                    variant_full_des.push(quote! {
                        #(
                            #var_fields_names: unsafe { <#var_fields_types>::_deserialize_full_inner(backend)? },
                        )*
                    });
                    variant_eps_des.push(quote! {
                        #(
                            #var_fields_names: unsafe { <#var_fields_types>::#methods(backend)? },
                        )*
                    });
                }
                syn::Fields::Unnamed(fields) => {
                    let mut var_fields_names = Vec::new();
                    let mut var_fields_vars = Vec::new();
                    let mut var_fields_types = Vec::new();
                    let mut methods: Vec<proc_macro2::TokenStream> = vec![];

                    fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .for_each(|(field_idx, unnamed)| {
                            let ty = &unnamed.ty;
                            let ident = syn::Index::from(field_idx);
                            if generics_types.iter().any(|x| is_subtype(ty, x)) {
                                fields_with_generics.push(ident.to_token_stream());
                                types_with_generics.push(ty.to_token_stream());
                            } else {
                                fields_without_generics.push(ident.to_token_stream());
                                types_without_generics.push(ty.to_token_stream());
                            }

                            var_fields_names.push(syn::Ident::new(
                                &format!("v{}", field_idx),
                                proc_macro2::Span::call_site(),
                            )
                            .to_token_stream());
                            var_fields_vars.push(syn::Index::from(field_idx));
                            var_fields_types.push(ty.to_token_stream());


                            // add that every struct field has to implement SerializeInner
                            let mut bounds_ser = Punctuated::new();
                            bounds_ser.push(syn::parse_quote!(epserde::ser::SerializeInner));
                            where_clause_ser
                                .predicates
                                .push(WherePredicate::Type(PredicateType {
                                    lifetimes: None,
                                    bounded_ty: ty.clone(),
                                    colon_token: token::Colon::default(),
                                    bounds: bounds_ser,
                            }));
                            // add that every struct field has to implement DeserializeInner
                            let mut bounds_des = Punctuated::new();
                            bounds_des.push(syn::parse_quote!(epserde::deser::DeserializeInner));
                            where_clause_des
                                .predicates
                                .push(WherePredicate::Type(PredicateType {
                                    lifetimes: None,
                                    bounded_ty: ty.clone(),
                                    colon_token: token::Colon::default(),
                                    bounds: bounds_des,
                            }));

                            if generics_names_raw.contains(&ty.to_token_stream().to_string()) {
                                methods.push(syn::parse_quote!(_deserialize_eps_inner));
                            } else {
                                methods.push(syn::parse_quote!(_deserialize_full_inner));
                            }

                        });

                    let ident = variant.ident.clone();
                    variants.push(quote! {
                        #ident( #( #var_fields_names, )* )
                    });
                    fields_types.extend(var_fields_types.clone());

                    variant_ser.push(quote! {
                        backend.write("tag", &#variant_id)?;
                        #(
                            backend.write(stringify!(#var_fields_names), #var_fields_names)?;
                        )*
                    });
                    variant_full_des.push(quote! {
                        #(
                            #var_fields_vars    : unsafe { <#var_fields_types>::_deserialize_full_inner(backend)? },
                        )*
                    });
                    variant_eps_des.push(quote! {
                        #(
                            #var_fields_vars    : unsafe { <#var_fields_types>::#methods(backend)? },
                        )*
                    });
                }
            }});

            // Gather deserialization types of fields,
            // which are necessary to derive the deserialization type.
            let deser_type_generics = generics_names
                .iter()
                .map(|ty| {
                    if types_with_generics
                        .iter()
                        .any(|x| x.to_token_stream().to_string() == ty.to_string())
                    {
                        quote!(<#ty as epserde::deser::DeserializeInner>::DeserType<'epserde_desertype>)
                    } else {
                        ty.clone()
                    }
                })
                .collect::<Vec<_>>();
            let ser_type_generics = generics_names
                .iter()
                .map(|ty| {
                    if types_with_generics
                        .iter()
                        .any(|x| x.to_token_stream().to_string() == ty.to_string())
                    {
                        quote!(<#ty as epserde::ser::SerializeInner>::SerType)
                    } else {
                        ty.clone()
                    }
                })
                .collect::<Vec<_>>();
            let tag = (0..variants.len()).collect::<Vec<_>>();

            if is_zero_copy {
                // In zero-copy types we do not need to add bounds to
                // the associated SerType/DeserType, as generics are not
                // replaced with their SerType/DeserType.

                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::CopyType for  #name<#concat_generics> #where_clause {
                        type Copy = epserde::traits::Zero;
                    }
                    #[automatically_derived]
                    impl<#generics_serialize> epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
                        type SerType = Self;

                        // Compute whether the type could be zero copy
                        const IS_ZERO_COPY: bool = #is_repr_c #(
                            && <#fields_types>::IS_ZERO_COPY
                        )*;

                        // The type is declared as zero copy, so a fortiori there is no mismatch.
                        const ZERO_COPY_MISMATCH: bool = false;
                        #[inline(always)]
                        unsafe fn _serialize_inner(&self, backend: &mut impl epserde::ser::WriteWithNames) -> epserde::ser::Result<()> {
                            // No-op code that however checks that all fields are zero-copy.
                            fn test<T: epserde::traits::ZeroCopy>() {}
                            #(
                                test::<#fields_types>();
                            )*
                            epserde::ser::helpers::serialize_zero(backend, self)
                        }
                    }

                    // SAFETY: &'epserde_desertype Self is covariant
                    #[automatically_derived]
                    impl<#generics_deserialize> epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des {
                        unsafe fn _deserialize_full_inner(
                            backend: &mut impl epserde::deser::ReadWithPos,
                        ) -> core::result::Result<Self, epserde::deser::Error> {
                            epserde::deser::helpers::deserialize_full_zero::<Self>(backend)
                        }

                        type DeserType<'epserde_desertype> = &'epserde_desertype Self;

                        unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                            backend: &mut epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                        ) -> core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, epserde::deser::Error>
                        {
                            epserde::deser::helpers::deserialize_eps_zero::<Self>(backend)
                        }
                    }
                }
            } else {
                add_ser_deser_bounds(
                    &derive_input,
                    &types_with_generics,
                    &mut where_clause_ser,
                    &mut where_clause_des,
                );

                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::CopyType for  #name<#concat_generics> #where_clause {
                        type Copy = epserde::traits::Deep;
                    }
                    #[automatically_derived]
                    impl<#generics_serialize> epserde::ser::SerializeInner for #name<#concat_generics> #where_clause_ser {
                        type SerType =  #name<#(#ser_type_generics,)*>;

                        // Compute whether the type could be zero copy
                        const IS_ZERO_COPY: bool = #is_repr_c #(
                            && <#fields_types>::IS_ZERO_COPY
                        )*;

                        // Compute whether the type could be zero copy but it is not declared as such,
                        // and the attribute `deep_copy` is missing.
                        const ZERO_COPY_MISMATCH: bool = ! #is_deep_copy #(&& <#fields_types>::IS_ZERO_COPY)*;
                        #[inline(always)]
                        unsafe fn _serialize_inner(&self, backend: &mut impl epserde::ser::WriteWithNames) -> epserde::ser::Result<()> {
                            epserde::ser::helpers::check_mismatch::<Self>();
                            match self {
                                #(
                                   Self::#variants => { #variant_ser }
                                )*
                            }
                            Ok(())
                        }
                    }
                    // SAFETY: #name is an enum, so it is covariant
                    #[automatically_derived]
                    impl<#generics_deserialize> epserde::deser::DeserializeInner for #name<#concat_generics> #where_clause_des {
                        unsafe fn _deserialize_full_inner(
                            backend: &mut impl epserde::deser::ReadWithPos,
                        ) -> core::result::Result<Self, epserde::deser::Error> {
                            use epserde::deser::DeserializeInner;
                            match unsafe { usize::_deserialize_full_inner(backend)? } {
                                #(
                                    #tag => Ok(Self::#variants_names{ #variant_full_des }),
                                )*
                                tag => Err(epserde::deser::Error::InvalidTag(tag)),
                            }
                        }

                        type DeserType<'epserde_desertype> = #name<#(#deser_type_generics,)*>;

                        unsafe fn _deserialize_eps_inner<'deserialize_eps_inner_lifetime>(
                            backend: &mut epserde::deser::SliceWithPos<'deserialize_eps_inner_lifetime>,
                        ) -> core::result::Result<Self::DeserType<'deserialize_eps_inner_lifetime>, epserde::deser::Error>
                        {
                            use epserde::deser::DeserializeInner;
                            match unsafe { usize::_deserialize_full_inner(backend)? } {
                                #(
                                    #tag => Ok(Self::DeserType::<'_>::#variants_names{ #variant_eps_des }),
                                )*
                                tag => Err(epserde::deser::Error::InvalidTag(tag)),
                            }
                        }
                    }
                }
            }
        }
        _ => todo!("Union types are not currently supported"),
    };

    let mut out: TokenStream = out.into();
    // automatically derive type hash
    out.extend(epserde_type_hash(input_for_type_hash));
    out
}

fn type_repr_max_size_of_where_clauses(
    where_clause: &WhereClause,
    generic_types: &[syn::Type],
) -> (WhereClause, WhereClause, WhereClause) {
    let mut bounds_type_hash = Punctuated::new();
    bounds_type_hash.push(syn::parse_quote!(epserde::traits::TypeHash));
    let mut where_clause_type_hash = where_clause.clone();

    let mut bounds_align_hash = Punctuated::new();
    bounds_align_hash.push(syn::parse_quote!(epserde::traits::AlignHash));
    let mut where_clause_align_hash = where_clause.clone();

    let mut bounds_max_size_of = Punctuated::new();
    bounds_max_size_of.push(syn::parse_quote!(epserde::traits::MaxSizeOf));
    let mut where_clause_max_size_of = where_clause.clone();

    generic_types.iter().for_each(|ty| {
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
    });
    (
        where_clause_type_hash,
        where_clause_align_hash,
        where_clause_max_size_of,
    )
}

/// Generate a partial ε-serde implementation for custom types.
///
/// It generates implementations just for the traits
/// `MaxSizeOf`, `TypeHash`, and `AlignHash`. See the documentation
/// of [`epserde_derive`] for more information.
#[proc_macro_derive(TypeInfo, attributes(zero_copy, deep_copy))]
pub fn epserde_type_hash(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let (_, is_zero_copy, _) = check_attrs(&input);

    let CommonDeriveInput {
        name,
        impl_generics,
        concat_generics,
        generics_types,
        const_names,
        const_names_raw,
        ..
    } = CommonDeriveInput::new(input.clone(), vec![]);

    let where_clause = input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(|| WhereClause {
            where_token: token::Where::default(),
            predicates: Punctuated::new(),
        });

    let out = match input.data {
        Data::Struct(s) => {
            //these are the type of the fields that contains generics parameters
            let mut generic_types = vec![];
            // Compute which fields types are super-types of the generic parameters
            s.fields.iter().for_each(|field| {
                let ty = &field.ty;
                if generics_types.iter().any(|x| is_subtype(ty, x)) {
                    generic_types.push(ty.clone());
                }
            });

            let (where_clause_type_hash, where_clause_align_hash, where_clause_max_size_of) =
                type_repr_max_size_of_where_clauses(&where_clause, &generic_types);

            let fields_names = s
                .fields
                .iter()
                .enumerate()
                .map(|(field_idx, field)| {
                    field
                        .ident
                        .as_ref()
                        .map(|ident| ident.to_string())
                        .unwrap_or_else(|| field_idx.to_string())
                })
                .collect::<Vec<_>>();

            let fields_types = s
                .fields
                .iter()
                .map(|field| field.ty.to_owned())
                .collect::<Vec<_>>();

            // Build type name
            let name_literal = name.to_string();

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
                    impl<#impl_generics> epserde::traits::TypeHash for #name<#concat_generics> #where_clause_type_hash {

                        #[inline(always)]
                        fn type_hash(
                            hasher: &mut impl core::hash::Hasher,
                        ) {
                            use core::hash::Hash;
                            // Hash in ZeroCopy
                            "ZeroCopy".hash(hasher);
                            // Hash the values of generic constants
                            #(
                                #const_names.hash(hasher);
                            )*
                            // Hash the identifiers of generic constants
                            #(
                                #const_names_raw.hash(hasher);
                            )*
                            // Hash in struct and field names.
                            #name_literal.hash(hasher);
                            #(
                                #fields_names.hash(hasher);
                            )*
                            // Recurse on all fields.
                            #(
                                <#fields_types as epserde::traits::TypeHash>::type_hash(hasher);
                            )*
                        }
                    }
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::AlignHash for #name<#concat_generics> #where_clause_align_hash{
                        #[inline(always)]
                        fn align_hash(
                            hasher: &mut impl core::hash::Hasher,
                            offset_of: &mut usize,
                        ) {
                            use core::hash::Hash;
                            // Hash in size, as padding is given by MaxSizeOf.
                            // and it is independent of the architecture.
                            core::mem::size_of::<Self>().hash(hasher);
                            // Hash in representation data.
                            #(
                                #repr.hash(hasher);
                            )*
                            // Recurse on all fields.
                            #(
                                <#fields_types as epserde::traits::AlignHash>::align_hash(
                                    hasher,
                                    offset_of,
                                );
                            )*
                        }
                    }
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::MaxSizeOf for #name<#concat_generics> #where_clause_max_size_of{
                        #[inline(always)]
                        fn max_size_of() -> usize {
                            let mut max_size_of = std::mem::align_of::<Self>();
                            // Recurse on all fields.
                            #(
                                if max_size_of < <#fields_types as epserde::traits::MaxSizeOf>::max_size_of() {
                                    max_size_of = <#fields_types as epserde::traits::MaxSizeOf>::max_size_of();
                                }
                            )*
                            max_size_of
                        }
                    }
                }
            } else {
                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::TypeHash for #name<#concat_generics> #where_clause_type_hash{

                        #[inline(always)]
                        fn type_hash(
                            hasher: &mut impl core::hash::Hasher,
                        ) {
                            use core::hash::Hash;
                            // No alignment, so we do not hash in anything.
                            // Hash in DeepCopy
                            "DeepCopy".hash(hasher);
                            // Hash the values of generic constants
                            #(
                                #const_names.hash(hasher);
                            )*
                            // Hash the identifiers of generic constants
                            #(
                                #const_names_raw.hash(hasher);
                            )*
                            // Hash in struct and field names.
                            #name_literal.hash(hasher);
                            #(
                                #fields_names.hash(hasher);
                            )*
                            // Recurse on all fields.
                            #(
                                <#fields_types as epserde::traits::TypeHash>::type_hash(hasher);
                            )*
                        }
                    }
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::AlignHash for #name<#concat_generics> #where_clause_align_hash {
                        #[inline(always)]
                        fn align_hash(
                            hasher: &mut impl core::hash::Hasher,
                            _offset_of: &mut usize,
                        ) {
                            // Recurse on all variants starting at offset 0
                            #(
                                <#fields_types as epserde::traits::AlignHash>::align_hash(hasher, &mut 0);
                            )*
                        }
                    }
                }
            }
        }
        Data::Enum(e) => {
            let where_clause = input
                .generics
                .where_clause
                .clone()
                .unwrap_or_else(|| WhereClause {
                    where_token: token::Where::default(),
                    predicates: Punctuated::new(),
                });

            let mut var_type_hashes = Vec::new();
            let mut var_align_hashes = Vec::new();
            let mut var_max_size_ofs = Vec::new();
            let mut generic_types = vec![];

            e.variants.iter().for_each(|variant| {
                let ident = variant.ident.to_owned();
                let mut var_type_hash = quote! { stringify!(#ident).hash(hasher); };
                let mut var_align_hash = quote! { };
                let mut var_max_size_of = quote! {  };
                match &variant.fields {
                    syn::Fields::Unit => {}
                    syn::Fields::Named(fields) => {
                        fields
                            .named
                            .iter()
                            .map(|named| {
                                (named.ident.as_ref().unwrap(), named.ty.clone())
                            })
                            .for_each(|(ident, ty)| {
                                var_type_hash.extend([quote! {
                                    stringify!(#ident).hash(hasher);
                                    <#ty as epserde::traits::TypeHash>::type_hash(hasher);
                                }]);
                                var_align_hash.extend([quote! {
                                    <#ty as epserde::traits::AlignHash>::align_hash(hasher, offset_of);
                                }]);
                                var_max_size_of.extend([
                                    quote! {
                                        if max_size_of < <#ty as epserde::traits::MaxSizeOf>::max_size_of() {
                                            max_size_of = <#ty as epserde::traits::MaxSizeOf>::max_size_of();
                                        }
                                    }
                                ]);
                                if generics_types.iter().any(|x| is_subtype(&ty, x)) {
                                    generic_types.push(ty);
                                }
                            });
                    }
                    syn::Fields::Unnamed(fields) => {
                        fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .for_each(|(field_idx, unnamed)| {
                                let ty = &unnamed.ty;
                                let field_name = field_idx.to_string();
                                var_type_hash.extend([quote! {
                                    #field_name.hash(hasher);
                                    <#ty as epserde::traits::TypeHash>::type_hash(hasher);
                                }]);
                                var_align_hash.extend([quote! {
                                    <#ty as epserde::traits::AlignHash>::align_hash(hasher, offset_of);
                                }]);
                                var_max_size_of.extend([
                                    quote! {
                                        if max_size_of < <#ty as epserde::traits::MaxSizeOf>::max_size_of() {
                                            max_size_of = <#ty as epserde::traits::MaxSizeOf>::max_size_of();
                                        }
                                    }
                                ]);
                                if generics_types.iter().any(|x| is_subtype(ty, x)) {
                                    generic_types.push(ty.clone());
                                }
                            });
                    }
                }
                var_type_hashes.push(var_type_hash);
                var_align_hashes.push(var_align_hash);
                var_max_size_ofs.push(var_max_size_of);
            });

            // Build type name
            let name_literal = name.to_string();

            // Add reprs
            let repr = input
                .attrs
                .iter()
                .filter(|x| x.meta.path().is_ident("repr"))
                .map(|x| x.meta.require_list().unwrap().tokens.to_string())
                .collect::<Vec<_>>();

            let (where_clause_type_hash, where_clause_align_hash, where_clause_max_size_of) =
                type_repr_max_size_of_where_clauses(&where_clause, &generic_types);

            if is_zero_copy {
                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::TypeHash for #name<#concat_generics> #where_clause_type_hash {

                        #[inline(always)]
                        fn type_hash(
                            hasher: &mut impl core::hash::Hasher,
                        ) {
                            use core::hash::Hash;
                            // Hash in ZeroCopy
                            "ZeroCopy".hash(hasher);
                            // Hash the values of generic constants
                            #(
                                #const_names.hash(hasher);
                            )*
                            // Hash the identifiers of generic constants
                            #(
                                #const_names_raw.hash(hasher);
                            )*
                            // Hash in struct and field names.
                            #name_literal.hash(hasher);
                            #(
                                #var_type_hashes
                            )*
                        }
                    }
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::AlignHash for #name<#concat_generics> #where_clause_align_hash {
                        #[inline(always)]
                        fn align_hash(
                            hasher: &mut impl core::hash::Hasher,
                            offset_of: &mut usize,
                        ) {
                            use core::hash::Hash;
                            // Hash in size, as padding is given by MaxSizeOf.
                            // and it is independent of the architecture.
                            core::mem::size_of::<Self>().hash(hasher);
                            // Hash in representation data.
                            #(
                                #repr.hash(hasher);
                            )*
                            // Recurse on all fields.
                            let old_offset_of = *offset_of;
                            #(
                                *offset_of = old_offset_of;
                                #var_align_hashes
                            )*
                        }
                    }
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::MaxSizeOf for #name<#concat_generics> #where_clause_max_size_of{
                        #[inline(always)]
                        fn max_size_of() -> usize {
                            let mut max_size_of = std::mem::align_of::<Self>();
                            #(
                                #var_max_size_ofs
                            )*
                            max_size_of
                        }
                    }
                }
            } else {
                quote! {
                    #[automatically_derived]
                    impl<#impl_generics> epserde::traits::TypeHash for #name<#concat_generics> #where_clause_type_hash{

                        #[inline(always)]
                        fn type_hash(
                            hasher: &mut impl core::hash::Hasher,
                        ) {
                            use core::hash::Hash;
                            // No alignment, so we do not hash in anything.
                            // Hash in DeepCopy
                            "DeepCopy".hash(hasher);
                            // Hash the values of generic constants
                            #(
                                #const_names.hash(hasher);
                            )*
                            // Hash the identifiers of generic constants
                            #(
                                #const_names_raw.hash(hasher);
                            )*
                            // Hash in struct and field names.
                            #name_literal.hash(hasher);
                            #(
                                #var_type_hashes
                            )*
                        }
                    }

                    impl<#impl_generics> epserde::traits::AlignHash for #name<#concat_generics> #where_clause_align_hash {
                        #[inline(always)]
                        fn align_hash(
                            hasher: &mut impl core::hash::Hasher,
                            offset_of: &mut usize,
                        ) {
                            // Recurse on all variants starting at offset 0
                            // Note that we share var_align_hashes with the
                            // zero-copy case, so we cannot pass &mut 0.

                            #(
                                *offset_of = 0;
                                #var_align_hashes
                            )*
                        }
                    }
                }
            }
        }
        _ => todo!("Union types are not currently supported"),
    };
    out.into()
}
