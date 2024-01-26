# ε-serde

[![downloads](https://img.shields.io/crates/d/epserde)](https://crates.io/crates/epserde)
[![dependents](https://img.shields.io/librariesio/dependents/cargo/epserde)](https://crates.io/crates/epserde/reverse_dependencies)
![GitHub CI](https://github.com/vigna/epserde-rs/actions/workflows/rust.yml/badge.svg)
![license](https://img.shields.io/crates/l/epserde)
[![](https://tokei.rs/b1/github/vigna/epserde-rs?type=Rust,Python)](https://github.com/vigna/epserde-rs)

ε-serde is a Rust framework for *ε*-copy *ser*ialization and *de*serialization.

## WARNING

With release 0.2.0 we had to change the type hash function, so all serialized data
must be regenerated. We apologize for the inconvenience.

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
The result is similar to that of the frameworks above, but the performance of the
deserialized structure will be identical to that of a standard, in-memory
Rust structure, as references are resolved at deserialization time.

We provide procedural macros implementing serialization and deserialization methods,
basic (de)serialization for primitive types, vectors, etc.,
convenience memory-mapping methods based on [mmap_rs](https://crates.io/crates/mmap-rs),
and a [`MemCase`](`deser::MemCase`) structure that couples a deserialized structure with its backend
(e.g., a slice of memory or a memory-mapped region).

## Who

Tommaso Fontana, while working at INRIA under the supervision of Stefano Zacchiroli,
came up with the basic idea for ε-serde, that is,
replacing structures with equivalent references. The code was developed jointly
with Sebastiano Vigna, who came up with the [`MemCase`](`deser::MemCase`) and the
[`ZeroCopy`](traits::ZeroCopy)/[`DeepCopy`](traits::DeepCopy) logic.

## Cons

These are the main limitations you should be aware of before choosing to use ε-serde:

- Your types cannot contain references. For example, you cannot use ε-serde on a tree.

- While we provide procedural macros that implement serialization and deserialization,
they require that your type is written and used in a specific way; in particular,
the fields you want to ε-copy must be type parameters implementing
[`DeserializeInner`](`deser::DeserializeInner`), to which a
[deserialized type](`deser::DeserType`) is associated.
For example, we provide implementations for
`Vec<T>`/`Box<[T]>`, where `T` [is zero-copy](traits::ZeroCopy), or `String`/`Box<str>`, which have
associated deserialized type `&[T]` or `&str`, respectively. Vectors and boxed slices of
types that are not zero-copy will be deserialized recursively in memory instead.

- After deserialization, you will obtain an associated deserialized type, which
will usually reference the underlying
serialized support (e.g., a memory-mapped region). If you need to store
the deserialized structure of type `T` in a field of a new structure
you will need to couple permanently the deserialized structure with its serialized
support, which is obtained by putting it in a [`MemCase`](`deser::MemCase`). A [`MemCase`](`deser::MemCase`) will
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

Let us start with the simplest case: data that can be zero-copy deserialized. In this case,
we serialize an array of a thousand zeros, and get back a reference to such
an array:

```rust
use epserde::prelude::*;

let s = [0_usize; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized0");
s.serialize(&mut std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: &[usize; 1000] =
    <[usize; 1000]>::deserialize_eps(b.as_ref()).unwrap();

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: [usize; 1000] = 
    <[usize; 1000]>::deserialize_full(
        &mut std::fs::File::open(&file).unwrap()
    ).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<&[usize; 1000]> = 
    <[usize; 1000]>::mmap(&file, Flags::empty()).unwrap();
assert_eq!(s, **u);
```

Note how we serialize an array, but we deserialize a reference.
The reference points inside `b`, so there is
no copy performed. The second call creates a new array instead.
The third call maps the data structure into memory and returns
a [`MemCase`](`deser::MemCase`) that can be used transparently as a reference to the array;
moreover, the [`MemCase`](`deser::MemCase`) can be passed to other functions or stored
in a structure field, as it contains both the structure and the
memory-mapped region that supports it.

## Examples: ε-copy of standard structures

Zero-copy deserialization is not that interesting because it can be applied only to
data whose memory layout and size are fixed and known at compile time.
This time, let us serialize a `Vec` containing a thousand zeros: ε-serde will deserialize its associated
deserialization type, which is a reference to a slice.

```rust
use epserde::prelude::*;

let s = vec![0; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized1");
s.serialize(&mut std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: &[usize] =
    <Vec<usize>>::deserialize_eps(b.as_ref()).unwrap();

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<usize> = 
    <Vec<usize>>::load_full(&file).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<&[usize]> = 
    <Vec<usize>>::mmap(&file, Flags::empty()).unwrap();
assert_eq!(s, **u);
```

Note how we serialize a vector, but we deserialize a reference
to a slice; the same would happen when serializing a boxed slice.
The reference points inside `b`, so there is very little
copy performed (in fact, just a field containing the length of the slice).
All this is because `usize` is a zero-copy type.
Note also that we use the convenience method [`Deserialize::load_full`](`deser::Deserialize::load_full`).

If your code must work both with the original and the deserialized
version, however, it must be written for a trait that is implemented
by both types, such as `AsRef<[usize]>`.

## Example: Zero-copy structures

You can define your types to be zero-copy, in which case they will
work like `usize` in the previous examples. This requires the structure
to be made of zero-copy fields, and to be annotated with `#[zero_copy]`
and `#[repr(C)]` (which means that you will lose the possibility that
the compiler reorders the fields to optimize memory usage):

```rust
use epserde::prelude::*;
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
s.serialize(&mut std::fs::File::create(&file).unwrap()).unwrap();
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: &[Data] =
    <Vec<Data>>::deserialize_eps(b.as_ref()).unwrap();

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<Data> = 
    <Vec<Data>>::load_full(&file).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<&[Data]> = 
    <Vec<Data>>::mmap(&file, Flags::empty()).unwrap();
assert_eq!(s, **u);
```

If a structure is not zero-copy, vectors of structures will be always
deserialized into vectors.

## Example: Structures with parameters

More flexibility can be obtained by defining structures with fields
whose types are defined by parameters. In this case, ε-serde
will deserialize the structure replacing its type parameters with
the associated deserialized type.

Let us design a structure that will contain an integer,
which will be copied, and a vector of integers that we want to ε-copy:

```rust
use epserde::prelude::*;
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
s.store(&file);
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t will be inferred--it is shown here only for clarity
let t: MyStruct<&[isize]> = 
    <MyStruct<Vec<isize>>>::deserialize_eps(b.as_ref()).unwrap();

assert_eq!(s.id, t.id);
assert_eq!(s.data, Vec::from(t.data));

// This is a traditional deserialization instead
let t: MyStruct<Vec<isize>> = 
    <MyStruct::<Vec<isize>>>::load_full(&file).unwrap();
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<MyStruct<&[isize]>> = 
    <MyStruct::<Vec<isize>>>::mmap(&file, Flags::empty()).unwrap();
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
use epserde::prelude::*;
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
s.store(&file);
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();
let t = MyStruct::deserialize_eps(b.as_ref()).unwrap();
// We can call the method on both structures
assert_eq!(s.sum(), t.sum());

let t = <MyStruct>::mmap(&file, Flags::empty()).unwrap();

// t works transparently as a MyStructParam<&[isize]>
assert_eq!(s.id, t.id);
assert_eq!(s.data, t.data.as_ref());
assert_eq!(s.sum(), t.sum());
```

## Example: Deep-copy structures with internal parameters

Internal parameters, that is, parameters used by the
types of your fields but that do not represent the type
of your fields are left untouched. However, to be serializable
they must be classified as deep-copy or zero-copy, and must have
a `'static` lifetime. For example,

```rust
use epserde::prelude::*;
use epserde_derive::*;

#[derive(Epserde, Debug, PartialEq)]
struct MyStruct<A: DeepCopy + 'static>(Vec<A>);

// Create a structure where A is a Vec<isize>
let s: MyStruct<Vec<isize>> = MyStruct(vec![vec![0, 1, 2, 3]]);
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized4");
s.store(&file);
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t is unchanged
let t: MyStruct<Vec<isize>> = 
    <MyStruct<Vec<isize>>>::deserialize_eps(b.as_ref()).unwrap();
```

Note how the field originally of type `Vec<Vec<isize>>` remains of
the same type.

## Example: Zero-copy structures with parameters

For zero-copy structure, things are slightly different because types are not
substituted, even if they represent the type of your fields.
So all parameters must be zero-copy and have a `'static` lifetime.
 For example,

```rust
use epserde::prelude::*;
use epserde_derive::*;

#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
#[repr(C)]
#[zero_copy]
struct MyStruct<A: ZeroCopy + 'static> {
    data: A,
}

// Create a structure where A is a Vec<isize>
let s: MyStruct<i32> = MyStruct { data: 0 };
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized5");
s.store(&file);
// Load the serialized form in a buffer
let b = std::fs::read(&file).unwrap();

// The type of t is unchanged
let t: &MyStruct<i32> = 
    <MyStruct<i32>>::deserialize_eps(b.as_ref()).unwrap();
```

Note how the field originally of type `i32` remains of the same type.

## Example: Enums

Enums are supported, but there are two caveats: first, if you want them to be zero-copy,
they must be `repr(C)`, and thus you will
lose the possibility that the compiler optimize their memory representation;
second, if you have type parameters that are not used by all variants you must be careful
to specify always the same type parameter when serializing and deserializing. This is
obvious for non-enum types, but with enum types with default type parameters it can
become tricky. For example,

```rust
use epserde::prelude::*;
use epserde_derive::*;

#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
enum Enum<T=Vec<usize>> {
    A,
    B(T),
}

// This enum has T=Vec<i32> by type inference
let e = Enum::B(vec![0, 1, 2, 3]);
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized6");
e.store(&file);
// Deserializing using just Enum will fail, as the type parameter 
// by default is Vec<usize>
assert!(<Enum>::load_full(&file).is_err());
```

## Example: `sux-rs`

The [`sux-rs`](http://crates.io/crates/sux-rs/) crate provides several data structures
that use ε-serde.

## Design

Every type serializable with ε-serde has two features that are in principle orthogonal,
but that in practice often condition one another:

- the type has an *associated deserialization type*, which is the type you obtain
upon deserialization;
- the type can be either [`ZeroCopy`](traits::ZeroCopy) or [`DeepCopy`](traits::DeepCopy);
  it can also be neither.

There is no constraint on the associated deserialization type: it can be literally
anything. In general, however, one tries to have a deserialization type that is somewhat
compatible with the original type, in the sense that they both satisfy a trait for which
implementations can be written: for example, ε-serde deserializes vectors as
references to slices, so implementations can be written for references to slices and
will work both on the original and the deserialized type.
And, in general, [`ZeroCopy`](traits::ZeroCopy) types deserialize into themselves.

Being [`ZeroCopy`](traits::ZeroCopy) or [`DeepCopy`](traits::DeepCopy) decides
instead how the type will be treated
when serializing and deserializing sequences, such as arrays, slices, boxed slices, and vectors.
Sequences of zero-copy types are deserialized using a reference, whereas sequences
of deep-copy types are recursively deserialized in allocated memory (to sequences of the
associated deserialization types). It is important to remark
that *you cannot serialize a sequence whose elements are of
a type that is neither* [`ZeroCopy`](traits::ZeroCopy) *nor* [`DeepCopy`](traits::DeepCopy)
(see the [`CopyType`](`traits::CopyType`) documentation for a deeper explanation).

Logically, zero-copy types should be deserialized to references, and this indeed happens
in most cases, and certainly in the derived code: however, *primitive types are always
fully deserialized*. There are two reasons behind this non-orthogonal choice:

- primitive types occupy so little space that deserializing them as a reference is
not efficient;
- if a type parameter `T` is a primitive type, writing generic code for `AsRef<T>` is
really not nice;
- deserializing primitive types to a reference would require further padding to
align them.

Since this is true only of primitive types, when deserializing a
1-tuple containing a primitive type one obtains a reference (and indeed this
workaround can be used if you really need to deserialize a primitive type as a reference).
The same happens if you deserialize a zero-copy
struct containing a single field of primitive type.

Deep-copy types instead are serialized and deserialized recursively, field by field.
The basic idea in ε-serde is that *if a field has a type that is a parameter, during
ε-copy deserialization the type will be replaced with its deserialization type*. Since
the deserialization type is defined recursively, replacement can happen at any depth level. For example,
a field of type `A = Vec<Vec<Vec<usize>>>` will be deserialized as a `A = Vec<Vec<&[usize]>>`.

This approach makes it possible to write ε-serde-aware structures that hide from
the user the substitution. A good example is the `BitFieldVec` structure from
[`sux-rs`](http://crates.io/sux/), which exposes an array of fields of fixed bit
width using (usually) a `Vec<usize>` as a backend; except for extension methods,
all methods of `BitFieldVec` come from the trait `BitFieldSlice`. If you have
your own struct and one of the fields is of type `A`, when serializing your
struct with `A` equal to `BitFieldVec<Vec<usize>>`, upon ε-copy deserialization
you will get a version of your struct with `BitFieldVec<&[usize]>`. All this
will happen under the hood because `BitFieldVec` is ε-serde-aware, and in fact
you will not even notice the difference if you access both versions using the
trait `BitFieldSlice`.

# Derived and hand-made implementation

We strongly suggest using the procedural macro [`Epserde`](`epserde_derive::Epserde`)
to make your own types serializable and deserializable. Just invoking the macro
on your structure will make it fully functional with ε-serde. The attribute
`#[zero_copy]` can be used to make a structure zero-copy, albeit it must satisfy
[a few prerequisites](traits::CopyType).

You can also implement manually
the traits [`CopyType`](traits::CopyType), [`MaxSizeOf`](traits::MaxSizeOf), [`TypeHash`](traits::TypeHash), [`ReprHash`](traits::ReprHash),
[`SerializeInner`](`ser::SerializeInner`), and [`DeserializeInner`](`deser::DeserializeInner`), but
the process is error-prone, and you must be fully aware of ε-serde's conventions. The procedural macro
[`TypeInfo`](`epserde_derive::TypeInfo`) can be used to generate automatically at least
[`MaxSizeOf`](traits::MaxSizeOf), [`TypeHash`](traits::TypeHash), and [`ReprHash`](traits::ReprHash) automatically.

# Acknowledgments

This software has been partially supported by project SERICS (PE00000014) under the NRRP MUR program funded by the EU - NGEU,
and by project ANR COREGRAPHIE, grant ANR-20-CE23-0002 of the French Agence Nationale de la Recherche.
