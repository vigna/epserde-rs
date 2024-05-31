# Change Log

## [0.5.2] - 2024-05-30

### Changed

* Updated MemDbg to 0.2.1
* Added const generic parameters values and names to type hash.

### Fixed

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
