/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! [`Epserde`] derive code for enum types.
//!
//! [`Epserde`]: derive@crate::Epserde

use quote::{ToTokens, quote};

use super::EpserdeContext;
use super::classify::FieldClassification;
use super::helpers::{
    EpserdeParts, gen_eps_deser_method_call, gen_epserde_parts, gen_zero_copy_impl,
};
use crate::attrs::is_force_full_copy;

/// [`Epserde`] derive code for enum types.
///
/// [`Epserde`]: derive@crate::Epserde
pub(crate) fn gen_epserde_enum_impl(
    ctx: &EpserdeContext,
    e: &syn::DataEnum,
) -> proc_macro2::TokenStream {
    let mut variant_ids = vec![];
    // For each variant, a match arm as a TokenStream
    let mut variant_arm = vec![];
    // For each variant, serialization code
    let mut variant_ser = vec![];
    // For each variant, full-copy deserialization code
    let mut variant_full_des = vec![];
    // For each variant, ε-copy deserialization code
    let mut variant_eps_des = vec![];
    // All field types for all variants
    let mut all_fields_types = vec![];
    // Whether each entry in all_fields_types is full-copy.
    let mut all_full_deser_fields = vec![];
    let mut cls = FieldClassification::default();

    for (variant_id, variant) in e.variants.iter().enumerate() {
        let ident = &variant.ident;
        variant_ids.push(ident);

        match &variant.fields {
            syn::Fields::Unit => {
                variant_arm.push(quote! { #ident });
                variant_ser.push(quote! {{
                    unsafe { WriteWithNames::write(backend, "tag", &#variant_id)? };
                }});
                variant_full_des.push(quote! {});
                variant_eps_des.push(quote! {});
            }
            syn::Fields::Named(fields) => {
                // The code in this arm is almost identical to the code for the
                // next one, except for the handling of field names.
                let mut field_names = vec![];
                // Bindings for the match arm: field names are rebound to
                // reserved identifiers so that a field named, e.g., backend
                // cannot shadow the writer parameter of _ser_inner.
                let mut field_bindings = vec![];
                let mut field_types = vec![];
                let mut method_calls = vec![];

                for field in &fields.named {
                    // It's a named field
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = &field.ty;
                    let force_full_copy = is_force_full_copy(field);

                    let deser_full = cls.classify_field(
                        ctx,
                        &format_args!("{ident}::{field_name}"),
                        field_type,
                        force_full_copy,
                    );

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        deser_full,
                    ));
                    field_names.push(quote! { #field_name });
                    // The raw prefix, if any, must be stripped, or the
                    // generated identifier would be invalid (e.g., for a
                    // field named r#type).
                    field_bindings.push(syn::Ident::new(
                        &format!(
                            "__ඞඞඞepserdeඞඞඞ_field_{}",
                            syn::ext::IdentExt::unraw(field_name)
                        ),
                        field_name.span(),
                    ));
                    field_types.push(field_type);
                    all_full_deser_fields.push(deser_full);
                }

                all_fields_types.extend(&field_types);

                variant_arm.push(quote! {
                    #ident{ #( #field_names: #field_bindings, )* }
                });

                variant_ser.push(quote! {
                    unsafe { WriteWithNames::write(backend, "tag", &#variant_id)? };
                    #(
                        unsafe { WriteWithNames::write(backend, stringify!(#field_names), #field_bindings)? };
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
                    let force_full_copy = is_force_full_copy(field);

                    let deser_full = cls.classify_field(
                        ctx,
                        &format_args!("{ident}::{field_idx}"),
                        field_type,
                        force_full_copy,
                    );

                    field_indices.push(
                        syn::Ident::new(&format!("v{}", field_idx), proc_macro2::Span::call_site())
                            .to_token_stream(),
                    );

                    method_calls.push(gen_eps_deser_method_call(
                        &field_name.to_token_stream(),
                        field_type,
                        deser_full,
                    ));
                    field_types.push(field_type);
                    field_names_in_arm.push(field_name);
                    all_full_deser_fields.push(deser_full);
                }

                all_fields_types.extend(&field_types);

                variant_arm.push(quote! {
                    #ident( #( #field_indices, )* )
                });

                variant_ser.push(quote! {
                    unsafe { WriteWithNames::write(backend, "tag", &#variant_id)? };
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

    let EpserdeParts {
        generics_for_ser_type,
        generics_for_deser_type,
        fixed_point_check,
        full_copy_consistency_check,
        seq_deep_check,
        is_zero_copy_expr,
        could_be_zero_copy,
        ser_where_clause,
        deser_where_clause,
    } = gen_epserde_parts(ctx, &cls, &all_fields_types, &all_full_deser_fields);
    let tag = (0..variant_arm.len()).collect::<Vec<_>>();

    let name = &ctx.derive_input.ident;
    let is_deep_copy = ctx.is_deep_copy;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;
    let where_clause = &ctx.where_clause;

    if ctx.is_zero_copy {
        gen_zero_copy_impl(
            ctx,
            &is_zero_copy_expr,
            &ser_where_clause,
            &deser_where_clause,
        )
    } else {
        let name_str = name.to_string();

        quote! {
            #seq_deep_check

            #[automatically_derived]
            unsafe impl #generics_for_impl ::epserde::traits::CopyType for #name #generics_for_type #where_clause {
                type Copy = ::epserde::traits::Deep;
            }
            #[automatically_derived]

            impl #generics_for_impl ::epserde::ser::SerInner for #name #generics_for_type #ser_where_clause {
                type SerType = #name<#(#generics_for_ser_type,)*>;

                // Whether the type could be zero-copy
                const IS_ZERO_COPY: bool = #is_zero_copy_expr;

                unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                    use ::epserde::ser::WriteWithNames;

                    // Check whether the type could be zero-copy but it is not
                    // declared as such, and the attribute #[epserde(deep_copy)]
                    // is missing
                    const { assert!(!(! #is_deep_copy && #could_be_zero_copy), concat!("Enum ", #name_str, " could be zero-copy, but it has not been declared as such; use either #[epserde(zero_copy)] or #[epserde(deep_copy)] to silence this error")); }

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
                #[allow(clippy::useless_transmute)]
                #[inline(always)]
                fn __check_covariance<'__long: '__short, '__short>(
                    proof: ::epserde::deser::CovariantProof<Self::DeserType<'__long>>,
                ) -> ::epserde::deser::CovariantProof<Self::DeserType<'__short>> {
                    // SAFETY: enums are covariant in all their fields, and each
                    // field's DeserType is covariant (enforced by its own
                    // __check_covariance, which is called below).
                    #(
                        ::epserde::deser::__check_type_covariance::<#all_fields_types>();
                    )*
                    unsafe { ::core::mem::transmute(proof) }
                }

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

                type DeserType<'__ඞඞඞepserdeඞඞඞ_desertype> = #name<#(#generics_for_deser_type,)*>;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserInner;
                    use ::epserde::deser::Error;

                    #fixed_point_check
                    #full_copy_consistency_check

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
