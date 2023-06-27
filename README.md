# epserde

**epserde is a Rust framework for *ε*-copy *ser*ialization and *de*serialization.**

## Why

Large immutable data structures need time to be deserialized using the [serde](https://serde.rs/)
approach. A possible solution for this problem is given by frameworks such as [rkiv](https://github.com/rkyv/rkyv) and
[zerovec](https://docs.rs/zerovec/latest/zerovec/), which provide *zero-copy* deserialization:
the stream of byte serializing the data structure can be used directly as a Rust structure.
In particular, this approach makes it possible
to map into memory the data structure, making it available instantly. 

However, in both cases, the performance of the data structure is not the same as that
of a standard, deserialized Rust structure. 

## How

Since in these data structures typically 
most of the data is given by large chunks of memory in the form of slices or vectors,
at deserialization time one can build quickly a proper Rust structure whose referenced
memory, however, is not copied. We call this approach *ε-copy deserialization*, as
typically a minuscule fraction of the serialized data is copied to build the structure.
The result is similar to that of the frameworks above, but with performance identical to 
that of a standard, in-memory Rust structure.

We provide procedural macros implementing serialization and deserialization methods, memory-mapping
methods based on [mmap_rs](https://crates.io/crates/mmap-rs), and a `MemCase` structure
that couples a deserialized structure with its backend (e.g., a slice of memory or a
memory-mapped region).
