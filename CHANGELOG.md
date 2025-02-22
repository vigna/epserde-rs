# Change Log

## [0.8.0] - 2025-02-22

### New

* The file format has unfortunately changed. The ReprHash of arrays was wrong
  and could have led to data corruption. We took this chance to fix a number of
  ReprHash representation for deep-copy types that were not empty (as they
  should have been).

* The implementation for tuples was broken because it assumed that the memory
  layout would have been the same of the source layout. We now just support
  tuples of zero-copy identical types up to size 12. For the other cases, it is
  necessary to create a `repr(C)` tuple newtype.

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
