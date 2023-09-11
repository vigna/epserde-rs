# ε-serde ![GitHub CI](https://github.com/vigna/epserde-rs/actions/workflows/rust.yml/badge.svg) ![Rust Version](https://img.shields.io/badge/status-stable-success) [![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0) [![License: LGPL v2.1](https://img.shields.io/badge/license-LGPL_2.1-blue.svg)](https://www.gnu.org/licenses/lgpl-2.1)

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
associated deserialized type `&[T]` or `&str`, respectively. Vectors and boxed slices of
types that are not zero copy will be fully deserialized in memory instead.

- After deserialization, you will obtain an associated deserialized type, which 
will usually reference the underlying
serialized support (e.g., a memory-mapped region). If you need to store
the deserialized structure of type `T` in a field of a new structure 
you will need to couple permanently the deserialized structure with its serialized
support, which is obtained by putting it in a [`MemCase`]. A [`MemCase`] will
deref to `T`, so it can be used transparently as long as fields and methods are 
concerned, but the field of the new structure will have to be of type `MemCase<T>`,
not `T`.

## Pros

- Almost instant deserialization with minimal allocation, provided that you designed
your type following the ε-serde guidelines or that you use standard types.

- The structure you get by deserialization is essentially of the same type as the structure
you serialized (e.g., vectors become references to slices, structures remain the same 
but with different type parameters, etc.).
This is not the case with [rkiv](https://crates.io/crates/rkyv/),
which requires you to reimplement all methods on the deserialized type.

- The structure you get by deserialization has exactly the same performance as
the structure you serialized. This is not the case with
[zerovec](https://crates.io/crates/zerovec).

- You can deserialize from read-only supports, as all dynamic information generated at
deserialization time is stored in newly allocated memory. This is not the case with
[Abomonation](https://crates.io/crates/abomonation).

## Example: Zero copy of standard types

Let us start with the simplest case: data that can be zero copied. In this case,
we serialize an array of a thousand zeros, and get back a reference to such 
an array:
```rust
use epserde::*;

let s = [0_usize; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized0");
s.serialize(std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: &[usize; 1000] =
    <[usize; 1000]>::deserialize_eps_copy(b.as_ref()).unwrap();

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: [usize; 1000] = 
    <[usize; 1000]>::deserialize_full_copy(std::fs::File::open(&file).unwrap()).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<&[usize; 1000]> = 
    epserde::map::<[usize; 1000]>(&file, Flags::empty()).unwrap();
assert_eq!(s, **u);
```
Note how we serialize an array, but we deserialize a reference. 
The reference points inside `b`, so there is 
no copy performed. The second call creates a new array instead.
The third call maps the data structure into memory and returns
a [`MemCase`] that can be used transparently as a reference to the array;
moreover, the [`MemCase`] can be passed to other functions or stored
in a structure field, as it contains both the structure and the
memory-mapped region that supports it.

## Examples: ε-copy of standard structures

Zero copy is not that interesting because it can be applied only to
data whose memory layout is stable and known at compile time. 
This time, let us serialize a `Vec` containing a 
a thousand zeros: ε-serde will deserialize its associated
deserialization type, which is a reference to a slice.
```rust
use epserde::*;

let s = vec![0; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized1");
s.serialize(std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: &[usize] =
    <Vec<usize>>::deserialize_eps_copy(b.as_ref()).unwrap();

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<usize> = 
    <Vec<usize>>::load_full(&file).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<&[usize]> = 
    epserde::map::<Vec<usize>>(&file, Flags::empty()).unwrap();
assert_eq!(s, **u);
```
Note how we serialize a vector, but we deserialize a reference
to a slice; the same would happen when serializing a boxed slice.
The reference points inside `b`, so there is very little
copy performed (in fact, just a field containing the length of the slice).
All this is due to the fact that `usize` is a zero-copy type.
Note also that we use the convenience method [`Deserialize::load_full`].

If your code must work both with the original and the deserialized
version, however, it must be written for a trait that is implemented
by both types, like `AsRef<[usize]>`.

## Example: Zero-copy structures

You can define your own types to be zero copy, in which case they will
work like `usize` in the previous examples. This requires the structure
to be made of zero-copy fields, and to be annotated with `#[zero_copy]` 
and `#[repr(C)]`:
```rust
use epserde::*;
use epserde_derive::*;

#[derive(Epserde, Debug, PartialEq, Copy, Clone)]
#[repr(C)]
#[zero_copy]
struct Data {
    foo: usize,
    bar: usize,
}

let s = vec![Data { foo: 0, bar: 0 }; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized2");
s.serialize(std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: &[Data] =
    <Vec<Data>>::deserialize_eps_copy(b.as_ref()).unwrap();

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<Data> = 
    <Vec<Data>>::load_full(&file).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<&[Data]> = 
    epserde::map::<Vec<Data>>(&file, Flags::empty()).unwrap();
assert_eq!(s, **u);
```
If a structure is not zero copy, vectors will be always deserialized to vectors
(i.e., the full copy and the ε-copy will be the same).

## Examples: ε-copy structures

More flexibility can be obtained by defining structures with fields
whose field types are defined by parameters. In this case, ε-serde
will deserialize the structure replacing its type parameters with
the associated deserialized type.

Let us design a structure that will contain an integer,
which will be copied, and a vector of integers that we want to ε-copy:
```rust
use epserde::*;
use epserde_derive::*;

#[derive(Epserde, Debug, PartialEq)]
struct MyStruct<A> {
    id: isize,
    data: A,
}

// Create a structure where A is a Vec<isize>
let s: MyStruct<Vec<isize>> = MyStruct { id: 0, data: vec![0, 1, 2, 3] };
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized3");
s.serialize(std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: MyStruct<&[isize]> = 
    <MyStruct<Vec<isize>>>::deserialize_eps_copy(b.as_ref()).unwrap();

assert_eq!(s.id, t.id);
assert_eq!(s.data, Vec::from(t.data));

// This is a traditional deserialization instead
let t: MyStruct<Vec<isize>> = 
    <MyStruct::<Vec<isize>>>::load_full(&file).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<MyStruct<&[isize]>> = 
    epserde::map::<MyStruct::<Vec<isize>>>(&file, Flags::empty()).unwrap();
assert_eq!(s.id, u.id);
assert_eq!(s.data, u.data.as_ref());
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

#[derive(Epserde, Debug, PartialEq)]
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
let mut file = std::env::temp_dir();
file.push("serialized4");
s.serialize(std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();
let t = MyStruct::deserialize_eps_copy(b.as_ref()).unwrap();
// We can call the method on both structures
assert_eq!(s.sum(), t.sum());

let t = epserde::map::<MyStruct>(&file, Flags::empty()).unwrap();

// t works transparently as a MyStructParam<&[isize]>
assert_eq!(s.id, t.id);
assert_eq!(s.data, t.data.as_ref());
assert_eq!(s.sum(), t.sum());
```

## Design

Every type serializable with ε-serde has two features that are in principle orthogonal,
but that in practice often condition one another:

- the type has an *associated deserialization type* which is the type you obtain
upon deserialization;
- the type can be either [`ZeroCopy`] or [`EpsCopy`]; it can also be neither.

There is no constraint on the associated deserialization type: it can be literally
anything. In general, however, one tries to have a deserialization type that is somewhat
compatible with the original type: for example, ε-serde deserializes vectors as 
references to slices, so all mutation method that do not change the length work on both.

Being [`ZeroCopy`] or [`EpsCopy`] decides instead how the type will be treated 
when serializing and deserializing sequences, such as slices, boxed slices, and vectors. 
Sequences of zero-copy types are deserialized using a reference, whereas sequences
of ε-copy types are fully deserialized in allocated memory. It is important to remark
that *you cannot serialize a vector whose elements are of a type that is neither*
(see the [`CopyType`] documentation for a deeper explanation).

Logically, zero-copy types should be deserialized to references, and this indeed happens
in most cases, and certainly in the derived code: however, *primitive types are always
fully deserialized*. There are two reasons behind this non-orthogonal choice:

- primitive types occupy so little space that deserializing them as a reference is
not efficient;
- if a type parameter `T` is a primitive type, writing generic code for `AsRef<T>` is
really not nice.

Since this is true only of primitive types, when deserializing a
1-tuple containing a primitive type one obtains a reference (and indeed this
workaround can be used if you really need to deserialize a primitive type as a reference).
The same happens if you deserialize a zero-copy 
struct containing a single field of primitive type.