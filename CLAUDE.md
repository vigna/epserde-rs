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

# Run a single test
cargo test test_name -- --skip fail

# Run all examples
cd epserde && for f in examples/*.rs; do cargo run --example "$(basename "${f%.rs}")"; done
```

CI tests against Rust 1.85.0, stable, beta, and nightly, with three feature combinations: default, `--no-default-features --features 'derive'`, and `--all-features`.

## Architecture

### Core Type System

The framework classifies types into two categories via the `CopyType` trait:

- **`ZeroCopy`** (`type Copy = Zero`): Fixed-layout types with no references. Must be `#[repr(C)]`. Serialized/deserialized as raw bytes. Marked with `#[epserde_zero_copy]`.
- **`DeepCopy`** (`type Copy = Deep`): Types requiring recursive field-by-field processing. Marked with `#[epserde_deep_copy]`.

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
- **Type parameter replacement**: Replaceable type params (those appearing as field types) are substituted with `T::DeserType<'a>` in the deserialized form
- **Static zero-copy assertion**: Uses const blocks to verify zero-copy candidates at compile time
- Supports structs and enums (unit, named, unnamed variants)

### Standard Type Implementations (`impls/`)

Each file implements serialization for a category: `prim.rs` (primitives), `vec.rs` (Vec with zero/deep-copy specialization), `boxed_slice.rs`, `string.rs`, `array.rs`, `tuple.rs` (up to 12 elements), `pointer.rs` (Box/Rc/Arc with erasure), `stdlib.rs` (Option, Range, ControlFlow), `slice.rs`, `iter.rs` (SerIter wrapper).

### PhantomDeserData

When a deep-copy type has a type parameter `T` appearing both in a field and in a `PhantomData`, use `PhantomDeserData<T>` instead of `PhantomData<T>` to avoid type mismatch after parameter replacement.

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
- A replaceable type parameter must not appear both as a field type and as a type argument of another field's type. The restriction can be lifted by marking the wrapper field with `#[epserde(force_repl)]` when its type substitutes the parameter transitively, or alternatively by marking the direct field with `#[epserde(force_irrepl)]` to pin the parameter as irreplaceable across the struct. Occurrences inside `PhantomData<…>` do not count toward either classification.
