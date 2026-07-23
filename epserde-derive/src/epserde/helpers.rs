/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Code generation shared by the struct and enum paths of the derive.

use quote::quote;
use std::collections::HashSet;
use syn::WhereClause;

use super::EpserdeContext;
use super::bounds::{
    bound_ser_deser_types, gen_generics_for_deser_type, gen_generics_for_ser_type,
    gen_ser_deser_where_clauses,
};
use super::checks::{
    gen_fixed_point_check, gen_full_copy_consistency_check, gen_seq_deep_check, gen_zero_copy_exprs,
};
use super::classify::{FieldClassification, push_conflict_idents};

/// The parts of the derived `SerInner` and `DeserInner` implementations that
/// are computed identically for structs and enums from the classification of
/// their fields.
///
/// Instances are built by [`gen_epserde_parts`].
pub(crate) struct EpserdeParts {
    /// The generic arguments of the serialization type (see
    /// [`gen_generics_for_ser_type`]).
    pub(crate) generics_for_ser_type: Vec<proc_macro2::TokenStream>,
    /// The generic arguments of the deserialization type (see
    /// [`gen_generics_for_deser_type`]).
    pub(crate) generics_for_deser_type: Vec<proc_macro2::TokenStream>,
    /// The fixed-point assertion (see [`gen_fixed_point_check`]).
    pub(crate) fixed_point_check: proc_macro2::TokenStream,
    /// The full-copy consistency assertion (see
    /// [`gen_full_copy_consistency_check`]).
    pub(crate) full_copy_consistency_check: proc_macro2::TokenStream,
    /// The ε-copy stability assertion (see [`gen_seq_deep_check`]).
    pub(crate) seq_deep_check: proc_macro2::TokenStream,
    /// The `IS_ZERO_COPY` associated-constant expression (see
    /// [`gen_zero_copy_exprs`]).
    pub(crate) is_zero_copy_expr: proc_macro2::TokenStream,
    /// The operand of the could-be-zero-copy assertion (see
    /// [`gen_zero_copy_exprs`]).
    pub(crate) could_be_zero_copy: proc_macro2::TokenStream,
    /// The where clause of the `SerInner` implementation.
    pub(crate) ser_where_clause: WhereClause,
    /// The where clause of the `DeserInner` implementation.
    pub(crate) deser_where_clause: WhereClause,
}

/// Computes the [`EpserdeParts`] of the derived implementations from the
/// classification of the fields.
///
/// `field_types` contains the types of all fields (for enums, of all
/// variants), and `full_deser_fields[i]` is `true` when `field_types[i]` is
/// full-copy.
pub(crate) fn gen_epserde_parts(
    ctx: &EpserdeContext,
    cls: &FieldClassification,
    field_types: &[&syn::Type],
    full_deser_fields: &[bool],
) -> EpserdeParts {
    // SerType substitutes every replaceable parameter
    // uniformly): the union of the ε-copy and full-copy parameters.
    let params: HashSet<&syn::Ident> = cls.eps_params.union(&cls.full_params).copied().collect();
    let generics_for_deser_type = gen_generics_for_deser_type(ctx, &cls.eps_params);
    let generics_for_ser_type = gen_generics_for_ser_type(ctx, &params);
    // A type parameter that is both ε-copy and full-copy produces
    // a slot mismatch in the generated _deser_eps_inner: one occurrence
    // becomes <T as DeserInner>::DeserType<'_>, the other stays as T. The
    // user can resolve the conflict with a bound that forces DeserType<'_>
    // = T (automatic for ZeroCopy types). The assertion below requests
    // EitherFullOrEpsCopy for each conflicting parameter so that, when the
    // bound is missing, the on_unimplemented message points at the fix
    // instead of leaving the user with rustc's raw slot mismatch. Each
    // conflicting parameter is re-spanned to the ε-copy field that uses it,
    // so the diagnostic points at that field.
    let mut conflict_params: Vec<syn::Ident> = vec![];
    push_conflict_idents(
        &cls.eps_params,
        &cls.full_params,
        &cls.eps_field_spans,
        &mut conflict_params,
    );
    let fixed_point_check = gen_fixed_point_check(&conflict_params);
    let deser_eps_lifetime: syn::Lifetime = syn::parse_quote!('deser_eps_inner_lifetime);
    let full_copy_consistency_check = gen_full_copy_consistency_check(
        &cls.full_copy_check_fields,
        &cls.eps_params,
        &deser_eps_lifetime,
    );
    let seq_deep_check = gen_seq_deep_check(
        &cls.seq_deep_idents,
        &ctx.generics_for_impl,
        ctx.where_clause,
    );
    let (is_zero_copy_expr, could_be_zero_copy) = gen_zero_copy_exprs(ctx.is_repr_c, field_types);
    let (mut ser_where_clause, mut deser_where_clause) =
        gen_ser_deser_where_clauses(field_types, ctx.is_zero_copy, full_deser_fields);

    // Add user-specified bounds from #[epserde(bound(...))]
    ser_where_clause
        .predicates
        .extend(ctx.ser_bounds.iter().cloned());
    deser_where_clause
        .predicates
        .extend(ctx.deser_bounds.iter().cloned());

    // Emit T: SerInner for every SerType-substituted parameter, and
    // T: DeserInner for every parameter occurring in an ε-copy field.
    // The field-type bound was skipped above for ε-copy fields;
    // these per-parameter bounds let Rust resolve kind-uniform wrapper
    // impls (Box, Rc, Arc, Option, Range, tuples). For wrappers whose
    // resolution depends on T's kind (Vec, Box<[…]>, [T; N]),
    // the user must additionally bound T: ZeroCopy or T: DeepCopy.
    if !ctx.is_zero_copy {
        for ident in &params {
            ser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::ser::SerInner
            ));
        }
        for ident in &cls.deser_inner_params {
            deser_where_clause.predicates.push(syn::parse_quote!(
                #ident: ::epserde::deser::DeserInner
            ));
        }

        // In zero-copy types we do not need to add bounds to
        // the associated SerType/DeserType, as generics are not
        // replaced with their SerType/DeserType.
        bound_ser_deser_types(
            ctx.derive_input,
            &params,
            &cls.eps_params,
            &mut ser_where_clause,
            &mut deser_where_clause,
        );
    }

    EpserdeParts {
        generics_for_ser_type,
        generics_for_deser_type,
        fixed_point_check,
        full_copy_consistency_check,
        seq_deep_check,
        is_zero_copy_expr,
        could_be_zero_copy,
        ser_where_clause,
        deser_where_clause,
    }
}

/// Generates the `CopyType`, `SerInner`, and `DeserInner` implementations of a
/// zero-copy type.
///
/// The generated code is identical for structs and enums: a zero-copy type is
/// (de)serialized as raw bytes by the zero-copy helpers, so the implementations
/// contain no per-field or per-variant code.
pub(crate) fn gen_zero_copy_impl(
    ctx: &EpserdeContext,
    is_zero_copy_expr: &proc_macro2::TokenStream,
    ser_where_clause: &WhereClause,
    deser_where_clause: &WhereClause,
) -> proc_macro2::TokenStream {
    let name = &ctx.derive_input.ident;
    let generics_for_impl = &ctx.generics_for_impl;
    let generics_for_type = &ctx.generics_for_type;
    let where_clause = &ctx.where_clause;

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

            unsafe fn _ser_inner(&self, backend: &mut impl ::epserde::ser::WriteWithNames) -> ::epserde::ser::Result<()> {
                unsafe { ::epserde::ser::helpers::ser_zero(backend, self) }
            }
        }

        #[automatically_derived]
        impl #generics_for_impl ::epserde::deser::DeserInner for #name #generics_for_type #deser_where_clause
        {
            #[inline(always)]
            fn __check_covariance<'__long: '__short, '__short>(
                proof: ::epserde::deser::CovariantProof<Self::DeserType<'__long>>,
            ) -> ::epserde::deser::CovariantProof<Self::DeserType<'__short>> {
                proof
            }

            unsafe fn _deser_full_inner(
                backend: &mut impl ::epserde::deser::ReadWithPos,
            ) -> ::core::result::Result<Self, ::epserde::deser::Error> {
                unsafe { ::epserde::deser::helpers::deser_full_zero::<Self>(backend) }
            }

            type DeserType<'__ඞඞඞepserdeඞඞඞ_desertype> = &'__ඞඞඞepserdeඞඞඞ_desertype Self;

            unsafe fn _deser_eps_inner<'deser_eps_inner_lifetime>(
                backend: &mut ::epserde::deser::SliceWithPos<'deser_eps_inner_lifetime>,
            ) -> ::core::result::Result<Self::DeserType<'deser_eps_inner_lifetime>, ::epserde::deser::Error>
            {
                unsafe { ::epserde::deser::helpers::deser_eps_zero::<Self>(backend) }
            }
        }
    }
}

/// Generates the per-field initializer used inside the derived
/// `_deser_eps_inner` body's struct literal.
///
/// For each field this is either a `_deser_full_inner` call, a
/// `_deser_eps_inner` call, a literal `PhantomData`, or a
/// `_deser_eps_inner_special` call for `PhantomDeserData`. The choice
/// between `_deser_full_inner` and `_deser_eps_inner` is taken by the
/// caller via `deser_full`; the `PhantomData` and `PhantomDeserData`
/// branches override that choice based on the field type.
///
/// The type of `field_name` is [`proc_macro2::TokenStream`] because it can be
/// either an identifier (for named fields) or an index (for unnamed fields).
pub(crate) fn gen_eps_deser_method_call(
    field_name: &proc_macro2::TokenStream,
    field_type: &syn::Type,
    deser_full: bool,
) -> proc_macro2::TokenStream {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path { segments, .. },
    }) = field_type
    {
        // This is a pretty weak check, as a user could define its own
        // PhantomDeserData, but it should be good enough in practice.
        // The two checks below are mutually exclusive (a path has
        // exactly one last segment). Note that we must accept a leading
        // colon, as in ::core::marker::PhantomData, mirroring the
        // replaceable-parameter walk.
        if let Some(segment) = segments.last() {
            if segment.ident == "PhantomDeserData" {
                return syn::parse_quote!(#field_name: unsafe { <#field_type>::_deser_eps_inner_special(backend)? });
            }
            // PhantomData<...> is handled natively: emit a literal
            // PhantomData whose generic parameter is inferred from the
            // surrounding Self::DeserType<'a> struct literal.
            if segment.ident == "PhantomData" {
                return syn::parse_quote!(#field_name: ::core::marker::PhantomData);
            }
        }
    }

    if deser_full {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_full_inner(backend)? })
    } else {
        syn::parse_quote!(#field_name: unsafe { <#field_type as DeserInner>::_deser_eps_inner(backend)? })
    }
}
