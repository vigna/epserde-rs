/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

//! Classification of fields and type parameters into ε-copy and full-copy.

use std::collections::{HashMap, HashSet};

use super::EpserdeContext;
use crate::utils::type_diag_span;

/// Records into `out` every *replaceable parameter* of `type_params` that
/// occurs in `ty`.
///
/// A type parameter is *replaceable* when it occurs in `ty` in its bare form
/// (as a single-segment path with no arguments, i.e. `T` itself) found by
/// descending through the supported type constructors (generic arguments,
/// tuples, arrays, slices). It is not made replaceable by an occurrence nested
/// inside `PhantomData` nor by one that is only a qualified projection such as
/// `T::Assoc` (opaque, not a bare `T`). The name reflects that such a parameter
/// is a candidate for replacement by `SerType`/`DeserType`.
pub(crate) fn collect_repl_param_occs<'a>(
    ty: &syn::Type,
    type_params: &HashSet<&'a syn::Ident>,
    out: &mut HashSet<&'a syn::Ident>,
    inside_phantom: bool,
) {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            if !inside_phantom
                && path.leading_colon.is_none()
                && path.segments.len() == 1
                && path.segments[0].arguments.is_empty()
            {
                let id = &path.segments[0].ident;
                if let Some(p) = type_params.get(id) {
                    out.insert(*p);
                    return;
                }
            }
            for segment in &path.segments {
                let segment_is_phantom = segment.ident == "PhantomData";
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let descend_inside_phantom = inside_phantom || segment_is_phantom;
                    for arg in &ab.args {
                        match arg {
                            syn::GenericArgument::Type(t) => {
                                collect_repl_param_occs(
                                    t,
                                    type_params,
                                    out,
                                    descend_inside_phantom,
                                );
                            }
                            syn::GenericArgument::AssocType(a) => {
                                collect_repl_param_occs(
                                    &a.ty,
                                    type_params,
                                    out,
                                    descend_inside_phantom,
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        syn::Type::Tuple(t) => {
            for e in &t.elems {
                collect_repl_param_occs(e, type_params, out, inside_phantom);
            }
        }
        syn::Type::Array(a) => collect_repl_param_occs(&a.elem, type_params, out, inside_phantom),
        syn::Type::Slice(s) => collect_repl_param_occs(&s.elem, type_params, out, inside_phantom),
        syn::Type::Paren(p) => collect_repl_param_occs(&p.elem, type_params, out, inside_phantom),
        syn::Type::Group(g) => collect_repl_param_occs(&g.elem, type_params, out, inside_phantom),
        _ => {}
    }
}

/// Returns true if `ty` contains a [replaceable] parameter from `type_params`.
/// Used to decide whether an unmarked field is ε-copy (a [replaceable] parameter
/// present) or full-copy (none: nothing to substitute).
///
/// [replaceable]: crate::epserde::classify::collect_repl_param_occs
pub(crate) fn has_repl_param(ty: &syn::Type, type_params: &HashSet<&syn::Ident>) -> bool {
    let mut out: HashSet<&syn::Ident> = HashSet::new();
    collect_repl_param_occs(ty, type_params, &mut out, false);
    !out.is_empty()
}

/// Accumulators for the classification of the fields of a type, filled by one
/// call to [`classify_field`](Self::classify_field) per field and consumed by
/// [`gen_epserde_parts`].
///
/// [`gen_epserde_parts`]: crate::epserde::helpers::gen_epserde_parts
#[derive(Default)]
pub(crate) struct FieldClassification<'a> {
    /// The ε-copy parameters: the non-force-full [replaceable] parameters of an
    /// ε-copy field. This is the `DeserType` substitution set, used directly.
    ///
    /// [replaceable]: collect_repl_param_occs
    pub(crate) eps_params: HashSet<&'a syn::Ident>,
    /// The full-copy parameters: the [replaceable] parameters of a force-full
    /// field, plus the `full_copy(...)`-listed ones. Used for the ε/full
    /// conflict diagnostic; its union with [`eps_params`](Self::eps_params) is
    /// the `SerType` substitution set (all [replaceable] parameters).
    ///
    /// [replaceable]: collect_repl_param_occs
    pub(crate) full_params: HashSet<&'a syn::Ident>,
    /// The [replaceable] parameters (force-full or not) of an ε-copy field. For
    /// ε-copy fields the field-type bound is suppressed (it would [shadow the
    /// `DeserType<'_>` projection]), so the caller emits an explicit `T:
    /// DeserInner` bound for each of these. Parameters of full-copy fields
    /// instead obtain `DeserInner` from their field-type bound, so they are
    /// not collected here.
    ///
    /// [shadow the `DeserType<'_>` projection]: https://github.com/rust-lang/rust/issues/152409
    /// [replaceable]: collect_repl_param_occs
    pub(crate) deser_inner_params: HashSet<&'a syn::Ident>,
    /// Maps each ε-copy parameter to the span of the first ε-copy field using
    /// it, so that a parameter that is also full-copy can have its conflict
    /// diagnostic point at that field.
    pub(crate) eps_field_spans: HashMap<&'a syn::Ident, proc_macro2::Span>,
    /// The ε-copy parameters occurring as a sequence element in an ε-copy
    /// field, each re-spanned to that field, so that the stability diagnostic
    /// points at the offending field.
    pub(crate) seq_deep_idents: Vec<syn::Ident>,
    /// The types of ε-copy fields that also contain a
    /// `#[epserde(full_copy(...))]`-pinned parameter. Such a field is sound
    /// only when its type holds the pinned parameter full-copy; the caller
    /// emits a [consistency assertion] for each, so a field that instead
    /// ε-copy deserializes the pinned parameter (e.g. `ControlFlow<F, E>`)
    /// gets a readable diagnostic rather than a raw slot mismatch.
    ///
    /// [consistency assertion]: crate::epserde::checks::gen_full_copy_consistency_check
    pub(crate) full_copy_check_fields: Vec<&'a syn::Type>,
}

impl<'a> FieldClassification<'a> {
    /// Examines one field, recording its [replaceable] parameters into the
    /// accumulators, and returns whether the field is full-copy.
    ///
    /// A field is full-copy when it carries `#[epserde(force_full_copy)]`, or
    /// when all its [replaceable] parameters are listed in
    /// `#[epserde(full_copy(...))]` (in particular, when it has none).
    ///
    /// When `force_full_copy_field` is true but the field has no [replaceable]
    /// parameter, the marker has no effect and a warning naming the field as
    /// `field_label` is printed on standard error.
    ///
    /// The walk matches against the type parameters in [`repl_params`], which
    /// excludes const parameters (never [replaceable]: a bare occurrence of a
    /// const parameter in a field type, e.g. as a forwarded generic argument,
    /// is indistinguishable from a type at the syntactic level, but must be
    /// left untouched by the substitution) as well as the parameters declared
    /// with the type-level `#[epserde(phantom(...))]` attribute, which are
    /// left completely untouched (no substitution, no bounds). The parameters
    /// in [`forced_params`] are pinned to full-copy deserialization by the
    /// type-level `#[epserde(full_copy(...))]` attribute.
    ///
    /// [`repl_params`]: EpserdeContext::repl_params
    /// [`forced_params`]: EpserdeContext::forced_params
    /// [replaceable]: collect_repl_param_occs
    pub(crate) fn classify_field(
        &mut self,
        ctx: &EpserdeContext<'a>,
        field_label: &dyn std::fmt::Display,
        field_type: &'a syn::Type,
        force_full_copy_field: bool,
    ) -> bool {
        if force_full_copy_field && !has_repl_param(field_type, &ctx.repl_params) {
            let type_name = &ctx.derive_input.ident;
            eprintln!(
                "warning: #[epserde(force_full_copy)] on field {field_label} of type {type_name} has no effect; consider removing the marker"
            );
        }

        let mut field_occ = HashSet::new();
        collect_repl_param_occs(field_type, &ctx.repl_params, &mut field_occ, false);

        if force_full_copy_field {
            // Every parameter of a force-full field is a full-copy parameter.
            self.full_params.extend(&field_occ);
            return true;
        }

        // Unmarked field: a force-full parameter is full-copy, otherwise it is
        // an ε-copy parameter. The field is full-copy iff it has no ε-copy
        // parameter.
        let mut has_eps = false;
        let mut has_forced = false;
        for p in &field_occ {
            if ctx.forced_params.contains(p) {
                self.full_params.insert(*p);
                has_forced = true;
            } else {
                self.eps_params.insert(*p);
                self.eps_field_spans
                    .entry(*p)
                    .or_insert_with(|| type_diag_span(field_type));
                has_eps = true;
            }
        }

        if has_eps {
            // The field-type DeserInner bound is suppressed for ε-copy fields,
            // so each of this field parameters (force-full ones included)
            // needs an explicit DeserInner bound emitted by the caller.
            self.deser_inner_params.extend(&field_occ);
            let mut field_seq_deep = HashSet::new();
            collect_seq_forced_deep_params(
                field_type,
                &ctx.repl_params,
                &mut field_seq_deep,
                false,
            );
            push_seq_deep_idents(
                &field_seq_deep,
                type_diag_span(field_type),
                &mut self.seq_deep_idents,
            );

            // An ε-copy field that also pins a parameter to full-copy keeps
            // that parameter verbatim in its DeserType slot while the field's
            // own `_deser_eps_inner` may substitute it: the caller asserts the
            // two agree.
            if has_forced {
                self.full_copy_check_fields.push(field_type);
            }
        }

        !has_eps
    }
}

/// Records into `out` every type parameter that occurs as the direct element of
/// a literal `Vec<…>`, boxed/bare slice `[…]`, or array `[…; N]` anywhere within
/// `ty`. Such a parameter is forced to be deep-copy for ε-copy stability: were
/// it zero-copy, the containing sequence would ε-copy deserialize to a slice
/// reference, a type not expressible as the original sequence.
///
/// An occurrence nested inside `PhantomData<…>` is ignored: a phantom slot is
/// zero-sized and never serialized, so it imposes no ε-copy-stability
/// requirement (mirroring [`collect_repl_param_occs`], which excludes phantom
/// occurrences from the [replaceable] set).
///
/// [replaceable]: crate::epserde::classify::collect_repl_param_occs
fn collect_seq_forced_deep_params<'a>(
    ty: &syn::Type,
    type_params: &HashSet<&'a syn::Ident>,
    out: &mut HashSet<&'a syn::Ident>,
    inside_phantom: bool,
) {
    fn record_if_bare<'a>(
        ty: &syn::Type,
        type_params: &HashSet<&'a syn::Ident>,
        out: &mut HashSet<&'a syn::Ident>,
    ) {
        if let syn::Type::Path(syn::TypePath { qself: None, path }) = ty {
            if path.leading_colon.is_none()
                && path.segments.len() == 1
                && path.segments[0].arguments.is_empty()
            {
                let id = &path.segments[0].ident;
                if let Some(p) = type_params.get(id) {
                    out.insert(*p);
                }
            }
        }
    }

    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            for segment in &path.segments {
                let segment_is_phantom = segment.ident == "PhantomData";
                if let syn::PathArguments::AngleBracketed(ab) = &segment.arguments {
                    let is_vec = segment.ident == "Vec";
                    let descend_inside_phantom = inside_phantom || segment_is_phantom;
                    for arg in &ab.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            if is_vec && !descend_inside_phantom {
                                record_if_bare(t, type_params, out);
                            }
                            collect_seq_forced_deep_params(
                                t,
                                type_params,
                                out,
                                descend_inside_phantom,
                            );
                        }
                    }
                }
            }
        }
        syn::Type::Slice(s) => {
            if !inside_phantom {
                record_if_bare(&s.elem, type_params, out);
            }
            collect_seq_forced_deep_params(&s.elem, type_params, out, inside_phantom);
        }
        syn::Type::Array(a) => {
            if !inside_phantom {
                record_if_bare(&a.elem, type_params, out);
            }
            collect_seq_forced_deep_params(&a.elem, type_params, out, inside_phantom);
        }
        syn::Type::Tuple(t) => {
            for e in &t.elems {
                collect_seq_forced_deep_params(e, type_params, out, inside_phantom);
            }
        }
        syn::Type::Reference(r) => {
            collect_seq_forced_deep_params(&r.elem, type_params, out, inside_phantom)
        }
        syn::Type::Paren(p) => {
            collect_seq_forced_deep_params(&p.elem, type_params, out, inside_phantom)
        }
        syn::Type::Group(g) => {
            collect_seq_forced_deep_params(&g.elem, type_params, out, inside_phantom)
        }
        _ => {}
    }
}

/// Pushes each parameter in `params` into `out`, re-spanned to `span`, so that
/// the stability assertion generated from `out` points at the field that forces
/// the parameter to be deep-copy.
fn push_seq_deep_idents(
    params: &HashSet<&syn::Ident>,
    span: proc_macro2::Span,
    out: &mut Vec<syn::Ident>,
) {
    for p in params {
        let mut id = (*p).clone();
        id.set_span(span);
        out.push(id);
    }
}

/// Computes the conflict parameters, that is, the intersection of `eps_params`
/// (the ε-copy parameters) with `full_params` (the full-copy parameters), and pushes
/// each into `out`, re-spanned to the ε-copy
/// field that uses it (recorded in `eps_field_spans`), so the fixed-point
/// diagnostic points at that field.
///
/// The `full_copy(...)`-listed members of `full_params` never cause an output:
/// being force-full, they are absent from `eps_params`, so the intersection
/// keeps only ε-copy parameters that also occur in a full-copy field.
pub(crate) fn push_conflict_idents(
    eps_params: &HashSet<&syn::Ident>,
    full_params: &HashSet<&syn::Ident>,
    eps_field_spans: &HashMap<&syn::Ident, proc_macro2::Span>,
    out: &mut Vec<syn::Ident>,
) {
    for p in eps_params.intersection(full_params) {
        let mut id = (*p).clone();
        if let Some(span) = eps_field_spans.get(*p) {
            id.set_span(*span);
        }
        out.push(id);
    }
}
