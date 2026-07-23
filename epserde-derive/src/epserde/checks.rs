/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Generators for the compile-time checks injected into the derived impls.

use quote::quote;
use std::collections::HashSet;

use crate::utils::type_diag_span;

/// Generates the `IS_ZERO_COPY` associated-constant expression and the
/// operand of the could-be-zero-copy assertion emitted in deep-copy
/// `_ser_inner`, expanding the per-field conjunction only once.
///
/// The two expressions differ: `IS_ZERO_COPY` is `is_repr_c` AND-ed with the
/// conjunction of the fields `IS_ZERO_COPY`, while the assertion operand is
/// the fields conjunction alone, so that the could-be-zero-copy hint also
/// fires for types that are only missing `repr(C)`. When `is_repr_c` is true
/// the two coincide, so the assertion can reference the just-defined
/// constant; when it is false the constant is simply `false`.
pub(crate) fn gen_zero_copy_exprs(
    is_repr_c: bool,
    field_types: &[&syn::Type],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let fields_conj = quote!(true #(&& <#field_types>::IS_ZERO_COPY)*);
    if is_repr_c {
        (
            fields_conj,
            quote!(<Self as ::epserde::ser::SerInner>::IS_ZERO_COPY),
        )
    } else {
        (quote!(false), fields_conj)
    }
}

/// Generates the fixed-point assertion injected at the top of
/// `_deser_eps_inner` when one or more type parameters are ε-copy yet are also
/// [replaceable] parameters of a field marked `#[epserde(force_full_copy)]`.
///
/// For each conflicting parameter the assertion requires the bound `for<'a> <T
/// as DeserInner>::DeserType<'a>: EitherFullOrEpsCopy<T>`. The blanket impl
/// `impl<T> EitherFullOrEpsCopy<T> for T` makes the bound trivially hold when
/// `DeserType<'a> = T` (the fixed-point condition the user can supply through
/// `bound(deser = ...)`); otherwise the impl does not apply and the
/// `#[diagnostic::on_unimplemented]` message on `EitherFullOrEpsCopy` show a hint
/// alongside rustc's slot-mismatch error.
///
/// Each ident in `conflict_params` is expected to be re-spanned to the ε-copy
/// field that forces the constraint (see [`push_conflict_idents`]), so that the
/// diagnostic points at that field rather than at the derive invocation. The
/// ε-copy field is chosen over the `#[epserde(force_full_copy)]` one because the
/// hint recommends adding `force_full_copy`, so underlining a field that already
/// has it would be contradictory.
///
/// Returns an empty token stream when there are no conflicts.
///
/// [`push_conflict_idents`]: super::classify::push_conflict_idents
/// [replaceable]: crate::epserde::classify::collect_repl_param_occs
pub(crate) fn gen_fixed_point_check(conflict_params: &[syn::Ident]) -> proc_macro2::TokenStream {
    if conflict_params.is_empty() {
        return quote!();
    }
    // The failing bound is on __Slot (the deserialization type), not on the
    // bare parameter, so re-spanning the parameter alone leaves the error on the
    // derive invocation. Emitting each call with quote_spanned! at the field
    // span makes the whole call expression carry that span, so the diagnostic
    // points at the ε-copy field.
    let checks = conflict_params.iter().map(|param| {
        quote::quote_spanned! {param.span()=>
            __epserde_fixed_point_check::<
                #param,
                <#param as ::epserde::deser::DeserInner>::DeserType<'_>,
            >();
        }
    });
    quote! {
        fn __epserde_fixed_point_check<__Outer, __Slot: ?Sized>()
        where
            __Slot: ::epserde::deser::EitherFullOrEpsCopy<__Outer>,
        {}
        #(#checks)*
    }
}

/// Substitutes each ε-copy type parameter `P` of `type_params` in a cloned type
/// with its deserialization type `::epserde::deser::DeserType<'lifetime, P>`,
/// reproducing the slot the derive forms for a field in
/// `Self::DeserType<'lifetime>`.
///
/// The fold is total: it descends through every type constructor (via the
/// default [`syn::fold`] recursion), so the result matches exactly what the
/// compiler produces when instantiating the type's `DeserType`, including
/// occurrences inside [`PhantomData`] and qualified projections such as
/// `P::Assoc`. This exactness is what lets the consistency assertion built from
/// the result avoid false positives on a field that legitimately holds `P`
/// full-copy. Forced, phantom, and const parameters, and every other type, are
/// left verbatim.
///
/// [`PhantomData`]: https://doc.rust-lang.org/core/marker/struct.PhantomData.html
struct EpsParamSubst<'a> {
    eps_params: &'a HashSet<&'a syn::Ident>,
    lifetime: &'a syn::Lifetime,
}

impl syn::fold::Fold for EpsParamSubst<'_> {
    fn fold_type(&mut self, ty: syn::Type) -> syn::Type {
        if let syn::Type::Path(tp) = &ty {
            if tp.qself.is_none() && tp.path.leading_colon.is_none() {
                let first = &tp.path.segments[0];
                if first.arguments.is_empty() && self.eps_params.contains(&first.ident) {
                    let lt = self.lifetime;
                    let p = &first.ident;
                    let deser: syn::Type = syn::parse_quote!(::epserde::deser::DeserType<#lt, #p>);
                    if tp.path.segments.len() == 1 {
                        // Bare `P` becomes `DeserType<'lifetime, P>`.
                        return deser;
                    }
                    // Projection `P::Assoc[::…]` becomes `<DeserType<'lifetime,
                    // P>>::Assoc[::…]`, mirroring instantiation of the leading
                    // parameter.
                    let mut rest = tp.path.clone();
                    rest.segments = rest.segments.into_iter().skip(1).collect();
                    return syn::Type::Path(syn::TypePath {
                        qself: Some(syn::QSelf {
                            lt_token: Default::default(),
                            ty: Box::new(deser),
                            position: 0,
                            as_token: None,
                            gt_token: Default::default(),
                        }),
                        path: rest,
                    });
                }
            }
        }
        syn::fold::fold_type(self, ty)
    }
}

/// Generates the consistency assertion injected into `_deser_eps_inner` for each
/// ε-copy field that carries a `#[epserde(full_copy(...))]`-pinned parameter
/// (collected in `check_fields`).
///
/// For each such field the assertion requires `<Field as
/// DeserInner>::DeserType<'_>: FullCopyConsistent<Slot>`, where `Slot` is the
/// field's slot in `Self::DeserType` (the field type with the ε-copy parameters
/// in `eps_params` substituted and the pinned ones left verbatim, built by
/// [`EpsParamSubst`]). The blanket impl `impl<T> FullCopyConsistent<T> for T`
/// makes the bound hold exactly when the field's real deserialization type
/// coincides with the slot — so a field that genuinely holds the pinned
/// parameter full-copy is silent; otherwise, the
/// `#[diagnostic::on_unimplemented]` message on `FullCopyConsistent` surfaces
/// alongside rustc's slot mismatch.
///
/// Each call is emitted with `quote_spanned!` at the field span so the
/// diagnostic points at the offending field. `lifetime` is the lifetime of the
/// enclosing `_deser_eps_inner`, shared by both type arguments.
///
/// Returns an empty token stream when there are no such fields.
pub(crate) fn gen_full_copy_consistency_check(
    check_fields: &[&syn::Type],
    eps_params: &HashSet<&syn::Ident>,
    lifetime: &syn::Lifetime,
) -> proc_macro2::TokenStream {
    if check_fields.is_empty() {
        return quote!();
    }
    let checks = check_fields.iter().map(|field_ty| {
        let mut subst = EpsParamSubst {
            eps_params,
            lifetime,
        };
        let slot_ty = syn::fold::Fold::fold_type(&mut subst, (*field_ty).clone());
        quote::quote_spanned! {type_diag_span(field_ty)=>
            __epserde_full_copy_consistency::<
                <#field_ty as ::epserde::deser::DeserInner>::DeserType<#lifetime>,
                #slot_ty,
            >();
        }
    });
    quote! {
        fn __epserde_full_copy_consistency<__A: ?Sized, __B: ?Sized>()
        where
            __A: ::epserde::deser::FullCopyConsistent<__B>,
        {}
        #(#checks)*
    }
}

/// Generates the ε-copy stability assertion emitted, as a standalone item, for
/// each type parameter that occurs as the direct element of a literal `Vec<…>`,
/// boxed slice, or array in an ε-copy field.
///
/// The assertion requires `T: DeepCopyInSeq`; the blanket impl holds as soon as
/// the user bounds `T: DeepCopy`, so the check is silent for well-formed types
/// and surfaces the `#[diagnostic::on_unimplemented]` hint on `DeepCopyInSeq`
/// otherwise. It is emitted as a free generic function inside a `const _` block
/// (carrying the type's own generics and where clause) rather than inside the
/// (de)serialization bodies, so that its clean hint is reported before the raw
/// trait-resolution errors that the unbounded parameter triggers elsewhere.
///
/// Returns an empty token stream when there are no such parameters.
pub(crate) fn gen_seq_deep_check(
    seq_deep_idents: &[syn::Ident],
    generics_for_impl: &syn::ImplGenerics,
    where_clause: &syn::WhereClause,
) -> proc_macro2::TokenStream {
    if seq_deep_idents.is_empty() {
        return quote!();
    }
    quote! {
        const _: () = {
            fn __epserde_seq_deep_assert #generics_for_impl () #where_clause {
                fn __check<__SeqElem: ::epserde::deser::DeepCopyInSeq>() {}
                #(
                    __check::<#seq_deep_idents>();
                )*
            }
        };
    }
}
