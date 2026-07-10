/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! [`TypeInfo`] derive code for enum types.
//!
//! [`TypeInfo`]: derive@crate::TypeInfo

use quote::quote;

use super::{
    TypeInfoContext, gen_type_hash_body, gen_type_info_traits, gen_type_info_where_clauses,
};

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

            // Hash in size, as padding is given by PadTo,
            // and it is independent of the architecture
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
        // Each field hashes with a fresh offset (see the field loops), so no
        // reset of offset_of is needed.
        quote! {
            use ::epserde::traits::AlignHash;
            use ::epserde::ser::SerType;

            #(
                #all_align_hashes
            )*
        }
    }
}

/// Replaces every Self identifier token in the given token stream with the
/// given name, recursing into groups.
///
/// Used when copying enum discriminant expressions into the fieldless mirror
/// enum of the zero-copy type-hash path: inside the mirror, Self would denote
/// the mirror itself, which has none of the enum's impls, whereas the
/// expression was written with Self denoting the derived enum. The
/// substitution is exact because a discriminant expression mentioning Self
/// compiles only for non-generic enums, where the bare name denotes the same
/// type.
fn replace_self(tokens: proc_macro2::TokenStream, name: &syn::Ident) -> proc_macro2::TokenStream {
    tokens
        .into_iter()
        .map(|token| match token {
            proc_macro2::TokenTree::Ident(ident) if ident == "Self" => {
                let mut name = name.clone();
                name.set_span(ident.span());
                proc_macro2::TokenTree::Ident(name)
            }
            proc_macro2::TokenTree::Group(group) => {
                let mut replaced =
                    proc_macro2::Group::new(group.delimiter(), replace_self(group.stream(), name));
                replaced.set_span(group.span());
                proc_macro2::TokenTree::Group(replaced)
            }
            token => token,
        })
        .collect()
}

/// Extends the per-variant `TypeHash`, `AlignHash`, and `PadTo` fragments with
/// the code for one field.
///
/// `field_name_expr` is the expression whose value is hashed as the field
/// name: `stringify!(name)` for a named field, the index as a string literal
/// for an unnamed one.
fn push_variant_field(
    ctx: &TypeInfoContext,
    field_name_expr: proc_macro2::TokenStream,
    field_type: &syn::Type,
    type_hash: &mut proc_macro2::TokenStream,
    align_hash: &mut proc_macro2::TokenStream,
    pad_to: &mut proc_macro2::TokenStream,
) {
    // See the comment in gen_struct_type_info_impl for why
    // zero-copy types hash the bare field type.
    let field_type_ts = if ctx.is_zero_copy {
        quote! { #field_type }
    } else {
        quote! { SerType<#field_type> }
    };

    type_hash.extend([quote! {
        Hash::hash(#field_name_expr, hasher);
        <#field_type_ts as TypeHash>::type_hash(hasher);
    }]);

    // In zero-copy enums fields are contiguous in memory, so
    // the offset accumulates; in deep-copy enums each field is
    // realigned in the stream, so each starts at offset zero,
    // mirroring the deep-copy struct body.
    if ctx.is_zero_copy {
        align_hash.extend([quote! {
            <#field_type_ts as AlignHash>::align_hash(hasher, offset_of);
        }]);
    } else {
        align_hash.extend([quote! {
            <#field_type_ts as AlignHash>::align_hash(hasher, &mut 0);
        }]);
    }

    pad_to.extend([quote! {
        if pad_to < <#field_type as PadTo>::pad_to() {
            pad_to = <#field_type as PadTo>::pad_to();
        }
    }]);
}

/// [`TypeInfo`] derive code for enum types.
///
/// [`TypeInfo`]: derive@crate::TypeInfo
pub(crate) fn gen_enum_type_info_impl(
    ctx: TypeInfoContext,
    e: &syn::DataEnum,
) -> proc_macro2::TokenStream {
    let mut all_type_hashes = vec![];
    let mut all_align_hashes = vec![];
    let mut all_pad_tos = vec![];
    let mut all_field_types = vec![];

    // A zero-copy enum is (de)serialized as raw memory, so its discriminant
    // values are part of the encoding and re-numbering variants must change the
    // type hash. We therefore hash the resolved discriminant of every variant,
    // symmetric across implicit and explicit discriminants (so that
    // layout-identical enums such as { A, B } and { A = 0, B = 1 } hash equal).
    //
    // A fieldless enum could be cast to an integer directly, but a data-carrying
    // enum cannot (one variant with a field makes the whole enum non-castable),
    // so we cannot write Self::Variant as i128 in general. Instead we emit a
    // fieldless mirror enum carrying the same repr and the same explicit
    // discriminant expressions: the compiler assigns it the identical
    // discriminants (fields never affect discriminant values, only layout), and
    // that fieldless mirror is castable. This lets the compiler compute every
    // discriminant, defaulted ones included, rather than reconstructing the
    // +1-per-variant rule by hand, and it evaluates each expression at the
    // declared discriminant type so overflow and integer inference behave as in
    // the original. A named constant in an expression contributes its value,
    // not its name, so changing that value is detected as a type-hash mismatch.
    //
    // Deep-copy enums write a positional tag rather than the discriminant, so
    // their discriminant values are irrelevant and are not hashed.
    let mirror_ident = syn::Ident::new(
        "__ඞඞඞepserdeඞඞඞ_DiscrMirror",
        proc_macro2::Span::call_site(),
    );
    // The mirror needs the repr that fixes the discriminant values. A primitive
    // integer repr sets both the discriminant type and its wrapping, so when one
    // is present it alone is mirrored: pairing it with C (as the original
    // data-carrying enum may, e.g. repr(C, u8)) is a conflicting-repr error on a
    // fieldless enum, where C would independently pick the discriminant type.
    // Otherwise C is mirrored, giving C-int-sized discriminants. align/packed
    // govern layout, not discriminant values, and are dropped.
    let mirror_repr = ctx
        .repr_attrs
        .iter()
        .find(|hint| {
            matches!(
                hint.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
            )
        })
        .or_else(|| ctx.repr_attrs.iter().find(|hint| hint.as_str() == "C"))
        .map(|hint| syn::Ident::new(hint, proc_macro2::Span::call_site()));
    // One unit variant per original variant, keeping its explicit discriminant
    // expression where present. Rust forbids enum discriminants from mentioning
    // the enum's generic parameters, so the expressions are safe to copy into
    // this non-generic mirror. Self, however, is permitted, and inside the
    // mirror it would denote the mirror itself, so it is replaced with the
    // enum's name; the substitution is exact because a discriminant mentioning
    // Self compiles only for non-generic enums. A Self variant reference thus
    // resolves to the same value, and a layout query such as size_of::<Self>()
    // cannot occur at all: it is rejected upstream as a query cycle.
    let mirror_variants = e.variants.iter().map(|variant| {
        let ident = &variant.ident;
        match &variant.discriminant {
            Some((_, expr)) => {
                let expr = replace_self(quote! { #expr }, ctx.name);
                quote! { #ident = #expr }
            }
            None => quote! { #ident },
        }
    });
    // The mirror is a local item in the type_hash body; it is emitted only for
    // zero-copy enums, where a repr is guaranteed to be present.
    let mirror_decl = ctx.is_zero_copy.then(|| {
        quote! {
            #[repr(#mirror_repr)]
            #[allow(dead_code)]
            enum #mirror_ident {
                #(#mirror_variants,)*
            }
        }
    });

    // Process each variant
    for variant in &e.variants {
        let ident = &variant.ident;
        let mut type_hash = quote! { Hash::hash(stringify!(#ident), hasher); };
        if ctx.is_zero_copy {
            type_hash.extend([quote! {
                Hash::hash(&(#mirror_ident::#ident as i128), hasher);
            }]);
        }
        let mut field_types = vec![];
        let mut align_hash = quote! {};
        let mut pad_to = quote! {};

        match &variant.fields {
            syn::Fields::Unit => {}

            syn::Fields::Named(fields) => {
                for field in &fields.named {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = &field.ty;
                    field_types.push(field_type);

                    push_variant_field(
                        &ctx,
                        quote! { stringify!(#field_name) },
                        field_type,
                        &mut type_hash,
                        &mut align_hash,
                        &mut pad_to,
                    );
                }
            }

            syn::Fields::Unnamed(fields) => {
                for (field_idx, field) in fields.unnamed.iter().enumerate() {
                    let field_name = field_idx.to_string();
                    let field_type = &field.ty;
                    field_types.push(field_type);

                    push_variant_field(
                        &ctx,
                        quote! { #field_name },
                        field_type,
                        &mut type_hash,
                        &mut align_hash,
                        &mut pad_to,
                    );
                }
            }
        }

        all_type_hashes.push(type_hash);
        all_align_hashes.push(align_hash);
        all_pad_tos.push(pad_to);
        all_field_types.extend(field_types);
    }

    // Prepend the fieldless mirror enum so the per-variant casts above can name
    // it (local items are visible throughout the enclosing block).
    if let Some(mirror_decl) = mirror_decl {
        all_type_hashes.insert(0, mirror_decl);
    }

    let (where_clause_type_hash, where_clause_align_hash, where_clause_pad_to) =
        gen_type_info_where_clauses(ctx.where_clause, ctx.is_zero_copy, &all_field_types);

    let type_hash_body = gen_type_hash_body(&ctx, &all_type_hashes);
    let align_hash_body = gen_enum_align_hash_body(&ctx, &all_align_hashes);
    // For a fieldless enum there is nothing to maximize over, so we avoid
    // emitting an unused import and a never-mutated binding.
    let pad_to_body = if all_pad_tos.iter().all(|t| t.is_empty()) {
        quote! {
            ::core::mem::align_of::<Self>()
        }
    } else {
        quote! {
            use ::epserde::traits::PadTo;

            let mut pad_to = ::core::mem::align_of::<Self>();
            #(
                #all_pad_tos
            )*
            pad_to
        }
    };

    let pad_to_body = if ctx.is_zero_copy {
        Some(pad_to_body)
    } else {
        None
    };

    gen_type_info_traits(
        ctx,
        where_clause_type_hash,
        where_clause_align_hash,
        where_clause_pad_to,
        type_hash_body,
        align_hash_body,
        pad_to_body,
    )
}
