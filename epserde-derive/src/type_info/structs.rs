/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! [`TypeInfo`] derive code for struct types.
//!
//! [`TypeInfo`]: derive@crate::TypeInfo

use quote::quote;

use super::{
    TypeInfoContext, gen_type_hash_body, gen_type_info_traits, gen_type_info_where_clauses,
};
use crate::utils::get_field_name;

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

            // Hash in size, as padding is given by PadTo.
            // and it is independent of the architecture.
            Hash::hash(&::core::mem::size_of::<Self>(), hasher);

            // Hash in representation data.
            #(
                Hash::hash(#repr_attrs, hasher);
            )*

            // Hash in all fields
            let old_offset_of = *offset_of;
            #(
                <#fields_types as AlignHash>::align_hash(
                    hasher,
                    offset_of,
                );
            )*

            // Advance offset_of to the end of Self. The field walk above stops
            // at the end of the last field, so it does not account for any
            // trailing padding; setting the offset here lets a following field
            // in a parent zero-copy type hash its padding at the correct
            // offset. Symmetric with the enum body.
            *offset_of = old_offset_of + ::core::mem::size_of::<Self>();
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

/// Generates the `PadTo` implementation body for struct types.
fn gen_struct_pad_to_body(fields_types: &[&syn::Type]) -> proc_macro2::TokenStream {
    quote! {
        use ::epserde::traits::PadTo;

        let mut pad_to = ::core::mem::align_of::<Self>();

        #(
            if pad_to < <#fields_types as PadTo>::pad_to() {
                pad_to = <#fields_types as PadTo>::pad_to();
            }
        )*
        pad_to
    }
}

/// [`TypeInfo`] derive code for struct types.
///
/// [`TypeInfo`]: derive@crate::TypeInfo
pub(crate) fn gen_struct_type_info_impl(
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

        // In zero-copy types the serialization type of every field is the
        // field type itself, so we hash the bare type: this matches the
        // where clauses, which bound the bare field types, and keeps
        // TypeInfo usable on generic zero-copy types whose parameters
        // implement just the type-information traits, not SerInner.
        if ctx.is_zero_copy {
            field_types_ts.push(quote! { #field_type });
        } else {
            field_types_ts.push(quote! { SerType<#field_type> });
        }
    }

    let (type_hash_where_clause, align_hash_where_clause, pad_to_where_clause) =
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
    let pad_to_body = if ctx.is_zero_copy {
        Some(gen_struct_pad_to_body(&field_types))
    } else {
        None
    };

    gen_type_info_traits(
        ctx,
        type_hash_where_clause,
        align_hash_where_clause,
        pad_to_where_clause,
        type_hash_body,
        align_hash_body,
        pad_to_body,
    )
}
