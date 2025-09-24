# Change Log

## [0.10.0] - 2025-09-20

### New

* New delegations of standard-library traits to `MemCase`; in particular,
  `AsRef` and `Deref` are back, but with a slightly different semantics, as
  the implementation `Deref` for `MemCase<S>` has target
  `S::DeserType<_>::Target`, and analogously for `AsRef`.

* New strategy for `MemCase::encase`, which uses a transparent wrapper `Owned`
  to bring back the original functionality.

### Changed

* Major disruptive change: vectors and boxed slices have now the same
  `TypeHash`. This makes them interchangeable at will in (de)serialization,
  which is an extremely useful feature. Unfortunately, old instances with
  replaceable type parameters whose concrete type is a vector will no longer be
  deserializable.

* `CopyType` is now unsafe as there is no way to check a type contains
  no references.

* `repr` attributes are now sorted lexicographically. This change was
  necessary as the order of such attributes is irrelevant, but it might make
  impossible to deserialize old instances whose type specifies `repr` attributes
  in a different order.

## [0.9.0] - 2025-09-17

### New

* Major disruptive change: `MemCase` does not implement `Deref` and `AsRef`
  anymore, as such implementations led to undefined behavior. Instead, `MemCase`
  provides an `uncase` method that returns a reference to the deserialized type,
  similarly to the `Yoke` crate. This is a major change as all code using
  `MemCase` must be updated. In particular, accessing the underlying structure
  requires a call do `uncase`, similarly to what happens with the `Borrow` and
  `AsRef` traits, and it is no longer possible to pass a `MemCase` as type
  parameter when the trait bound is `Deref` or `AsRef` to the underlying type.
  Moreover, `encase` still exists, but it accepts only types implementing
  `DeserializeInner` and whose deserialization types is `Self`. Using a
  structure of type `S`and a `MemCase<S>` interchangeably now requires
  implementing the same traits in both cases. For some elaboration, see the
  `MemCase` documentation.

* New `read_mem` and `read_mmap` methods that work like `load_mem` and
  `load_mmap` but accept any `Read` implementation and a length instead of file
  paths. They make writing unit tests involving `MemCase` much easier.

* We now generate a syntax error for types with lifetimes and where clauses
  (which never supported in the first place).

* There is now support for serializing references, and support by erasure
  for `Box`,  `Rc`, and `Arc` in the `pointer` module.

### Changed

* All serialization and deserialization methods are now unsafe. See their
  safety section for more information.

* All deserialization helper methods handling zero-copy types are also unsafe.
  This change is necessary because such methods can deserialize uninhabited
  types.

* The `TypeHash` of tuples has changed as it was ambiguous. If you
  serialized a structure using tuples, it will be no longer deserializable.

### Fixed

* ε-copy deserializing slices of zero-width zero-copy types now works.

* ε-copy deserialization of primitive types will return an error on EOF
  instead of panicking.

* Since the beginning, associated (de)serialization types of zero-copy
  types where built by the derive code using associated (de)serialization
  types of their generic type parameters, but this is not correct and does
  not always work, as the associated (de)serialization type of zero-copy
  type is just `Self`.

* Trait bounds for `TypeHash`, `AlignHash` and `MaxSizeOf` were generated
  incorrectly.

## [0.8.0] - 2025-03-03

### New

* The ReprHash (now AlignHash) of arrays was wrong and could have led to data
  corruption. As a result, some serialized file might return an alignment
  error.

* The implementation for tuples was broken because it assumed that the memory
  layout would have been the same of the source layout. We now just support
  tuples of zero-copy identical types up to size 12, and `TypeHash` for generic
  tuples up to size 12 to help with the idiom `PhantomData<(T, U)>`. For the
  other cases, it is necessary to create a `repr(C)` tuple newtype. Note that up
  to ε-serde 0.7.0 we provided an erroneous implementation for mixed zero-copy
  types. If you serialized a structure using such a tuple, it will be no longer
  deserializable.

* You can now serialize exact-size iterators that will be deserialized as
  vectors, making it possible to save incrementally structures larger
  than the available memory.

## [0.7.0] - 2025-02-18

### New

* Now `SerializeInner` inner has an associated type `SerType` that is used to
  write the file header. This is done so `Data<&[u32]>` can be conveniently
  serialized as if it were `Data<Vec<u32>>`. There is no change in the file
  format.

## [0.6.3] - 2025-02-07

### New

* Memory-mapping can be disabled using the `mmap` default feature.

## [0.6.2] - 2025-02-07

### Improved

* Added missing implementation of `TypeHash`, `ReprHash`, `MaxSizeOf`,
  `SerializeInner`, `DeserializeInner` for `Range`, `RangeFrom`, `RangeFull`,
  `RangeInclusive`, `RangeTo`, `RangeToInclusive`, `Bound`, `ControlFlow`.

### Fixed

* The return type of `Deserialize::load_full` is how an `anyhow::Result`,
  analogously to the other `load` functions.

## [0.6.1] - 2024-06-03

### Fixed

* Added missing implementation of MaxSizeOf for PhantomData.

## [0.6.0] - 2024-06-03

### Changed

* Updated MemDbg to 0.2.1.

### Fixed

* Added const generic parameters values and names to type hash. Note that
  this change will invalidate type hashes for structures with generic
  constants.

* Fixed handling of zero-sized zero-copy structs eps_deserialization.

## [0.5.1] - 2024-03-18

### Changed

* Added MemDbg, MemSize, and Debug to most structs.

### Fixed

* Renamed the lifetime `'a` in derives to `deserialize_eps_inner_lifetime`
  to avoid clashes.

## [0.5.0] - 2024-03-18

### Changed

* `util` -> `utils` to follow new guidelines.
