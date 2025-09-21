# ε-serde

[![downloads](https://img.shields.io/crates/d/epserde)](https://crates.io/crates/epserde)
[![dependents](https://img.shields.io/librariesio/dependents/cargo/epserde)](https://crates.io/crates/epserde/reverse_dependencies)
![GitHub CI](https://github.com/vigna/epserde-rs/actions/workflows/rust.yml/badge.svg)
![license](https://img.shields.io/crates/l/epserde)
[![](https://tokei.rs/b1/github/vigna/epserde-rs?type=Rust,Python)](https://github.com/vigna/epserde-rs)
[![Latest version](https://img.shields.io/crates/v/epserde.svg)](https://crates.io/crates/epserde)
[![Documentation](https://docs.rs/epserde/badge.svg)](https://docs.rs/epserde)

ε-serde is a Rust framework for *ε*-copy *ser*ialization and *de*serialization.

## Why

Large immutable data structures need time to be deserialized using the [serde]
approach. A possible solution for this problem is given by frameworks such as
[Abomonation], [rkiv], and [zerovec], which provide *zero-copy* deserialization:
the stream of bytes serializing the data structure can be used directly as a
Rust structure. In particular, this approach makes it possible to map into
memory an on-disk data structure, making it available instantly. It also makes
it possible to load the data structure in a memory region with particular
attributes, such as transparent huge pages on Linux. Even when using standard
memory load and deserialization happen much faster as the entire structure can
be loaded with a single read operation.

ε-serde has the same goals as the zero-copy frameworks above but provides
different tradeoffs.

## How

Since in these data structures typically most of the data is given by large
chunks of memory in the form of slices or vectors, at deserialization time one
can build quickly a proper Rust structure whose referenced memory, however, is
not copied. We call this approach *ε-copy deserialization*, as typically a
minuscule fraction of the serialized data is copied to build the structure. The
result is similar to that of the frameworks above, but the performance of the
deserialized structure will be identical to that of a standard, in-memory Rust
structure, as references are resolved at deserialization time.

We provide procedural macros implementing serialization and deserialization
methods, basic (de)serialization for primitive types, vectors, etc., convenience
memory-mapping methods based on [mmap_rs], and a [`MemCase`] structure that
couples a deserialized structure with its backend (e.g., a slice of memory or a
memory-mapped region). A [`MemCase`] provides a [`uncase`] method that yields
references to the deserialized instance it contains. Moreover, with proper trait
delegation a [`MemCase`] can be used almost transparently as said instance.

## Who

Tommaso Fontana, while working at INRIA under the supervision of Stefano
Zacchiroli, came up with the basic idea for ε-serde, that is, replacing
structures with equivalent references. The code was developed jointly with
Sebastiano Vigna, who came up with the [`MemCase`] and the
[`ZeroCopy`]/[`DeepCopy`] logic. Valentin Lorentz joined later, providing
major improvements in soundness.

## Cons

These are the main limitations you should be aware of before choosing to use
ε-serde:

- Your types cannot contain references. For example, you cannot use ε-serde on a
  tree.

- While we provide procedural macros that implement serialization and
  deserialization, they require that your type is written and used in a specific
  way for ε-copy deserialization to work properly; in particular, the fields you
  want to ε-copy must be type parameters implementing [`DeserializeInner`], to
  which a [deserialized type] is associated. For example, we provide
  implementations for `Vec<T>`/`Box<[T]>`, where `T` is zero-copy, or
  `String`/`Box<str>`, which have associated deserialized type `&[T]` or `&str`,
  respectively. Vectors and boxed slices of types that are not zero-copy will be
  deserialized recursively in memory instead.

- After deserialization of an instance of type `T`, you will obtain an instance
  of an associated deserialized type [`DeserType<'_,T>`], which will usually
  reference the underlying serialized support (e.g., a memory-mapped region);
  hence the need for a lifetime. If you need to store the deserialized instance
  in a field of a new structure you will need to couple permanently the instance
  with its serialization support, which is obtained by putting it in a
  [`MemCase`] using the convenience methods [`Deserialize::load_mem`],
  [`Deserialize::read_mem`], [`Deserialize::load_mmap`],
  [`Deserialize::read_mmap`], and [`Deserialize::mmap`].

- No validation or padding cleaning is performed on zero-copy types. If you plan
  to serialize data and distribute it, you must take care of these issues.

## Pros

- Almost instant deserialization with minimal allocation provided that you
  designed your type following the ε-serde guidelines or that you use standard
  types.

- The instance you get by deserialization has essentially the same type
  as the one you serialized, except that type parameters will be replaced by
  their associated deserialization type (e.g., vectors will become references to
  slices). This is not the case with [rkiv], which requires you to reimplement
  all methods on a new, different deserialized type.

- The structure you get by deserialization has exactly the same performance as
  the structure you serialized. This is not the case with [zerovec] or [rkiv].

- You can serialize structures containing references to slices, or even
  exact-size iterators, and they will be deserialized as if you had written a
  vector. It is thus possible to serialize structures larger than the available
  memory.

- You can deserialize from read-only supports, as all dynamic information
  generated at deserialization time is stored in newly allocated memory. This is
  not the case with [Abomonation].

## Example: Zero-copy of standard types

Let us start with the simplest case: data that can be zero-copy deserialized. In
this case, we serialize an array of a thousand zeros, and get back a reference
to such an array:

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let s = [0_usize; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized0");
unsafe { s.serialize(&mut std::fs::File::create(&file)?)? };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t will be inferred--it is shown here only for clarity
let t: &[usize; 1000] =
    unsafe { <[usize; 1000]>::deserialize_eps(b.as_ref())? };

assert_eq!(s, *t);

// You can derive the deserialization type, with a lifetime depending on b
let t: DeserType<'_, [usize; 1000]> =
    unsafe { <[usize; 1000]>::deserialize_eps(b.as_ref())? };

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: [usize; 1000] =
    unsafe { <[usize; 1000]>::deserialize_full(
        &mut std::fs::File::open(&file)?
    )? };
assert_eq!(s, t);

// In this case we map the data structure into memory
//
// Note: requires the `mmap` feature.
let u: MemCase<[usize; 1000]> =
    unsafe { <[usize; 1000]>::mmap(&file, Flags::empty())? };

assert_eq!(s, **u.uncase());
#     Ok(())
# }
```

Note how we serialize an array, but we deserialize a reference. The reference
points inside `b`, so there is no copy performed. The call to
[`deserialize_full`] creates a new array instead. The third call maps the data
structure into memory and returns a [`MemCase`] that can be used to get
a reference to the array; moreover, the [`MemCase`] can be passed to other
functions or stored in a structure field, as it contains both the structure and
the memory-mapped region that supports it.

The type alias [`DeserType`] can be used to derive the deserialized type
associated with a type. It contains a lifetime, which is the lifetime of the
memory region containing the serialized data. When deserializing into a
[`MemCase`], however, the lifetime is `'static`, as [`MemCase`] is an owned
type.

## Examples: ε-copy of standard structures

Zero-copy deserialization is not that interesting because it can be applied only
to data whose memory layout and size are fixed and known at compile time. This
time, let us serialize a `Vec` containing a thousand zeros: ε-serde will
deserialize its associated deserialization type, which is a reference to a
slice.

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let s = vec![0; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized1");
unsafe { s.serialize(&mut std::fs::File::create(&file)?)? };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t will be inferred--it is shown here only for clarity
let t: DeserType<'_, Vec<usize>> =
    unsafe { <Vec<usize>>::deserialize_eps(b.as_ref())? };

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<usize> =
    unsafe { <Vec<usize>>::load_full(&file)? };
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<Vec<usize>> =
    unsafe { <Vec<usize>>::mmap(&file, Flags::empty())? };
assert_eq!(s, **u.uncase());

// You can even deserialize directly into a boxed slice
// as they are interchangeable with vectors
let t: Box<[usize]> =
    unsafe { <Box<[usize]>>::load_full(&file)? };
assert_eq!(s.as_slice(), &*t);
#     Ok(())
# }
```

Note how we serialize a vector, but we deserialize a reference to a slice; the
same would happen when serializing a boxed slice; in fact, vectors and boxed
slices are interchangeable. The reference points inside `b`, so there is very
little copy performed (in fact, just a field containing the length of the
slice). All this is because `usize` is a zero-copy type. Note also that we use
the convenience method [`Deserialize::load_full`].

If your code must work both with the original and the deserialized version,
however, it must be written for a trait that is implemented by both types, such
as `AsRef<[usize]>`.

## Example: Zero-copy structures

You can define your types to be zero-copy, in which case they will work like
`usize` in the previous examples. This requires the structure to be made of
zero-copy fields, and to be annotated with `#[zero_copy]` and `#[repr(C)]`
(which means that you will lose the possibility that the compiler reorders the
fields to optimize memory usage):

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
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
unsafe { s.serialize(&mut std::fs::File::create(&file)?)? };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t will be inferred--it is shown here only for clarity
let t: DeserType<'_, Vec<Data>> =
    unsafe { <Vec<Data>>::deserialize_eps(b.as_ref())? };

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<Data> =
    unsafe { <Vec<Data>>::load_full(&file)? };
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<Vec<Data>> =
    unsafe { <Vec<Data>>::mmap(&file, Flags::empty())? };
assert_eq!(s, **u.uncase());
#     Ok(())
# }
```

If a type is not zero-copy, instead, vectors/boxed slices will be always
deserialized into vectors/boxed slices.

## Example: Structures with parameters

More flexibility can be obtained by defining types with fields whose types are
defined by parameters. In this case, ε-serde will deserialize instances of the
type replacing its type parameters with the associated deserialized type.

Let us design a structure that will contain an integer, which will be copied,
and a vector of integers that we want to ε-copy:

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
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
unsafe { s.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t will be inferred--it is shown here only for clarity
let t: MyStruct<&[isize]> =
    unsafe { <MyStruct<Vec<isize>>>::deserialize_eps(b.as_ref())? };

assert_eq!(s.id, t.id);
assert_eq!(s.data, Vec::from(t.data));

// This is a traditional deserialization instead
let t: MyStruct<Vec<isize>> =
    unsafe { <MyStruct<Vec<isize>>>::load_full(&file)? };
assert_eq!(s, t);

// In this case we map the data structure into memory
let u: MemCase<MyStruct<Vec<isize>>> =
    unsafe { <MyStruct<Vec<isize>>>::mmap(&file, Flags::empty())? };
let u: &MyStruct<&[isize]> = u.uncase();
assert_eq!(s.id, u.id);
assert_eq!(s.data, u.data.as_ref());
#     Ok(())
# }
```

Note how the field originally containing a `Vec<isize>` now contains a
`&[isize]` (this replacement is generated automatically). The reference points
inside `b`, so there is no need to copy the field. Nonetheless, deserialization
creates a new structure `MyStruct`, ε-copying the original data. The second call
creates a full copy instead. We can write methods for our structure that will
work for the ε-copied version: we just have to take care that they are defined
in a way that will work both on the original type parameter and on its
associated deserialized type; we can also use `type` to reduce the clutter:

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
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
unsafe { s.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;
let t = unsafe { MyStruct::deserialize_eps(b.as_ref())? };
// We can call the method on both structures
assert_eq!(s.sum(), t.sum());

let t = unsafe { <MyStruct>::mmap(&file, Flags::empty())? };
let t: &MyStructParam<&[isize]> = t.uncase();

// t works transparently as a &MyStructParam<&[isize]>
assert_eq!(s.id, t.id);
assert_eq!(s.data, t.data.as_ref());
assert_eq!(s.sum(), t.sum());
#     Ok(())
# }
```

It is important to note that since the derive code replaces type parameters that
are types of fields with their associated (de)serialization type when
generating the (de)serialization type of your structure, you cannot have a type
parameter that appears both as the type of a field and as a type parameter of
another field. For example, the following code will not compile:

```compile_fail
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq)]
struct MyStructParam<A> {
    id: isize,
    data: A,
    vec: Vec<A>
}
#     Ok(())
# }
```

The result will be an error message similar to the following:
```text
|
| #[derive(Epserde, Debug, PartialEq)]
|          ^^^^^^^ expected `Vec<<A as DeserializeInner>::DeserType<'_>>`, found `Vec<A>`
| struct MyStructParam<A> {
|                      - found this type parameter
```

## Example: Deep-copy structures with internal parameters

Internal type parameters, that is, type parameters used by the types of your
fields but that do not represent the type of a fields, are left untouched.
However, to be serializable they must be classified as deep-copy or zero-copy,
and in the first case they must have a `'static` lifetime. The only exception to
this rule is for types inside a [`PhantomData`], which do not even need to be
serializable. For example,

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq)]
struct MyStruct<A: DeepCopy + 'static>(Vec<A>);

// Create a structure where A is a Vec<isize>
let s: MyStruct<Vec<isize>> = MyStruct(vec![vec![0, 1, 2, 3]]);
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized5");
unsafe { s.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t is unchanged
let t: MyStruct<Vec<isize>> =
    unsafe { <MyStruct<Vec<isize>>>::deserialize_eps(b.as_ref())? };
#     Ok(())
# }
```

Note how the field originally of type `Vec<Vec<isize>>` remains of the same
type.

## Example: Zero-copy structures with parameters

For zero-copy types, things are slightly different because type parameters are
not substituted, even if they are the type of a field. So all type parameters
must be zero-copy. This must hold even for types inside a [`PhantomData`]. For
example,

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
#[repr(C)]
#[zero_copy]
struct MyStruct<A: ZeroCopy> {
    data: A,
}

// Create a structure where A is a Vec<isize>
let s: MyStruct<i32> = MyStruct { data: 0 };
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized6");
unsafe { s.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t is unchanged
let t: &MyStruct<i32> =
    unsafe { <MyStruct<i32>>::deserialize_eps(b.as_ref())? };
#     Ok(())
# }
```

Note how the field originally of type `i32` remains of the same type.

## Example: Enums

Enums are supported, but there are two caveats: first, if you want them to be
zero-copy, they must be `repr(C)`, and thus you will lose the possibility that
the compiler optimizes their memory representation; second, if you have type
parameters that are not used by all variants you must be careful to specify
always the same type parameter when serializing and deserializing. This is
obvious for non-enum types, but with enum types with default type parameters it
can become tricky. For example,

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
enum Enum<T=Vec<usize>> {
    A,
    B(T),
}

// This enum has T=Vec<i32> by type inference
let e = Enum::B(vec![0, 1, 2, 3]);
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized7");
unsafe { e.store(&file) };
// Deserializing using just Enum will fail, as the type parameter
// by default is Vec<usize>
assert!(unsafe { <Enum>::load_full(&file) }.is_err());
#     Ok(())
# }
```

## Example: (Structures containing references to) slices

For convenience, ε-serde can serialize references to slices, and will
deserialize them as if they were vectors/boxed slices. You must however be careful to
deserialize with the correct type. For example,

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let v = vec![0, 1, 2, 3];
// This is a slice
let s: &[i32] = v.as_ref();
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized8");
unsafe { s.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// We must deserialize as a vector, even if we are getting back a reference
let t: &[i32] = unsafe { <Vec<i32>>::deserialize_eps(b.as_ref())? };
let t: Vec<i32> = unsafe { <Vec<i32>>::deserialize_full(
        &mut std::fs::File::open(&file)?
    )? };
let t: MemCase<Vec<i32>> = unsafe { <Vec<i32>>::mmap(&file, Flags::empty())? };

// Or as a boxed slice
let t: &[i32] = unsafe { <Box<[i32]>>::deserialize_eps(b.as_ref())? };

// Within a structure
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    s: A,
}

let d = Data { s };
// Serialize it
unsafe { d.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// We must deserialize the field as a vector, even if we are getting back a reference
let t: Data<&[i32]> = unsafe { <Data<Vec<i32>>>::deserialize_eps(b.as_ref())? };
let t: Data<Vec<i32>> = unsafe { <Data<Vec<i32>>>::deserialize_full(
        &mut std::fs::File::open(&file)?
    )? };
let t: MemCase<Data<Vec<i32>>> = unsafe { <Data<Vec<i32>>>::mmap(&file, Flags::empty())? };

// Or as a boxed slice
let t: Data<&[i32]> = unsafe { <Data<Box<[i32]>>>::deserialize_eps(b.as_ref())? };

# Ok(())
# }
```

## Example: (Structures containing) iterators

ε-serde can serialize iterators returning references to a type. The resulting
field can be deserialized as a vector/boxed slice. In this case we need to wrap
the iterator in a [`SerIter`], as ε-serde cannot implement the serialization
traits directly on [`Iterator`]. For example,

```rust
# use epserde::prelude::*;
# use std::slice::Iter;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let v = vec![0, 1, 2, 3];
// This is an iterator
let i: Iter<'_, i32> = v.iter();
// Serialize it by wrapping it in a SerIter
let mut file = std::env::temp_dir();
file.push("serialized9");
unsafe { SerIter::from(i).store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// We must deserialize as a vector, even if we are getting back a reference
let t: &[i32] = unsafe { <Vec<i32>>::deserialize_eps(b.as_ref())? };
let t: Vec<i32> = unsafe { <Vec<i32>>::deserialize_full(
        &mut std::fs::File::open(&file)?
    )? };
let t: MemCase<Vec<i32>> = unsafe { <Vec<i32>>::mmap(&file, Flags::empty())? };

// Or as a boxed slice
let t: &[i32] = unsafe { <Box<[i32]>>::deserialize_eps(b.as_ref())? };

// Within a structure
#[derive(Epserde, Debug, PartialEq, Eq, Default, Clone)]
struct Data<A> {
    s: A,
}

let d = Data { s: SerIter::from(v.iter()) };
// Serialize it
unsafe { d.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// We must deserialize the field as a vector, even if we are getting back a reference
let t: Data<&[i32]> = unsafe { <Data<Vec<i32>>>::deserialize_eps(b.as_ref())? };
let t: Data<Vec<i32>> = unsafe { <Data<Vec<i32>>>::deserialize_full(
        &mut std::fs::File::open(&file)?
    )? };
let t: MemCase<Data<Vec<i32>>> = unsafe { <Data<Vec<i32>>>::mmap(&file, Flags::empty())? };

// Or as a boxed slice
let t: Data<&[i32]> = unsafe { <Data<Box<[i32]>>>::deserialize_eps(b.as_ref())? };
# Ok(())
# }
```

## Example: `sux-rs`

The [`sux-rs`] crate provides several data structures that use ε-serde.

## References and Smart Pointers

You can serialize using just a (mutable) reference. Moreover, smart pointers
such as [`Box`], [`Rc`], and [`Arc`] are supported by _erasure_—they
dynamically removed from the type and dynamically reinstated at deserialization.
Please see the documentation of the [`pointer`] module for more details.

## Vectors and Boxed slices

Vectors and boxed slices are entirely interchangeable in ε-serde. In
particular, you can serialize a vector and deserialize it as a boxed slice, or
vice versa, even when they are fields of a structure (given, of course, that they
are the concrete type of a type parameter).

## [`PhantomData`]

[`PhantomData`] undergoes a special treatment: its type parameter `T` does not
have to be (de)serializable or sized—it is sufficient that it implements
[`TypeHash`].

There might be corner cases in which `T` appears both as a parameter of
[`PhantomData`] and as a type parameter of a field of a type. In this case, you
can use [`PhantomDeserData`] instead of [`PhantomData`].

## MemDbg / MemSize

All ε-serde structures implement the [`MemDbg`] and [`MemSize`] traits.

## Design

Every type serializable with ε-serde has three features that are in principle
orthogonal, but that in practice often condition one another:

- the type has an *associated serialization type*, which is the type we
  write when serializing;
- the type has an *associated deserialization type*, which is the type you
  obtain upon deserialization;
- the type can be either [`ZeroCopy`] or [`DeepCopy`]; it can also be neither.

There is no constraint on the associated (de)serialization type: it can be
literally anything. In general, however, one tries to have a deserialization
type that is somewhat compatible with the original type, in the sense that they
both satisfy a trait for which implementations can be written: for example,
ε-copy deserialization turns vectors/boxed slices into references to slices, so
implementations can be written for `AsRef<[·]>` and will work both on the
original and the deserialized instance. And, in general, [`ZeroCopy`] types
deserialize into themselves. Presently the associated serialization type is
almost always `Self`, with the notable exception of references to slices and
iterators, which are serialized for convenience as vectors/boxed slices.

Being [`ZeroCopy`] or [`DeepCopy`] decides instead how the type will be treated
when serializing and deserializing sequences, such as arrays, slices, boxed
slices, and vectors. Sequences of zero-copy types are ε-copy deserialized using
a reference, whereas sequences of deep-copy types are always recursively
deserialized in allocated memory (to sequences of the associated deserialization
types). It is important to remark that *you cannot serialize a sequence whose
elements are of a type that is neither* [`ZeroCopy`] *nor* [`DeepCopy`] (see the
[`CopyType`] documentation for a deeper explanation).

Logically, zero-copy types should be deserialized to references, and this indeed
happens in most cases, and certainly in the derived code: however, *primitive
types are always fully deserialized*. There are two reasons behind this
non-orthogonal choice:

- primitive types occupy so little space that deserializing them as a reference
  is not efficient;
- if a type parameter `T` is a primitive type, writing generic code for
  `AsRef<T>` is really not nice;
- deserializing primitive types to a reference would require further padding to
  align them.

Since this is true only of primitive types, when deserializing a 1-tuple
containing a primitive type one obtains a reference (and indeed this workaround
can be used if you really need to deserialize a primitive type as a reference).
The same happens if you deserialize a zero-copy struct containing a single field
of primitive type.

Instances of deep-copy types instead are serialized and deserialized
recursively, field by field. The basic idea in ε-serde is that *if the type of a
field is a type parameter, during ε-copy deserialization the type will be
replaced with its deserialization type*. Since the deserialization type is
defined recursively, replacement can happen at any depth level. For example, a
field of type `A = Vec<Vec<Vec<usize>>>` will be deserialized as a `A =
Vec<Vec<&[usize]>>`.

Note, however, that field types are not replaced if they are not type
parameters: a field of type `Vec<T>` will always be deserialized as a `Vec<T>`,
even if `T` is [`ZeroCopy`]. In particular, you cannot have `T` both as the type
of a field and as a type parameter of another field (but see the exception below
for [`PhantomData`]).

This approach makes it possible to write ε-serde-aware structures that hide from
the user the substitution. A good example is the [`BitFieldVec`] structure from
[`sux`], which exposes an array of fields of fixed bit width using (usually)
a `Vec<usize>` as a backend; except for extension methods, all methods of
[`BitFieldVec`] come from the trait [`BitFieldSlice`]. If you have your own struct
and one of the fields is of type `A`, when serializing your struct with `A`
equal to `BitFieldVec<Vec<usize>>`, upon ε-copy deserialization you will get a
version of your struct with `BitFieldVec<&[usize]>`. All this will happen under
the hood because [`BitFieldVec`] is ε-serde-aware, and in fact you will not even
notice the difference if you access both versions using the trait
[`BitFieldSlice`].

## Specification

It this section we describe in a somewhat formal way the specification of 
ε-serde. We suggest to get acquainted with the examples before reading it. 

An ε-serde serialization process involves two types: 

* `S`, the _serializable type_, which must implement SerializeInner (and thus Serializable), which in turn requires the implementation....

* Its _serialization type_ S:SerType.

In general the serialization type of S is S, but there is some normalization and erasure involved (e.g., vectors become boxed slices, and some smart pointers such as Rc are erased). Moreover, for convenience a few types that are not really serializable have a convenience serialization type (e.g., iterators become boxed slices). The derivation of the serialization type will be detailed later. 

When you invoke serialize on an instance of S, ε-serde
writes a type hash which is derived from S:SerType, and which represents the definition of the type (copy type, field names, types, etc.), an alignment hash which is derived from the alignment of S:SerType (essentially, recording where padding had been inserted in the zero-copy parts of the type), and then the data contained in the instance.

An ε-serde deserialization process involves instead three types: 

* D, the _deserializable type_, which must implement DeserializeInner (and thus Deserializable), which in turn requires the implementation....

* The _serialization type_ of D, D:SerType.

* The _deserialization type_ D:DeserType.

You must invoke deserialize as a method of 'D', and pass it the bytes obtained by serialing S. Deserialization will happen only if the type hash of D:SerType matches that of S:SerType, and the same must happen for the alignment hash: otherwise, you'll get an error. Now, depending on which type of deserialization you requested (full or ε-copy) you will obtain an instance of D or D:DeserType.



## Derived and hand-made implementations

We strongly suggest using the procedural macro [`Epserde`] to make your own
types serializable and deserializable. Just invoking the macro on your structure
will make it fully functional with ε-serde. The attribute `#[zero_copy]` can be
used to make a structure zero-copy, albeit it must satisfy [a few
prerequisites].

You can also implement manually the traits [`CopyType`], [`MaxSizeOf`],
[`TypeHash`], [`ReprHash`], [`SerializeInner`], and [`DeserializeInner`], but
the process is error-prone, and you must be fully aware of ε-serde's
conventions. The procedural macro [`TypeInfo`] can be used to generate
automatically at least [`CopyType`], [`MaxSizeOf`], [`TypeHash`], and
[`ReprHash`] automatically.

## Acknowledgments

This software has been partially supported by project SERICS (PE00000014) under
the NRRP MUR program funded by the EU - NGEU, and by project ANR COREGRAPHIE,
grant ANR-20-CE23-0002 of the French Agence Nationale de la Recherche.
Views and opinions expressed are however those of the authors only and do not
necessarily reflect those of the European Union or the Italian MUR. Neither the
European Union nor the Italian MUR can be held responsible for them.

[`MemCase`]: <https://docs.rs/epserde/latest/epserde/deser/mem_case/struct.MemCase.html>
[`uncase`]: <https://docs.rs/epserde/latest/epserde/deser/mem_case/struct.MemCase.html#method.uncase>
[`ZeroCopy`]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.ZeroCopy.html>
[`DeepCopy`]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.DeepCopy.html>
[`CopyType`]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html>
[`MaxSizeOf`]: <https://docs.rs/epserde/latest/epserde/traits/type_info/trait.MaxSizeOf.html>
[`TypeHash`]: <https://docs.rs/epserde/latest/epserde/traits/type_info/trait.TypeHash.html>
[`ReprHash`]: <https://docs.rs/epserde/latest/epserde/traits/type_info/trait.ReprHash.html>
[`DeserializeInner`]: <https://docs.rs/epserde/latest/epserde/deser/trait.DeserializeInner.html>
[`SerializeInner`]: <https://docs.rs/epserde/latest/epserde/ser/trait.SerializeInner.html>
[`TypeInfo`]: <https://docs.rs/epserde/latest/epserde/derive.TypeInfo.html>
[`Epserde`]: <https://docs.rs/epserde/latest/epserde_derive/derive.Epserde.html>
[`Deserialize::load_full`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_full>
[`deserialize_full`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#tymethod.deserialize_full>
[`DeserType`]: <https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html>
[`Deserialize::load_mem`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mem>
[`Deserialize::load_mmap`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mmap>
[`Deserialize::read_mem`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mem>
[`Deserialize::read_mmap`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mmap>
[`Deserialize::mmap`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.mmap>
[a few prerequisites]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html>
[deserialized type]: <https://docs.rs/epserde/latest/epserde/deser/trait.DeserializeInner.html#associatedtype.DeserType>
[`DeserType<'_,T>`]: <https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html>
[`sux`]: <http://crates.io/sux/>
[serde]: <https://serde.rs/>
[Abomonation]: <https://crates.io/crates/abomonation>
[rkiv]: <https://crates.io/crates/rkyv/>
[zerovec]: <https://crates.io/crates/zerovec>
[mmap_rs]: <https://crates.io/crates/mmap-rs>
[`MemDbg`]: https://docs.rs/mem_dbg/latest/mem_dbg/trait.MemDbg.html
[`MemSize`]: https://docs.rs/mem_dbg/latest/mem_dbg/trait.MemSize.html
[`PhantomData`]: <https://doc.rust-lang.org/std/marker/struct.PhantomData.html>
[`Iterator`]: <https://doc.rust-lang.org/std/iter/trait.Iterator.html>
[`SerIter`]: <https://docs.rs/epserde/latest/epserde/impls/iter/struct.SerIter.html>
[`PhantomDeserData`]: <https://docs.rs/epserde/latest/epserde/struct.PhantomDeserData.html>
[`Box`]: <https://doc.rust-lang.org/std/boxed/struct.Box.html>
[`Rc`]: <https://doc.rust-lang.org/std/rc/struct.Rc.html>
[`Arc`]: <https://doc.rust-lang.org/std/sync/struct.Arc.html>
[`pointer`]: <https://docs.rs/epserde/latest/epserde/impls/pointer/index.html>
[`BitFieldVec`]: <https://docs.rs/sux/latest/sux/bits/bit_field_vec/struct.BitFieldVec.html>
[`BitFieldSlice`]: <https://docs.rs/sux/latest/sux/traits/bit_field_slice/trait.BitFieldSlice.html>
