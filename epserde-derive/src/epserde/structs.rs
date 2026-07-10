/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! [`Epserde`] derive code for struct types.
//!
//! [`Epserde`]: derive@crate::Epserde

use quote::quote;

use super::EpserdeContext;
use super::classify::FieldClassification;
use super::helpers::{
    EpserdeParts, gen_eps_deser_method_call, gen_epserde_parts, gen_zero_copy_impl,
};
use crate::attrs::is_force_full_copy;
use crate::utils::get_field_name;

/// [`Epserde`] derive code for struct types.
///
/// [`Epserde`]: derive@crate::Epserde
pub(crate) fn gen_epserde_struct_impl(
    ctx: &EpserdeContext,
    s: &syn::DataStruct,
) -> proc_macro2::TokenStream {
    let mut field_names = vec![];
    let mut field_types = vec![];
    let mut method_calls = vec![];
    let mut full_deser_fields = vec![];
    let mut cls = FieldClassification::default();

    for (field_idx, field) in s.fields.iter().enumerate() {
        let field_name = get_field_name(field, field_idx);
        let field_type = &field.ty;
        let force_full_copy = is_force_full_copy(field);

        let deser_full = cls.classify_field(ctx, &field_name, field_type, force_full_copy);

        method_calls.push(gen_eps_deser_method_call(
            &field_name,
            field_type,
            deser_full,
        ));

        field_names.push(field_name);
        field_types.push(field_type);
        full_deser_fields.push(deser_full);
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
    } = gen_epserde_parts(ctx, &cls, &field_types, &full_deser_fields);

    let name = &ctx.derive_input.ident;
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
        let is_deep_copy = ctx.is_deep_copy;
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
                    const { assert!(!(! #is_deep_copy && #could_be_zero_copy), concat!("Structure ", #name_str, " could be zero-copy, but it has not been declared as such; use either #[epserde(zero_copy)] or #[epserde(deep_copy)] to silence this error")); }

                    #(
                        unsafe { WriteWithNames::write(backend, stringify!(#field_names), &self.#field_names)?; }
                    )*
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
                    // SAFETY: structs are covariant in all their fields, and each
                    // field's DeserType is covariant (enforced by its own
                    // __check_covariance, which is called below).
                    #(
                        ::epserde::deser::__check_type_covariance::<#field_types>();
                    )*
                    unsafe { ::core::mem::transmute(proof) }
                }

                unsafe fn _deser_full_inner(
                    backend: &mut impl ::epserde::deser::ReadWithPos,
                ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                    use ::epserde::deser::DeserInner;

                    Ok(#name{
                        #( #field_names: unsafe { <#field_types as DeserInner>::_deser_full_inner(backend)? }, )*
                    })
                }

                type DeserType<'__ඞඞඞepserdeඞඞඞ_desertype> = #name<#(#generics_for_deser_type,)*>;

                unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                    backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
                ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
                {
                    use ::epserde::deser::DeserInner;

                    #fixed_point_check
                    #full_copy_consistency_check

                    Ok(#name{
                        #( #method_calls, )*
                    })
                }
            }
        }
    }
}
