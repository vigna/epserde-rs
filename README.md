# ε-serde

ε-serde is a Rust framework for *ε*-copy *ser*ialization and *de*serialization.

## Why

Large immutable data structures need time to be deserialized using the [serde](https://serde.rs/)
approach. A possible solution for this problem is given by frameworks such as 
[Abomonation](https://crates.io/crates/abomonation), [rkiv](https://crates.io/crates/rkyv/), and
[zerovec](https://crates.io/crates/zerovec), which provide *zero-copy* deserialization:
the stream of bytes serializing the data structure can be used directly as a Rust structure.
In particular, this approach makes it possible
to map into memory an on-disk data structure, making it available instantly.
It also makes it possible to load the data structure in a memory region with
particular attributes, such as transparent huge pages on Linux. Even when 
using standard memory load and deserialization happen much
faster as the entire structure can be loaded with a single read operation.

ε-serde has the same goals as the zero-copy frameworks above but provides different tradeoffs.

## How

Since in these data structures typically 
most of the data is given by large chunks of memory in the form of slices or vectors,
at deserialization time one can build quickly a proper Rust structure whose referenced
memory, however, is not copied. We call this approach *ε-copy deserialization*, as
typically a minuscule fraction of the serialized data is copied to build the structure.
The result is similar to that of the frameworks above, but with performance identical to 
that of a standard, in-memory Rust structure, as references are resolved at deserialization
time.

We provide procedural macros implementing serialization and deserialization methods,
basic (de)serialization for primitive types, vectors, etc.,
convenience memory-mapping methods based on [mmap_rs](https://crates.io/crates/mmap-rs), 
and a `MemCase` structure that couples a deserialized structure with its backend 
(e.g., a slice of memory or a memory-mapped region).

## Cons

These are the main limitations you should be aware of before choosing to use ε-serde:

- Your types cannot contain references. For example, you cannot use ε-serde on a tree.

- While we provide procedural macros that implement serialization and deserialization, 
they require that your type is written and used in a specific way; in particular, 
the fields you want to ε-copy must be type parameters implementing
[`DeserializeEpsCopy`], to which a [deserialized type](`DeserializeEpsCopyInner::DeserType`) is associated.
For example, we provide implementations for
`AsRef<[T]>`, where `T` [`IsZeroCopy`], or `AsRef<str>`, which have deserialized type
`&[T]` or `&str`, respectively.

- After deserialization, you will obtain a structure in which the type parameters
have been instantiated to their respective deserialized type, which will usually reference the underlying
serialized support (e.g., a memory-mapped region). If you need to store
the deserialized structure of type `T` in a field of a new structure or to pass it
around as a function argument,
you will need to couple permanently the deserialized structure with its serialized
support, which is obtained by putting it in a [`MemCase`]. A [`MemCase`] will
deref to `T`, so it can be used transparently as long as methods are 
concerned, but the field of the new structure will have to be of type `MemCase<T>`,
not `T`.

## Pros

- Almost instant deserialization with minimal allocation, provided that you designed
your type following the ε-serde guidelines.

- The structure you get by deserialization is of the same type as the structure
you serialized (but with different type parameters).
This is not the case with [rkiv](https://crates.io/crates/rkyv/),
which requires you to reimplement all methods on the deserialized type.

- The structure you get by deserialization has exactly the same performance as
the structure you serialized. This is not the case with
[zerovec](https://crates.io/crates/zerovec).

- You can deserialize from read-only supports, as all dynamic information generated at
deserialization time is stored in newly allocated memory. This is not the case with
[Abomonation](https://crates.io/crates/abomonation).

## An Example

Let us design a structure that will contain an integer,
which will be copied, and a vector of integers that we want to ε-copy:
```rust
use epserde::*;
use epserde_derive::*;

#[derive(Serialize, Deserialize, TypeName, Debug, PartialEq)]
struct MyStruct<A> {
    id: isize,
    data: A,
}

// Create a structure where A is a Vec<isize>
let s: MyStruct<Vec<isize>> = MyStruct { id: 0, data: vec![0, 1, 2, 3] };
// Serialize it
s.serialize(std::fs::File::create("serialized").unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read("serialized").unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: MyStruct<&[isize]> = 
    <MyStruct<Vec<isize>>>::deserialize_eps_copy(b.as_ref()).unwrap();

assert_eq!(s.id, t.id);
assert_eq!(s.data, Vec::from(t.data));

// This is a traditional deserialization instead
let t: MyStruct<Vec<isize>> = 
    <MyStruct::<Vec<isize>>>::deserialize_full_copy(b.as_ref()).unwrap();
assert_eq!(s, t);
```
Note how the field originally containing a `Vec<isize>` now contains a `&[isize]` (this 
replacement is generated automatically). The reference points inside `b`, so there is 
no need to copy the field. Nonetheless, deserialization creates a new structure `MyStruct`,
ε-copying the original data. The second call creates a full copy instead.

If you want to memory-map the data structure, it is convenient to store the ε-copied structure
and its support in a [`MemCase`]:
```rust
use epserde::*;
use epserde_derive::*;

#[derive(Serialize, Deserialize, TypeName, Debug, PartialEq)]
struct MyStruct<A> {
    id: isize,
    data: A,
}

let s: MyStruct<Vec<isize>> = MyStruct { id: 0, data: vec![0, 1, 2, 3] };
s.serialize(std::fs::File::create("serialized").unwrap()).unwrap();
// Load the serialized form in a buffer
let f = Flags::empty();
// The type of t will be inferred--it is shown here only for clarity
let t: MemCase<MyStruct<&[isize]>> = 
    epserde::map::<_,MyStruct<Vec<isize>>>("serialized", &f).unwrap();

assert_eq!(s.id, t.id);
assert_eq!(s.data, Vec::from(t.data));
```

