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
and a [`MemCase`] structure that couples a deserialized structure with its backend 
(e.g., a slice of memory or a memory-mapped region).

## Who

Tommaso Fontana, while working at INRIA under the supervision of Stefano Zacchiroli, 
came up with the basic idea for ε-serde, that is, 
replacing structures with equivalent references. The code was developed jointly
with Sebastiano Vigna, who came up with the [`MemCase`] logic.

## Cons

These are the main limitations you should be aware of before choosing to use ε-serde:

- Your types cannot contain references. For example, you cannot use ε-serde on a tree.

- While we provide procedural macros that implement serialization and deserialization, 
they require that your type is written and used in a specific way; in particular, 
the fields you want to ε-copy must be type parameters implementing
[`DeserializeInner`], to which a [deserialized type](`DeserializeInner::DeserType`) is associated.
For example, we provide implementations for
`Vec<T>`/`Box<[T]>`, where `T` [is zero-copy](`ZeroCopy`), or `String`/`Box<str>`, which have 
associated deserialized type `&[T]` or `&str`, respectively.

- After deserialization, you will obtain a structure in which the type parameters
have been instantiated to their respective associated deserialized type, which will usually reference the underlying
serialized support (e.g., a memory-mapped region). If you need to store
the deserialized structure of type `T` in a field of a new structure 
you will need to couple permanently the deserialized structure with its serialized
support, which is obtained by putting it in a [`MemCase`]. A [`MemCase`] will
deref to `T`, so it can be used transparently as long as fields and methods are 
concerned, but the field of the new structure will have to be of type `MemCase<T>`,
not `T`.

- Until Rust gets specialization, vectors and boxed slices can be automatically
(e.g., using derive) ε-copy serialized and deserialized *only* if the type 
of their elements [is zero-copy](`ZeroCopy`). If you need to
store, say, a vector of vectors of integers, you must to implement the 
(de)serialization logic by yourself.

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

## Examples

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

We can write methods for our structure that will work for the ε-copied version: we just have
to take care that they are defined in a way that will work both on the original type parameter and on
its associated deserialized type; we can also use `type` to reduce the clutter:
```rust
use epserde::*;
use epserde_derive::*;

#[derive(Serialize, Deserialize, TypeName, Debug, PartialEq)]
struct MyStructParam<A> {
    id: isize,
    data: A,
}

/// This method can be called on both the original and the ε-copied structure
impl<A: AsRef<[isize]>> MyStructParam<A> {
    fn sum(&self) -> isize {
        self.data.as_ref().iter().sum()
    }
}

type MyStruct = MyStructParam<Vec<isize>>;

// Create a structure where A is a Vec<isize>
let s = MyStruct { id: 0, data: vec![0, 1, 2, 3] };
// Serialize it
s.serialize(std::fs::File::create("serialized").unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read("serialized").unwrap();
let t = MyStruct::deserialize_eps_copy(b.as_ref()).unwrap();
// We can call the method on both structures
assert_eq!(s.sum(), t.sum());
```

If you want to map the data structure into memory, you can use a convenience method
that stores the ε-copied structure and its support in a [`MemCase`]:
```rust
use epserde::*;
use epserde_derive::*;

#[derive(Serialize, Deserialize, TypeName, Debug, PartialEq)]
struct MyStructParam<A> {
    id: isize,
    data: A,
}

type MyStruct = MyStructParam<Vec<isize>>;

impl<A: AsRef<[isize]>> MyStructParam<A> {
    fn sum(&self) -> isize {
        self.data.as_ref().iter().sum()
    }
}

let s = MyStruct { id: 0, data: vec![0, 1, 2, 3] };
s.serialize(std::fs::File::create("serialized").unwrap()).unwrap();
// Load the serialized form in a buffer
let f = Flags::empty();
// The type of t will be inferred--it is shown here only for clarity
let t: MemCase<MyStructParam<&[isize]>> =
    epserde::map::<MyStruct>("serialized", &f).unwrap();

// t works transparently as a MyStructParam<&[isize]>
assert_eq!(s.id, t.id);
assert_eq!(s.data, Vec::from(t.data));
assert_eq!(s.sum(), t.sum());
```

