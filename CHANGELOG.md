# Change Log

## [0.13.0] - Unreleased

### New

- A type parameter can be declared phantom throughout the type with the
  type-level `#[epserde(phantom(T, …))]` attribute: every occurrence of the
  listed parameters in field types must be inside a `PhantomData`; neither
  `SerType` nor `DeserType` substitutes them, and no `SerInner`/`DeserInner`
  bounds are emitted for them, so they can be instantiated with
  non-serializable types such as `str`.

- `PhantomData<T>` is now handled natively by the derive: `T` is substituted
  inside the phantom slot of the deserialization type, so a parameter that
  appears both in a `PhantomData` field and elsewhere stays consistent.

- `PhantomDeserData` is deprecated since 0.13.0 in favor of plain
  `PhantomData`; note that the migration changes the type hash.

- Also mutable references to slices can be now serialized.

- Much better diagnostic for violations of ε-copy stability.

- Support for `Result`.

### Changed

- Major design change: ε-copy deserialization is always propagated through
  fields (used to stop at fields whose type is not a parameter). The old
  behavior can be obtained by decorating the field with
  `#[epserde(force_full_copy)]`. The new design opens the possibility for type
  replacement inside fields whose type is not a parameter (e.g., `S<A>([A; 3])`
  can have `Vec<usize>` as a parameter, getting the deserialization type
  `S<&[usize]>`).

- `TypeHash` and `AlignHash` now are computed using SHA-256 (the major version
  of the file format has been bumped).

- Renamed the `AlignTo` trait to `PadTo`, and its method `align_to` to
  `pad_to`: the value is the power-of-two boundary to which serialized
  zero-copy data is padded, and the previous name was too easily confused
  with `std::mem::align_of`, which is a different quantity (references need
  `align_of`; the stream is padded to `pad_to`).

- A type parameter can be pinned to full-copy deserialization across the whole
  type with the type-level `#[epserde(full_copy(T, …))]` attribute.

- Unrecognized field-level `#[epserde(...)]` keys are now rejected at derive
  time (previously they were silently ignored); the only valid field-level key
  is `force_full_copy`.

- "Replaceable" (type parameter) and "irreplaceable" are now "ε-copy" and
  "full-copy".

- `TypeHash` now uses a fully-qualified name for user-defined types.
  This will break the serialization format in all practical cases.
  Hence, the major revision of the file format was bumped.

- Deserialization no longer validates `NonZero*`, `char`, or `String` values
  (previously the full-copy paths, and the ε-copy paths of scalar values,
  panicked on invalid data), and `bool` is no longer coerced (previously any
  nonzero byte deserialized as true): all deserialization now uses unchecked
  conversions, making the safety contract uniform (deserialized data must
  come from a correct serialization).

- The serialization helpers `ser_zero`, `ser_zero_unchecked`, `ser_slice_zero`,
  and `ser_slice_deep`, the method `WriteWithNames::write`, and the
  deserialization helpers `deser_full_vec_deep` and `deser_eps_vec_deep` are
  now `unsafe`, closing safe paths to the operations that the
  `Serialize`/`Deserialize` contracts gate.

- `ser::Error` and `deser::Error` are now `#[non_exhaustive]`; the new
  variants `ser::Error::IoError` and `deser::Error::IoError`, available with
  the `std` feature, carry the underlying `std::io::Error` of the writer or
  reader; `deser::Error::FileOpenError` is now used by all convenience
  methods and its message has been fixed.

- A corrupted or truncated length prefix claiming an implausible capacity now
  returns the new `deser::Error::CapacityOverflow` variant instead of panicking
  or aborting inside the allocator, so all header-valid corruption is reported
  as a recoverable error.

- The `AlignHash` of `Option`, `Bound`, `ControlFlow`, and `Result` now
  forwards the alignment hash of their payload types, so cross-architecture
  layout mismatches of zero-copy payloads are detected; the alignment hash of
  `RangeInclusive` no longer includes a `bool` that is never serialized. This
  breaks the serialization format for `Option`, `Bound`, `ControlFlow`, and
  `RangeInclusive`.

- The `AlignHash` of a deep-copy enum now hashes each field with a fresh zero
  offset, mirroring deep-copy structs, since every field is realigned in the
  stream; previously the offset accumulated across the fields of a variant
  (and across variants). This breaks the serialization format for deep-copy
  enums with zero-copy fields.

- `SliceWithPos::skip` can now return an error.

- The `TypeHash` of a zero-copy enum now includes its discriminant values,
  since such an enum is (de)serialized as raw bytes and re-numbering its
  variants changes the encoding. For a fieldless enum, every variant's
  discriminant is hashed as its cast integer value; for a data-carrying enum
  (which cannot be cast), every variant's resolved running discriminant is
  hashed (the last explicit discriminant expression, evaluated in the enum's
  scope, plus the number of variants since). In both cases a change in the
  value of a named constant used as a discriminant is detected, and
  equivalent spellings of the same value (implicit or explicit, literal or
  named constant) hash equal. This makes such changes a detectable type-hash
  mismatch rather than a silent mis-decode, and breaks the serialization
  format for all zero-copy enums.

- `BuildHasherDefault` is now classified as deep-copy, making sequences of it
  serializable (it is not `Copy`, so it could never satisfy the zero-copy
  bound).

- `SerIter` now documents that serialization consumes the iterator; it stops
  writing at the declared length when an iterator under-reports its length;
  its deep-copy path now records per-item schema entries; `SerIter::new` is
  now `const`; and spurious derived trait bounds on its phantom parameter have
  been removed (the `Clone`, `PartialEq`, `Eq`, and `Default` implementations
  were dropped, and `Debug` is now bounded only on the iterator).

- `AlignedCursor::new` and `AlignedCursor::set_position` are now `const`;
  read-only accessors no longer require `Default + Clone`; a zero-sized
  alignment type is now rejected at compile time; the `no_std` `write_all` now
  honors all-or-error semantics, returning an error instead of silently
  truncating at the `usize::MAX` length limit; and the `no_std` `read_exact`
  no longer overflows on positions near `usize::MAX`. Reading or writing
  zero bytes at a position past the end now succeeds without any effect (in
  particular, an empty write no longer extends the cursor length), matching
  the standard `read_exact`/`write_all` contracts on both the `std` and
  `no_std` implementations.

- `MemCase::encase` has been moved so that type inference works: it no longer
  needs a turbofish.

- Header checking now reads the serialized type name as raw bytes with lossy
  UTF-8 conversion, as it is diagnostic data coming from a possibly foreign
  file; the name kept for diagnostics is capped at 8192 bytes, so a hostile
  length prefix can no longer drive an unbounded allocation; the
  deserializing type's names are no longer allocated on the success path.

- Stream alignment now writes padding in chunks rather than one byte at a
  time.

- `Schema::debug` has been renamed `Schema::to_csv_with_data`, mirroring the
  sibling `Schema::to_csv`.

- The inherent method `MemBackend::as_ref` has been renamed
  `MemBackend::as_bytes`, since it returns `Option<&[u8]>` rather than following
  the `AsRef` convention.

- The `data` and `pos` fields of `SliceWithPos` are now `pub(crate)`; read them
  through the new `SliceWithPos::data` and `SliceWithPos::pos` accessors
  instead. This removes the hazard of setting `pos` inconsistently with `data`.

- Variants of `ser::Error` and `deser::Error` have more precise names (e.g.,
  `AlignHashMismatch` instead of `WrongAlignHash`).

- `epserde_zero_copy` and `epserde_deep_copy` are no longer available.

- `AlignedCursor` now uses sealed types to avoid easy UB.

### Fixed

- Fixed alignment issues with zero-width types: in particular, ε-copy
  deserialization of zero-sized zero-copy types with nonzero alignment
  (e.g., aligned unit structs, `[T; 0]`, and vectors thereof) no longer
  desynchronizes the stream, which could silently corrupt every following
  field.

- The `PadTo` implementation generated for zero-copy enums now imports the
  trait with a fully qualified path, so deriving `Epserde` on such an enum
  compiles also in files that do not import the prelude.

- Fixed a few possible UB soundness issues and leaks when I/O errors happen
  during (de)serialization; in particular, `MemCase`-producing methods no
  longer leak the memory backend if ε-copy deserialization fails or panics,
  and array deserialization no longer leaks initialized elements if an
  element deserialization panics.

- Now we read correctly attributes like `#[repr(C, align(N))]`, and the
  contribution of such attributes to the alignment hash is normalized.

- Serializing an exhausted `RangeInclusive` now correctly returns an error,
  as the range cannot be deserialized.

- The `TypeHash` of `Vec<T>` was the same as that of `Box<[T]>`, which would
  have made them equivalent as parameters in `PhantomData`.

- The derive macro no longer requires a spurious `DeepCopy` bound on a type
  parameter that occurs only inside a `PhantomData` sequence (e.g.
  `PhantomData<Vec<U>>`) sharing a field with an ε-copy sequence.

- The derive macro no longer breaks on an enum named-variant field called
  `backend`, and now recognizes `::core::marker::PhantomData` written with a
  leading path separator.

- `#[derive(TypeInfo)]` on generic zero-copy types no longer requires
  `SerInner` bounds on the type parameters.

- Homogeneous tuples now forward `IS_ZERO_COPY` from their element type
  instead of hardcoding `true`.

- `ser::Error` now exposes the underlying I/O error as the `source()` of its
  `FileOpenError` variant.

- A corrupted or truncated length prefix claiming an implausible capacity now
  returns the new `deser::Error::CapacityOverflow` variant instead of panicking
  or aborting inside the allocator, so all header-valid corruption is reported
  as a recoverable error.

## [0.12.6] - 2026-04-02

### Fixed

- No more warnings for `mem_size_flat`.

## [0.12.5] - 2026-03-26

### New

- `*const T` now implements `TypeHash`, making it possible to use the
  expression in tuples in a `PhantomData` when `T` is unsized.

### Fixed

- Fixed error message for usage of `epserde_zero_copy`.

## [0.12.4] - 2026-03-17

### New

- It is now possible to add bounds to the serialization and deserialization
  types. This is particularly useful with associated types, as it makes it
  possible, for example, to pin them in the deserialization type so that they
  are identical to the original type.

- Better attribute names `epserde(zero_copy)` and `epserde(deep_copy)`.

### Fixed

- Too large files now generate an error on 32-bit architectures.

- Fixed regression tests and pinned them to known architectures.

## [0.12.3] - 2026-03-07

### Changed

- Removed dependency from `common_traits`.

## [0.12.2] - 2026-02-20

### New

- Covariance-checking macros for leaner code and custom-implementation
  support.

### Improved

- All `__check_covariance` implementations are now `#[inline(always)]`.

### Changed

- `__check_field_covariance` has been renamed `__check_type_covariance`.

## [0.12.1] - 2026-02-17

- Covariance-checking infrastructure makes undefined behavior from `MemCase`
  impossible, unless implementors use an unsafe bypass. This removes one of
  the possible sources of undefined behavior in deserialization.

## [0.12.0] - 2026-02-11

### Changed

- Removed old attributes `zero_copy` and `deep_copy` as they were deprecated in favor of
  `epserde_zero_copy` and `epserde_deep_copy` in 0.10.4.

- Updated `mmap-rs` dependency to 0.7.0 and `mem_dbg` to 0.4.0.

### Fixed

- What were previously panics in the derive code are now proper compiler
  errors.

- Several bug fixes, in particular the deserialization of `ControlFlow`.

- The `AlignHash` implementation for ranges has been fixed. This might
  cause some serialized files to be no longer deserializable.

- The `AlignHash` of `Option` and `ControlFlow` has been made into a no-op,
  as it should. This might cause some serialized files to be no longer
  deserializable.

- The `Send`/`Sync` bounds for `MemCase` are now on the deserialization
  type and not on the deserialization type.

## [0.11.5] - 2025-12-19

### Changed

- Removed dependency on `maligned`. The internal types `Aligned16` and
  `Aligned64` have been reimplemented directly in this crate.

## [0.11.4] - 2025-11-03

- Added missing `TypeHash` implementations for references to `str` and slices.

- Added support for `std::hash::DefaultHash` and
  `core::hash::BuildHasherDefault`.

- Removed useless lifetime from `SerIter`.

## [0.11.3] - 2025-10-21

### Changed

- `SerIter` now is generic over iterators whose items implement `Borrow<T>`,
  rather than `&T`.

### Fixed

- Fixed `mmap` dependency to 0.6.

## [0.11.2] - 2025-10-21

### Fixed

- Added missing `CopyType` to `&str` and references to slices.

- Relaxed `mmap` dependency to >= 0.6, <= 0.7.

## [0.11.1] - 2025-10-16

### Changed

- Updated `mmap` dependency.

## [0.11.0] - 2025-10-15

### New

- Thanks to const blocks, the previous runtime check for a deep-type type being
  a candidate for zero-copy is now a static assertion. This is a significant
  improvement, but it requires a major release as previous code might not
  compile anymore.

## [0.10.4] - 2025-09-30

### New

- The attributes `zero_copy` and `deep_copy` were dangerously commonly named,
  and they have been replaced by `epserde_zero_copy` and `epserde_deep_copy`.
  The old names are still supported but deprecated, and will be removed in a
  future release. There are deprecation warnings, but unfortunately for the time
  being they will only appear on nightly.

### Changed

- Revamped examples; new `schema` feature for enabling schema printing.

## [0.10.3] - 2025-09-26

### Fixed

- Removed usage of `is_multiple_of`.

## [0.10.2] - 2025-09-26

### Fixed

- Restored previous type hash of string (only for use with `PhantomData`).

- Added back previous type hashes for `str` and `[T]` for use with
  `PhantomData`.

- Revised bound propagation (again).

- Renamed `MaxSizeOf` to `AlignTo` as it includes results from
  `std::mem::align_of` (renamed again to `PadTo` in 0.13.0).

### Changed

- The indices of ranges and bounds from the standard library can now be
  deep-copy.

## [0.10.1] - 2025-09-25

### Fixed

- Fixed type hash of strings.

## [0.10.0] - 2025-09-25

### New

- New delegations of standard-library traits to `MemCase`; in particular,
  `AsRef` and `Deref` are back, but with a slightly different semantics, as
  the implementation `Deref` for `MemCase<S>` has target
  `S::DeserType<_>::Target`, and analogously for `AsRef`.

- New strategy for `MemCase::encase`, which uses a transparent wrapper `Owned`
  to bring back the original functionality.

- New `SerType` type alias, analogous to `DeserType`.

- Major internal code restructuring: `TypeHash`/`AlignHash`/`PadTo` are now
  computed on the serialization type, not on the serializing type.

- New convenience serialization implementation for `&str`, in the same vain as
  that for `&[T]`.

### Changed

- Major disruptive change: vectors and boxed slices have now the same
  serialization type. This makes them interchangeable at will in
  (de)serialization, which is an extremely useful feature. Unfortunately, old
  instances with ε-copy type parameters whose concrete type is a vector
  will no longer be deserializable. The same happens for `String` and
  `Box<str>`.

- `CopyType` is now unsafe as there is no way to check a type contains
  no references.

- `repr` attributes are now sorted lexicographically. This change was
  necessary as the order of such attributes is irrelevant, but it might make
  impossible to deserialize old instances whose type specifies `repr` attributes
  in a different order.

- A few `TypeHash`/`AlignHash` implementation that were not really necessary
  have been removed.

### Fixed

- The `nostd` feature now works.

## [0.9.0] - 2025-09-17

### New

- Major disruptive change: `MemCase` does not implement `Deref` and `AsRef`
  anymore, as such implementations led to undefined behavior. Instead, `MemCase`
  provides an `uncase` method that returns a reference to the deserialization
  type, similarly to the `Yoke` crate. This is a major change as all code using
  `MemCase` must be updated. In particular, accessing the underlying structure
  requires a call do `uncase`, similarly to what happens with the `Borrow` and
  `AsRef` traits, and it is no longer possible to pass a `MemCase` as type
  parameter when the trait bound is `Deref` or `AsRef` to the underlying type.
  Moreover, `encase` still exists, but it accepts only types implementing
  `DeserInner` and whose deserialization types is `Self`. Using a
  structure of type `S`and a `MemCase<S>` interchangeably now requires
  implementing the same traits in both cases. For some elaboration, see the
  `MemCase` documentation.

- New `read_mem` and `read_mmap` methods that work like `load_mem` and
  `load_mmap` but accept any `Read` implementation and a length instead of file
  paths. They make writing unit tests involving `MemCase` much easier.

- We now generate a syntax error for types with lifetimes and where clauses
  (which never supported in the first place).

- There is now support for serializing references, and support by erasure
  for `Box`, `Rc`, and `Arc` in the `pointer` module.

### Changed

- All serialization and deserialization methods are now unsafe. See their
  safety section for more information.

- All deserialization helper methods handling zero-copy types are also unsafe.
  This change is necessary because such methods can deserialize uninhabited
  types.

- The `TypeHash` of tuples has changed as it was ambiguous. If you
  serialized a structure using tuples, it will be no longer deserializable.

### Fixed

- ε-copy deserializing slices of zero-width zero-copy types now works.

- ε-copy deserialization of primitive types will return an error on EOF
  instead of panicking.

- Since the beginning, associated (de)serialization types of zero-copy
  types where built by the derive code using associated (de)serialization
  types of their generic type parameters, but this is not correct and does
  not always work, as the associated (de)serialization type of zero-copy
  type is just `Self`.

- Trait bounds for `TypeHash`, `AlignHash` and `PadTo` were generated
  incorrectly.

## [0.8.0] - 2025-03-03

### New

- The ReprHash (now AlignHash) of arrays was wrong and could have led to data
  corruption. As a result, some serialized file might return an alignment
  error.

- The implementation for tuples was broken because it assumed that the memory
  layout would have been the same of the source layout. We now just support
  tuples of zero-copy identical types up to size 12, and `TypeHash` for generic
  tuples up to size 12 to help with the idiom `PhantomData<(T, U)>`. For the
  other cases, it is necessary to create a `repr(C)` tuple newtype. Note that up
  to ε-serde 0.7.0 we provided an erroneous implementation for mixed zero-copy
  types. If you serialized a structure using such a tuple, it will be no longer
  deserializable.

- You can now serialize exact-size iterators that will be deserialized as
  vectors, making it possible to save incrementally structures larger
  than the available memory.

## [0.7.0] - 2025-02-18

### New

- Now `SerInner` inner has an associated type `SerType` that is used to
  write the file header. This is done so `Data<&[u32]>` can be conveniently
  serialized as if it were `Data<Vec<u32>>`. There is no change in the file
  format.

## [0.6.3] - 2025-02-07

### New

- Memory-mapping can be disabled using the `mmap` default feature.

## [0.6.2] - 2025-02-07

### Improved

- Added missing implementation of `TypeHash`, `ReprHash`, `PadTo`,
  `SerInner`, `DeserInner` for `Range`, `RangeFrom`, `RangeFull`,
  `RangeInclusive`, `RangeTo`, `RangeToInclusive`, `Bound`, `ControlFlow`.

### Fixed

- The return type of `Deserialize::load_full` is how an `anyhow::Result`,
  analogously to the other `load` functions.

## [0.6.1] - 2024-06-03

### Fixed

- Added missing implementation of PadTo for PhantomData.

## [0.6.0] - 2024-06-03

### Changed

- Updated MemDbg to 0.2.1.

### Fixed

- Added const generic parameters values and names to type hash. Note that
  this change will invalidate type hashes for structures with generic
  constants.

- Fixed handling of zero-sized zero-copy structs eps_deserialization.

## [0.5.1] - 2024-03-18

### Changed

- Added MemDbg, MemSize, and Debug to most structs.

### Fixed

- Renamed the lifetime `'a` in derives to `deser_eps_inner_lifetime`
  to avoid clashes.

## [0.5.0] - 2024-03-18

### Changed

- `util` -> `utils` to follow new guidelines.
