# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**ε-serde** is an ε-copy (almost zero-copy) serialization/deserialization framework for Rust. It minimizes allocation during deserialization by allowing references directly into serialized data, providing near-instant deserialization. The workspace contains two crates:

- **`epserde`**: Main library (edition 2024, MSRV 1.85)
- **`epserde-derive`**: Procedural macros (`#[derive(Epserde)]` and `#[derive(TypeInfo)]`)

## Build & Test Commands

```bash
# Build
cargo build --all-features

# Lint (CI runs both)
cargo fmt -- --check
cargo clippy

# Run tests (excludes compile-fail trybuild tests)
cargo test -- --skip fail

# Run compile-fail tests only
cargo test fail

# Regenerate compile-fail .stderr (use the current stable toolchain — see note below)
TRYBUILD=overwrite cargo test --test fail

# Run a single test
cargo test test_name -- --skip fail

# Run all examples
cd epserde && for f in examples/*.rs; do cargo run --example "$(basename "${f%.rs}")"; done
```

CI tests against Rust 1.85.0, stable, beta, and nightly, with three feature combinations: default, `--no-default-features --features 'derive'`, and `--all-features`.

**Compile-fail (`trybuild`) tests run on only one CI job — the `build` matrix entry with `rust: stable` and default features** (every other entry, plus `build-i686`, passes `--skip fail`). The committed `tests/fail/*.stderr` snapshots are tied to the compiler's diagnostic rendering, which changes between toolchains (e.g. 1.85.0 abbreviates long type arguments with `...` in some labels where newer rustc prints them in full). They must therefore match the **current stable** compiler — the one a developer runs `cargo test` with — so regenerate them with the default toolchain, never an old pinned one. The MSRV/`i686` job deliberately skips these tests for this reason. The rendering of these particular diagnostics is architecture-independent.

## Architecture

### Core Type System

The framework classifies types into two categories via the `CopyType` trait:

- **`ZeroCopy`** (`type Copy = Zero`): Fixed-layout types with no references. Must be `#[repr(C)]`. Serialized/deserialized as raw bytes. Marked with `#[epserde(zero_copy)]`.
- **`DeepCopy`** (`type Copy = Deep`): Types requiring recursive field-by-field processing. Marked with `#[epserde(deep_copy)]`.

This distinction drives specialization throughout the crate via `SerHelper<Zero>`/`SerHelper<Deep>` and `DeserHelper<Zero>`/`DeserHelper<Deep>` (sealed trait pattern).

### Trait Hierarchy

**Serialization** (`ser/`):
- `Serialize` — user-facing (provides `serialize()`, `store()`)
- `SerInner` — derived/implemented (has `SerType` associated type for type normalization/erasure, `_ser_inner()`)

**Deserialization** (`deser/`):
- `Deserialize` — user-facing (provides `deserialize_full()`, `deserialize_eps()`, `load_full()`, `mmap()`)
- `DeserInner` — derived/implemented (has `DeserType<'a>` associated type for lifetime-dependent deserialized form, `_deser_full_inner()`, `_deser_eps_inner()`)

**Type identity** (`traits/`):
- `TypeHash`, `AlignHash`, `AlignTo` — compute hashes for type/alignment validation during deserialization

### MemCase & Backends

`MemCase<T>` couples a deserialized instance with its memory backend (`MemBackend`), which can be `None` (owned), `Memory` (heap-allocated aligned buffer), or `Mmap` (memory-mapped file). This ensures the backing memory outlives borrowed references.

### Derive Macros (`epserde-derive/src/lib.rs`)

`#[derive(Epserde)]` generates `CopyType`, `SerInner`, `DeserInner`, `TypeHash`, `AlignHash`, and `AlignTo`. Key behavior:
- **Type parameter replacement**: A *replaceable parameter* is a type parameter that occurs in *bare* form (as `T` itself — not inside `PhantomData`, not as a projection like `T::Assoc`) in a field's type. A replaceable parameter of an unmarked field is ε-copy: the derive substitutes it with `T::DeserType<'a>` in the deserialized form. Marking a field with `#[epserde(force_full_copy)]` opts it out (full-copy deserialization, type verbatim); a field with no replaceable parameter is full-copy as well. The type-level `#[epserde(full_copy(T, ...))]` pins listed parameters to full-copy in `DeserType` only (`SerType` still substitutes them): use it when a nested field type holds the parameter full-copy, so the syntactic walk would wrongly ε-substitute it. The naming is intentional: the field marker *forces* a field that could be ε-copy to full-copy, while the type-level form *declares* parameters the local walk cannot see are full-copy. The type-level `#[epserde(phantom(T, ...))]` is the stronger claim: the listed parameters are phantom throughout the type (nested field types hold them only in `PhantomData` slots), so they are excluded from the replaceable-parameter walk entirely — no `SerType`/`DeserType` substitution and no `SerInner`/`DeserInner` bounds — and can be instantiated with non-serializable types such as `str`. Const parameters are never replaceable (a forwarded const argument is syntactically indistinguishable from a type and must stay verbatim).
- **Static zero-copy assertion**: Uses const blocks to verify zero-copy candidates at compile time
- Supports structs and enums (unit, named, unnamed variants)

### Standard Type Implementations (`impls/`)

Each file implements serialization for a category: `prim.rs` (primitives, Option), `vec.rs` (Vec with zero/deep-copy specialization), `boxed_slice.rs`, `string.rs`, `array.rs`, `tuple.rs` (up to 12 elements), `wrapper.rs` (`&T`/`&mut T` plus Box/Rc/Arc with erasure), `stdlib.rs` (Range, Bound, ControlFlow, Result, BuildHasherDefault), `slice.rs`, `iter.rs` (SerIter wrapper).

### PhantomData

`PhantomData<T>` is handled natively by the derive: `T` is substituted inside the phantom slot of the deserialization type, so a parameter that appears both in a `PhantomData` field and elsewhere stays consistent. The legacy `PhantomDeserData<T>` workaround is `#[deprecated]` (see the doc on `epserde::PhantomDeserData`); new code should use plain `PhantomData<T>`.

## Development Guidelines

This project follows https://github.com/vigna/rust-dev-guidelines. Key conventions:

- Modules use directories with `mod.rs`; plural names for countables (`traits`, `impls`)
- Source file order: declaration → derivable trait impls → macros → inherent impls → crate trait impls → external crate impls → stdlib impls
- Tests return `anyhow::Result<()>` with `?`; avoid `unwrap`/`expect`; actual value first in asserts
- Minimize trait bounds on generics
- Macros reference types with `::` prefix; use `$crate::` in declarative macros

## Features

- `default = ["std", "mmap", "derive"]`
- `derive`: Procedural macros
- `std`: Standard library support (crate supports `no_std` with `alloc`)
- `mmap`: Memory-mapped file support
- `schema`: Schema output for debugging

## Key Invariants

- Zero-copy types must be `#[repr(C)]` and contain no references
- Type hashes include alignment/padding info — mismatches cause deserialization errors
- Serialization writes uninitialized padding bytes (unsafe)
- A *replaceable parameter* is a type parameter occurring in bare form (as `T` itself, not inside `PhantomData` and not as a projection like `T::Assoc`) in a field's type. Every replaceable parameter of an unmarked field is ε-copy; the derive substitutes it with `<T as DeserInner>::DeserType<'_>` in the deserialization type. Marking a field with `#[epserde(force_full_copy)]` pins it to full-copy deserialization, keeps its type verbatim in the deserialization type, and excludes its replaceable parameters from the ε-copy set. Fields with no replaceable parameter default to full-copy as well, since they have nothing to substitute. The type-level `#[epserde(full_copy(T, ...))]` instead removes listed parameters from the *deserialization* substitution set only (`SerType` keeps substituting them, since σ-normalization is orthogonal to the ε/full witness): a field whose only replaceable parameters are forced is then full-copy deserialized. It is the declarative alternative to the `DeserFixedPoint` bound for resolving an ε-copy/full-copy mismatch, and is rejected on zero-copy types, const parameters, and unknown idents. Pinning is sound only when the field type actually holds the parameter full-copy (so its own `DeserType` keeps it verbatim); when a pinned parameter shares a field with an ε-copy parameter *and* the field type deserializes the pinned one ε-copy (e.g. `ControlFlow<F, E>`), the derive emits a `FullCopyConsistent` assertion that surfaces an actionable diagnostic instead of a raw slot mismatch. Because the broken case is syntactically indistinguishable from the legitimate one (`Inner<F>` holding `F` in a `force_full_copy` slot alongside an ε-copy `G` in the same field), the check is type-level: it stays silent when the field's real `DeserType` matches the emitted slot. The type-level `#[epserde(phantom(T, ...))]` makes the stronger claim that the listed parameters are phantom throughout the type: they are removed from the replaceable-parameter walk altogether, so *both* substitution sets leave them verbatim and no `SerInner`/`DeserInner` bounds are emitted for them, allowing instantiation with non-serializable types (e.g., `str`). It shares the `full_copy` rejections and is additionally mutually exclusive with `full_copy` on the same parameter. When transplanting a parameter's bounds onto its substituted forms, relaxed bounds (`?Sized`) are filtered out, as Rust permits them only on the item's own type parameters.
