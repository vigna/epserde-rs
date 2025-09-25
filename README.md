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
instances with equivalent references. The code was developed jointly with
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
  want to ε-copy must be type parameters implementing [`DeserInner`], to which a
  [deserialized type] is associated. For example, we provide implementations for
  `Vec<T>`/`Box<[T]>`, where `T` is zero-copy, or `String`/`Box<str>`, which
  have associated deserialized type `&[T]` or `&str`, respectively. Vectors and
  boxed slices whose elements are not zero-copy will be deserialized recursively
  in memory instead.

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

- You can serialize instances containing references to slices, or even
  exact-size iterators, and they will be deserialized as if you had written a
  vector. It is thus possible to serialize instances larger than the available
  memory.

- You can deserialize from read-only supports, as all dynamic information
  generated at deserialization time is stored in newly allocated memory. This is
  not the case with [Abomonation].

## Example: Zero-copy of standard types

Let us start with the simplest case: data that can be zero-copy deserialized. In
this case, we serialize an array of a thousand zeros, and get back a reference
to such an array:

```rust
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
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

## Examples: ε-copy of standard types

Zero-copy deserialization is not that interesting because it can be applied only
to data whose memory layout and size are fixed and known at compile time. This
time, let us serialize a `Vec` containing a thousand zeros: ε-serde will
deserialize its associated deserialization type, which is a reference to a
slice.

```rust
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
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

## Example: Zero-copy user-defined structures

You can define your types to be zero-copy, in which case they will work like
`usize` in the previous examples. This requires the structure to be made of
zero-copy fields, and to be annotated with `#[zero_copy]` and `#[repr(C)]`
(which means that you will lose the possibility that the compiler reorders the
fields to optimize memory usage):

```rust
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
```

If a type is not zero-copy, instead, vectors/boxed slices will be always
deserialized into vectors/boxed slices.

If you define a type that satisfies the requirements for being zero-copy, but
has not been annotated with `#[zero_copy]`, ε-serde will print a warning message
each time you serialize an instance of the type. You can use the `#[deep_copy]`
annotation to silence the warning. The reason for this (annoying) message is
that it is not possible to detect at compile time this missed opportunity. In
some cases, however, you might want to have a deep-copy type (e.g., because
field reordering is beneficial for memory usage).

## Example: User-defined structures with parameters

More flexibility can be obtained by defining types with fields whose types are
defined by parameters. In this case, ε-serde will deserialize instances of the
type replacing its type parameters with the associated deserialized type.

Let us design a structure that will contain an integer, which will be copied,
and a vector of integers that we want to ε-copy:

```rust
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
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
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
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
|          ^^^^^^^ expected `Vec<<A as DeserInner>::DeserType<'_>>`, found `Vec<A>`
| struct MyStructParam<A> {
|                      - found this type parameter
```

## Example: User-defined deep-copy structures with internal parameters

Internal type parameters, that is, type parameters used by the types of your
fields but that do not represent the type of a fields, are left untouched.
However, to be serializable they must be classified as deep-copy or zero-copy.
The only exception to this rule is for types inside a [`PhantomData`], which do
not even need to be serializable, or for types inside a [`PhantomDeserData`].
For example,

```rust
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
```

Note how the field originally of type `Vec<Vec<isize>>` remains of the same
type.

## Example: User-defined zero-copy structures with parameters

For zero-copy types, things are slightly different because type parameters are
not substituted, even if they are the type of a field. So all type parameters
must be zero-copy. This must hold even for types inside a [`PhantomData`]. For
example,

```rust
use epserde::prelude::*;

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
    
# Ok::<(), Box<dyn std::error::Error>>(())
```

Note how the field originally of type `i32` remains of the same type.

## Example: User-defined enum types

Enums are supported, but there are two caveats: first, if you want them to be
zero-copy, they must be `repr(C)`, and thus you will lose the possibility that
the compiler optimizes their memory representation; second, if you have type
parameters that are not used by all variants you must be careful to specify
always the same type parameter when serializing and deserializing. This is
obvious for non-enum types, but with enum types with default type parameters it
can become tricky. For example,

```rust
use epserde::prelude::*;

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
```

## Example: (Structures containing references to) slices

For convenience, ε-serde can serialize references to slices, and will
deserialize them as if they were vectors/boxed slices. You must however be careful to
deserialize with the correct type. For example,

```rust
use epserde::prelude::*;

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

# Ok::<(), Box<dyn std::error::Error>>(())
```

## Example: (Structures containing) iterators

ε-serde can serialize exact-size iterators returning references to a type. The
resulting field can be deserialized as a vector/boxed slice. In this case we
need to wrap the iterator in a [`SerIter`], as ε-serde cannot implement the
serialization traits directly on [`Iterator`]. For example,

```rust
use epserde::prelude::*;
use core::slice::Iter;

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

# Ok::<(), Box<dyn std::error::Error>>(())
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
vice versa, even when they are fields of a type (given, of course, that they
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
  obtain when deserialized;
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

Being [`ZeroCopy`] or [`DeepCopy`] decides how the type will be treated upon
deserialization. Instances of zero-copy types are ε-copy deserialized as a
reference, whereas instances of deep-copy types are are always recursively
deserialized in allocated memory.

Sequences of zero-copy types are ε-copy deserialized using a reference to a
slice, whereas sequences of deep-copy types are deserialized in allocated memory
(to sequences of the associated deserialization types). It is important to
remark that *you cannot serialize a sequence whose elements are of a type that
is neither* [`ZeroCopy`] *nor* [`DeepCopy`] (see the [`CopyType`] documentation
for a deeper explanation).

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
The same happens if you deserialize a zero-copy instance containing a single field
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
[`sux`], which exposes an array of fields of fixed bit width using (usually) a
`Vec<usize>` as a backend; except for extension methods, all methods of
[`BitFieldVec`] come from the trait [`BitFieldSlice`]. If you have your own
user-defined type and one of the fields is of type `A`, when serializing an
instance with `A` equal to `BitFieldVec<Vec<usize>>`, upon ε-copy
deserialization you will get a version of your instance of type
`BitFieldVec<&[usize]>`. All this will happen under the hood because
[`BitFieldVec`] is ε-serde-aware, and in fact you will not even notice the
difference if you access both versions using the trait [`BitFieldSlice`].

## Specification

It this section we describe in a somewhat formal way the specification of
ε-serde. We suggest to get acquainted with the examples before reading it.

### Types

Within ε-serde, types are classified as [_deep-copy_] or [_zero-copy_], usually
by implementing the unsafe [`CopyType`] trait with associated type
[`CopyType::Copy`] equal to [`Deep`] or [`Zero`]; types without such an
implementation are considered deep-copy. There is a blanket implementation for
the [`DeepCopy`] marker trait for all types implementing [`CopyType`] with
associated type [`CopyType::Copy`] equal to [`Deep`]. Zero-copy types must also
implement [`Copy`], [`MaxSizeOf`]; they must also outlive `'static` lifetime, and be
`repr(C)`. Under those conditions, there is a similar blanket implementation for
the [`ZeroCopy`] marker trait. Zero-copy types cannot contain any reference, but
this condition cannot be checked by the compiler (`'static` does not prevent,
say, references to string constants), which is why [`CopyType`] is unsafe.

Zero-copy types have the property that their memory representation can
be serialized as a sequence of bytes; a reference to the sequence is then
a valid reference to an instance of the type. This happens because `repr(C)`
guarantees a fixed memory layout, and because the type does not contain any
reference to other data.

Deep-copy types instead are types must can be serialized and deserialized
field by field.

Note that all fields of a type you want to (de)serialize must be of a type
implementing [`CopyType`] for the derive code to work.

### Replaceable and irreplaceable parameters

Given a type `T`with generics, we say that a type parameter is _replaceable_ if
it appears as the type of a field of `T`. We say instead that a type parameter
is _irreplaceable_ if it is a generic parameter of the type of a field of `T`.

The basic assumption in what follows, and in the derived code of ε-serde, is
that no type parameter is both replaceable and irreplaceable. This means that
you cannot have a type parameter that appears both as the type of a field and as
a type parameter of the type of a field. If that happen, you will have to write
the (de)serialization code by hand. For example, in the following structure
```rust
struct Bad<A> {
    data: A,
    vec: Vec<A>,
}
```
the type parameter `A` is both replaceable (it is the type of the field `data`)
and irreplaceable (it is a type parameter of the type of field `vec`).

The only exception to this rule is for type parameters that appear inside a
[`PhantomDeserData`], which must be replaceable.

The fundamental idea at the basis of ε-serde is that replaceable parameters make
it possible to refer to serialized zero-copy data without copying it. For
example, given a structure

```rust
struct Good<A> {
    data: A,
}
```

if `A` is `Vec<Z>` and `Z` is zero-copy, then we can deserialize an instance of
`Good<Vec<Z>>` from in-memory data by replacing `Vec<Z>` with `&[Z]`, and the
resulting structure will have type `Good<&[Z]>`: the slice will refer directly
to the serialized data, and only a small fraction of the structure (a pointer
and an integer) will need to be allocated—hence, the term “ε-copy”.

This replacement happens recursively thanks to Rust's type system. As long as
`impl` sections are written in terms of traits implemented by both the original
and the replaced type, the code will work transparently on both types; in this
case, methods should be written with the bound `A: AsRef<[Z]>`.

### Serialization and deserialization

An ε-serde serialization process involves two types:

* `S`, the _serializable type_, which is the type of the instance you want to
  serialize. It must implement [`SerInner`] (which implies
  [`Serialize`] by a blanket implementation).

* Its associated _serialization type_ [`S::SerType`], which must implement
  [`TypeHash`] and [`ReprHash`].

In general the serialization type of `S` is `S`, but there is some normalization
and erasure involved (e.g., vectors become boxed slices, and some smart pointers
such as [`Rc`] are erased). Moreover, for convenience a few types that are not
really serializable have a convenience serialization type (e.g., iterators
become boxed slices). The derivation of the serialization type will be detailed
later, but the key feature is that when deriving the serialization type of `S`
replaceable type parameters of `S` are replaced with their
serialization type.

When you invoke serialize on an instance of type `S`, ε-serde writes a type hash
which is derived from [`S::SerType`], and which represents the definition of the
type (copy type, field names, types, etc.), an alignment hash which is derived
from the alignment of [`S::SerType`] (essentially, recording where padding had
been inserted in the zero-copy parts of the type), and then recursively the data
contained in the instance.

An ε-serde deserialization process involves instead three types:

* `D`, the _deserializable type_, which must implement [`DeserInner`],
  [`TypeHash`], and [`ReprHash`], so the blanket implementation for
  [`Deserialize`] applies. This is the type on which deserialization
  methods are invoked.

* The associated _serialization type_  [`D::SerType`].

* The associated _deserialization type_ [`D::DeserType<'_>`].

In general `D` is the same as `S`, but the only relevant condition for
deserializing using the deserializable type `D` an instance serialized with
serializable type `S` is that [`D::SerType`] is equal to [`S::SerType`].
This gives some latitude in the choice of the deserializable type—for example, a
boxed array instead of a vector for a replaceable parameter.

The deserialization type, instead, is the main technical novelty of ε-serde: it
is a reference, instead of an instance, for zero-copy types, and a reference to
a slice, rather than owned data, for vectors, boxed slices, and arrays of such
types. For vectors, boxed slices, and arrays of deep-copy types it is obtained
by replacing their type parameter with its deserialization type. For more
complex types, it is obtained by the replacing replaceable parameters with
their deserialization type.

For example:

* `T::DeserType<'_>` is `&T` if `T` is zero-copy, but
  `T` if `T` is deep-copy;

* `<Vec<T>>::DeserType<'_>` is `&[T]` if `T` is zero-copy, but
  `Vec<T>` if `T` is deep-copy;

* `<Good<T>>::DeserType<'_>` is `Good<&T>` if `T` is zero-copy, but
  `Good<T>` if `T` is deep-copy;

* `Good<Vec<T>>::DeserType<'_>` is `Good<&[T]>` if `T` is zero-copy, but
  `Good<Vec<T>>` if `T` is deep-copy;

* `Good<Vec<Vec<T>>>::DeserType<'_>` is `Good<Vec<&[T]>>` if `T` is zero-copy,
  but again `Good<Vec<Vec<T>>>` if `T` is deep-copy.

There are now two types of deserialization:

* [`deserialize_full`] performs _full-copy deserialization_, which reads recursively
  the serialized data from a [`Read`] and builds an instance of `D`. This is
  basically a standard deserialization, except that it is usually much faster if
  you have large sequences of zero-copy types, as they are deserialized in a
  single [`read_exact`].

* [`deserialize_eps`] perform _ε-copy deserialization_, which accesses the
  serialized data as a byte slice, and builds an instance of `D::DeserType<'_>`
  that refers to the data inside the byte slice.

Whichever method you invoke on `D`, deserialization will happen only if the type
hash of [`D::SerType`] matches that of [`S::SerType`], and the same must happen
for the alignment hash: otherwise, you will get an error. Note that the
serialized data does not contain a structural copy of any type: it is the
responsibility of the code invoking the deserialization method to know the type
of the data it is reading.

### Serialization and deserialization types

Given a user-defined type `T`:

- if `T` is zero-copy, the serialization type is `T`, and the deserialization
  type is `&T`;

- if `T` is deep-copy, the (de)serialization type is obtained as follow.
  Assuming `T` is a concrete type obtained by resolving the type parameters
  `P₀`, `P₁`, `P₂`, … of a type definition (struct or enum) to concrete types
  `T₀`, `T₁`, `T₂`, …, then `T:(De)serType` is obtained by resolving each
  replaceable type parameter `Pᵢ` with the concrete type `Tᵢ:(De)serType`
  instead. (Note that the first rule still applies, so if `Tᵢ` is zero-copy
  the serialization type is `Tᵢ` and the deserialization type is `&Tᵢ`.)

We can describe the replacements leading to the deserialization type in a
non-recursive way as follows: consider the syntax tree of the type `D`, in which
the root, labeled by `D`, is connected to the root of the syntax trees of
its fields, and each children is further labeled by the name of the field.
Replacement happens in two cases:

* There is a path starting at the root, traversing only fields whose type is a
  replaceable parameter, and ending at node that is a vector/boxed slice/array
  whose elements are zero-copy: it will be replaced with a reference to a
  slice.

* This is a _shortest_ path starting at the root, traversing only fields whose type is a
  replaceable parameter, and ending at a node that is zero-copy: it will be
  replaced with a reference to the same type.

Note that shortest-path condition: this is necessary because when you reach a
zero-copy type the recursion in the definition of the deserialization type
stops. Note also that if `D` is zero-copy the empty path satisfies the
second condition, and indeed `D::DeserType<'_>` is `&D`.

  For standard types and [`PhantomDeserData`], we have:

* all primitive types, such as `u8`, `i32`, `f64`, `char`, `bool`, etc., `()`,
  and `PhantomData<T>` are zero-copy and their (de)serialization type is
  themselves;

* `Option<T>` and `PhantomDeserData<T>` are deep-copy and their
  (de)serialization type is themselves, with `T` replaced by its
  (de)serialization type;

* `Vec<T>`, `Box<[T]>`, `&[T]` and `SerIter<T>` are deep-copy, and their
  serialization type is `Box<[T]>` if `T` is zero-copy, but `Box<[T::SerType]>`
  if `T` is deep-copy; the deserialization type of `Vec<T>`/`Box<[T]>` is `&[T]`
  if `T` is zero-copy, and `Vec<T::DeserType<'_>>`/`Box<[T::DeserType<'_>]>` if
  `T` is deep-copy; `&[T]` and `SerIter<T>` are not deserializable.

* arrays `[T; N]` are zero-copy if and only if `T` is zero-copy; their
  serialization type is `[T; N]` if `T` is zero-copy, but `[T::SerType; N]` if
  `T` is deep-copy; their deserialization type is `&[T; N]` if `T` is zero-copy,
  but `[T::DeserType<'_>; N]` if `T` is deep-copy;

* tuples up to size 12 made of the same zero-copy type `T` are zero-copy, their
  serialization type is themselves, and their deserialization type is a
  reference to themselves (the other cases must be covered using [newtypes]);

* [`String`], `Box<str>` and `&str` are deep-copy, and their serialization type
  is `Box<str>`; the deserialization type of [`String`] and `Box<str>` is `&str`,
  whereas `&str` is not deserializable;

* ranges and `ControlFlow<B, C>` behave like user-defined deep-copy types;

* `Box<T>`, `Rc<T>`, and `Arc<T>`, for sized `T`, are deep-copy, and their
  serialization/deserialization type are the same of `T`.

## Derived and hand-made implementations

We strongly suggest using the procedural macro [`Epserde`] to make your own
types serializable and deserializable. Just invoking the macro on your structure
will make it fully functional with ε-serde. The attribute `#[zero_copy]` can be
used to make a structure zero-copy, albeit it must satisfy [a few
prerequisites].

You can also implement manually the traits [`CopyType`], [`MaxSizeOf`],
[`TypeHash`], [`ReprHash`], [`SerInner`], and [`DeserInner`], but
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
[`DeserInner`]: <https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html>
[`Deserialize`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html>
[`SerInner`]: <https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html>
[`Serialize`]: <https://docs.rs/epserde/latest/epserde/ser/trait.Serialize.html>
[`TypeInfo`]: <https://docs.rs/epserde/latest/epserde/derive.TypeInfo.html>
[`Epserde`]: <https://docs.rs/epserde/latest/epserde_derive/derive.Epserde.html>
[`Deserialize::load_full`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_full>
[`deserialize_full`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#tymethod.deserialize_full>
[`deserialize_eps`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#tymethod.deserialize_eps>
[`DeserType`]: <https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html>
[`Deserialize::load_mem`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mem>
[`Deserialize::load_mmap`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mmap>
[`Deserialize::read_mem`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mem>
[`Deserialize::read_mmap`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mmap>
[`Deserialize::mmap`]: <https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.mmap>
[a few prerequisites]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html>
[deserialized type]: <https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html#associatedtype.DeserType>
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
[`S::SerType`]: <https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType>
[`D::SerType`]: <https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType>
[`D::DeserType<'_>`]: <https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html#associatedtype.DeserType>
[`Read`]: <https://doc.rust-lang.org/std/io/trait.Read.html>
[`read_exact`]: <https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact>
[_deep-copy_]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.DeepCopy.html>
[_zero-copy_]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.ZeroCopy.html>
[`Deep`]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/struct.Deep.html>
[`Zero`]: <https://docs.rs/epserde/latest/epserde/traits/copy_type/struct.Zero.html>
[newtypes]: <https://docs.rs/epserde/latest/epserde/impls/tuple/index.html>
