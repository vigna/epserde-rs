# ε-serde

[![crates.io](https://img.shields.io/crates/v/epserde.svg)](https://crates.io/crates/epserde)
[![docs.rs](https://docs.rs/epserde/badge.svg)](https://docs.rs/epserde)
[![rustc](https://img.shields.io/badge/rustc-1.85+-red.svg)](https://rust-lang.github.io/rfcs/2495-min-rust-version.html)
[![CI](https://github.com/vigna/epserde-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/vigna/epserde-rs/actions)
![license](https://img.shields.io/crates/l/epserde)
[![downloads](https://img.shields.io/crates/d/epserde)](https://crates.io/crates/epserde)
[![coveralls](https://coveralls.io/repos/github/vigna/epserde-rs/badge.svg?branch=main)](https://coveralls.io/github/vigna/epserde-rs?branch=main)

ε-serde is a Rust framework for ε-_copy_ *ser*ialization and *de*serialization.

## Why

Large immutable data structures need time to be deserialized using the [serde]
approach. A possible solution for this problem is given by frameworks such as
[Abomonation], [rkyv], and [zerovec], which provide _zero-copy_ deserialization:
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
not copied. We call this approach _ε-copy deserialization_, as typically a
minuscule fraction of the serialized data is copied to build the structure. The
result is similar to that of the frameworks above, but the performance of the
deserialized structure will be identical to that of a standard, in-memory Rust
structure, as references are resolved at deserialization time.

We provide procedural macros implementing serialization and deserialization
methods, basic (de)serialization for primitive types, vectors, etc., convenience
memory-mapping methods based on [mmap_rs], and a [`MemCase`] structure that
couples a deserialized instance with its backend (e.g., a slice of memory or a
memory-mapped region). A [`MemCase`] provides an [`uncase`] method that yields
references to the deserialized instance it contains. Moreover, a [`MemCase`] can
also contain an owned instance, making it possible to use the same code for both
owned and, say, memory-mapped instances.

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
  tree. (You can actually have some references to zero-copy data though—see
  [this example].)

- While we provide procedural macros that implement serialization and
  deserialization, they require that your type is written and used in a specific
  way for ε-copy deserialization to work properly; in particular, the fields you
  want to ε-copy must be type parameters implementing [`DeserInner`], to which a
  [deserialization type] is associated. For example, we provide implementations
  for `Vec<T>`/`Box<[T]>`, where `T` is zero-copy, or `String`/`Box<str>`, which
  have deserialization associated type `&[T]` or `&str`, respectively. Vectors
  and boxed slices whose elements are not zero-copy will be deserialized
  recursively in memory instead.

- After deserialization of an instance of type `T`, you will obtain an instance
  of a deserialization associated type [`DeserType<'_,T>`], which is just an
  instance of `T` with different values for the type parameters (e.g.,
  `&[usize]` instead of `Vec<usize>`), and which will usually reference the
  underlying serialized support (e.g., a memory-mapped region); hence the need
  for a lifetime. If you need to store the deserialized instance in a field of a
  new structure you will need to couple permanently the instance with its
  serialization support, which is obtained by putting it in a [`MemCase`] using
  the convenience methods [`Deserialize::load_mem`], [`Deserialize::read_mem`],
  [`Deserialize::load_mmap`], [`Deserialize::read_mmap`], and
  [`Deserialize::mmap`].

- You must write `impl` blocks that work both for `T` and for
  `DeserType<'_,T>`. For example, if you have a field of type `Vec<T>`, you will
  get a field of type `&[T]` after deserialization, so you must write your code
  in a way that works for both types (e.g., by using `AsRef<[T]>`).

- No validation or padding cleaning is performed on (de)serialized instances. If
  you plan to serialize data and distribute it, you must take care of these
  issues.

## Pros

- Almost instant deserialization with minimal allocation provided that you
  designed your type following the ε-serde guidelines or that you use standard
  types.

- The instance you get by deserialization has essentially the same type
  as the one you serialized, except that type parameters will be replaced by
  their deserialization associated type (e.g., vectors will become references to
  slices). This is not the case with [rkyv], which requires you to reimplement
  all methods on a new, different deserialization type.

- The structure you get by deserialization has exactly the same performance as
  the structure you serialized. This is not the case with [zerovec] or [rkyv],
  which have to resolve relative addressing.

- You can serialize instances containing references to slices, or even
  exact-size iterators, and they will be deserialized as if you had written a
  vector. It is thus possible to serialize instances larger than the available
  memory, and later map them into memory.

- You can deserialize from read-only supports, as all dynamic information
  generated at deserialization time is stored in newly allocated memory. This is
  not the case with [Abomonation].

## Example: Zero-copy of standard types

Let us start with the simplest case: data that can be zero-copy deserialized. In
this case, we serialize an array of a thousand zeros, and get back a reference
to such an array:

```rust
# use epserde::prelude::*;
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
    unsafe { <[usize; 1000]>::deserialize_full(&mut std::fs::File::open(&file)?)? };
assert_eq!(s, t);

// In this case we map the array into memory
let u: MemCase<[usize; 1000]> =
    unsafe { <[usize; 1000]>::mmap(&file, Flags::empty())? };

assert_eq!(s, **u.uncase());

# Ok::<(), Box<dyn std::error::Error>>(())
```

Note how we serialize an array, but we deserialize a reference. The reference
points inside `b`, so there is no copy performed. The call to
[`deserialize_full`] creates a new array instead. The fourth call maps the data
structure into memory and returns a [`MemCase`] that can be used to get
a reference to the array; moreover, the [`MemCase`] can be passed to other
functions or stored in a structure field, as it contains both the structure and
the memory-mapped region that supports it.

The type alias [`DeserType`] can be used to derive the deserialization type
associated with a type. It contains a lifetime, which is the lifetime of the
memory region containing the serialized data. When deserializing into a
[`MemCase`], however, the lifetime is `'static`, as [`MemCase`] is an owned
type.

## Example: ε-copy of standard types

Zero-copy deserialization is not that interesting because it can be applied only
to data whose memory layout and size are fixed and known at compile time. This
time, let us serialize a `Vec` containing a thousand zeros: ε-serde will
deserialize its deserialization associated type, which is a reference to a
slice.

```rust
# use epserde::prelude::*;
let s = vec![0; 1000];

// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized1");
unsafe { s.serialize(&mut std::fs::File::create(&file)?)? };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t will be inferred--it is shown here only for clarity
let t: &[usize] =
    unsafe { <Vec<usize>>::deserialize_eps(b.as_ref())? };

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<usize> =
    unsafe { <Vec<usize>>::load_full(&file)? };
assert_eq!(s, t);

// In this case we map the vector into memory
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
little copy performed (in fact, just a pointer and the length of the slice). All
this is because `usize` is a zero-copy type. Note also that we use the
convenience method [`Deserialize::load_full`].

If your code must work both with the original and the deserialized version,
however, it must be written for a trait that is implemented by both types, such
as `AsRef<[usize]>`.

## Example: Zero-copy user-defined structures

You can define your types to be zero-copy, in which case they will work like
`usize` in the previous examples. This requires the structure to be made of
zero-copy fields, and to be annotated with `#[epserde(zero_copy)]` and `#[repr(C)]`
(which means that you will lose the possibility that the compiler reorders the
fields to optimize memory usage):

```rust
# use epserde::prelude::*;
#[derive(Epserde, Debug, PartialEq, Copy, Clone)]
#[repr(C)]
#[epserde(zero_copy)]
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
let t: &[Data] =
    unsafe { <Vec<Data>>::deserialize_eps(b.as_ref())? };

assert_eq!(s, *t);

// This is a traditional deserialization instead
let t: Vec<Data> =
    unsafe { <Vec<Data>>::load_full(&file)? };
assert_eq!(s, t);

// In this case we map the vector into memory
let u: MemCase<Vec<Data>> =
    unsafe { <Vec<Data>>::mmap(&file, Flags::empty())? };
assert_eq!(s, **u.uncase());

# Ok::<(), Box<dyn std::error::Error>>(())
```

If a type is not zero-copy, instead, vectors/boxed slices will be always
deserialized into vectors/boxed slices.

If you define a type that satisfies the requirements for being zero-copy, but
has no annotation, ε-serde will cause a compilation error. You must annotate the
type with either `#[epserde(zero_copy)]` or `#[epserde(deep_copy)]` to silence the
error.

## Example: User-defined structures with parameters

More flexibility can be obtained by defining types with fields whose types are
defined by parameters. In this case, ε-serde will deserialize instances of the
type replacing its type parameters with the deserialization associated type.

Let us design a structure that will contain an integer, which will be copied,
and a vector of integers that we want to ε-copy:

```rust
# use epserde::prelude::*;
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
unsafe { s.store(&file)? };
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

// In this case we map the structure into memory
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
deserialization associated type; we can also use `type` to reduce the clutter:

```rust
# use epserde::prelude::*;
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
unsafe { s.store(&file)? };
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

It is important to note that the derive code replaces every occurrence of a type
parameter with its associated (de)serialization type. The substitution is
uniform across the structure, so the same parameter cannot end up in two
different forms in the deserialization type. For example, the following code
will not compile:

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

The error reports that the trait bound `A: CopyType` is not satisfied: the
derive can substitute `A` consistently in both fields only when it knows
whether `A` is deep-copy or zero-copy, because `Vec<A>`'s deserialization type
depends on that.

Adding a bound on `A` resolves the issue:

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq)]
struct MyStructParam<A: DeepCopy> {
    id: isize,
    data: A,
    vec: Vec<A>
}
#     Ok(())
# }
```

Alternatively, when you want a specific field's type to stay verbatim in the
deserialization type (no substitution inside it, full-copy deserialization),
mark that field with the [`#[epserde(force_full_copy)]`
attribute](#example-pinning-a-field-with-force_full_copy).

Note that adding the bound `A: ZeroCopy` will not work—the derive should replace
`Vec<A>` with `&[A]` in the deserialization type, but that is incompatible
with the syntax of the type (a specific error will be issued).

Replacement happens recursively:

```rust
# use epserde::prelude::*;
#[derive(Epserde, Debug, PartialEq)]
struct MyStructRec<A: DeepCopy> {
    data: Vec<A>,
}

// Create a structure where A is a Vec<isize>
let s = MyStructRec { data: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]] };
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized5");
unsafe { s.store(&file)? };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;
let t: MyStructRec<&[i32]> = unsafe { <MyStructRec<Vec<i32>>>::deserialize_eps(b.as_ref())? };

assert_eq!(s.data.len(), t.data.len());
assert_eq!(&s.data[0], t.data[0]);
assert_eq!(&s.data[1], t.data[1]);

# Ok::<(), Box<dyn std::error::Error>>(())
```

In this case we have to bound `A` to be `DeepCopy` because a zero-copy `A` would
make the type [_unstable_](#specification)—ε-copy deserialization would require
replacing `Vec<A>` with `&[A]`, which is impossible. The rules governing type
replacement are discussed in the [specification](#specification).

## Example: User-defined deep-copy structures without parameters

When a deep-copy structure has no type parameters, its fields have no variable
position to substitute. The derive deserializes every field via the full-copy
path automatically, and the deserialization type is the original type itself.
For example,

```rust
# use epserde::prelude::*;
#[derive(Epserde, Debug, PartialEq)]
struct MyStruct(Vec<isize>);

let s = MyStruct(vec![0, 1, 2, 3]);
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized6");
unsafe { s.store(&file)? };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t is unchanged
let t: MyStruct = unsafe { <MyStruct>::deserialize_eps(b.as_ref())? };

# Ok::<(), Box<dyn std::error::Error>>(())
```

Note how the field of type `Vec<isize>` remains of the same type. To keep an
internal parameter untouched in the deserialization type of a _generic_
structure (e.g., a `Vec<A>` field where you do not want `A` to be substituted
across the structure), use [`#[epserde(force_full_copy)]`
attribute](#example-pinning-a-field-with-force_full_copy) on the field.

## Example: User-defined zero-copy structures with parameters

For zero-copy types, things are slightly different because type parameters are
not substituted, even if they are the type of a field. So all type parameters
must be zero-copy. This must hold even for types inside a [`PhantomData`]. For
example,

```rust
# use epserde::prelude::*;
#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
#[repr(C)]
#[epserde(zero_copy)]
struct MyStruct<A: ZeroCopy> {
    data: A,
}

// Create a structure where A is a Vec<isize>
let s: MyStruct<i32> = MyStruct { data: 0 };
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized7");
unsafe { s.store(&file)? };
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
# use epserde::prelude::*;
#[derive(Epserde, Debug, PartialEq, Clone, Copy)]
enum Enum<T=Vec<usize>> {
    A,
    B(T),
}

// This enum has T=Vec<i32> by type inference
let e = Enum::B(vec![0, 1, 2, 3]);
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized8");
unsafe { e.store(&file)? };
// Deserializing using just Enum will fail, as the type parameter
// by default is Vec<usize>
assert!(unsafe { <Enum>::load_full(&file) }.is_err());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Example: (Structures containing references to) slices

For convenience, ε-serde can serialize references to slices, and will
deserialize them as if they were vectors/boxed slices. You must however be careful to
deserialize with the correct type. For example,

```rust
# use epserde::prelude::*;
let v = vec![0, 1, 2, 3];
// This is a slice
let s: &[i32] = v.as_ref();
// Serialize it
let mut file = std::env::temp_dir();
file.push("serialized9");
unsafe { s.store(&file)? };
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
unsafe { d.store(&file)? };
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

## Example: Pinning a field with `force_full_copy`

By default the derive substitutes every occurrence of a type parameter with its
deserialization associated type, and deserializes every field by ε-copy
deserialization if it contains some type parameter. The field-level attribute
`#[epserde(force_full_copy)]` opts a specific field out of that default: the field is
fully deserialized.

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq)]
#[epserde(deep_copy)]
struct Inner<T>(T);

#[derive(Epserde, Debug, PartialEq)]
struct Outer<T>(#[epserde(force_full_copy)] Inner<T>);

let s: Outer<Vec<isize>> = Outer(Inner(vec![0, 1, 2, 3]));
let mut file = std::env::temp_dir();
file.push("serialized10");
unsafe { s.store(&file)? };
let b = std::fs::read(&file)?;

// force_full_copy pins the inner field: its type stays Inner<Vec<isize>> in the
// deserialization type rather than being substituted to Inner<&[isize]>.
let t: Outer<Vec<isize>> =
    unsafe { <Outer<Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.0.0, t.0.0);

# Ok::<(), Box<dyn std::error::Error>>(())
# }
```

`force_full_copy` takes no arguments and affects only deserialization. It is
rejected if it appears anywhere inside a type marked `#[epserde(zero_copy)]`, as
the marker has no meaning there. On a field whose type contains no type
parameter the marker is a silent no-op.

Marking a field whose type contains a parameter that also appears in another
unmarked field is inconsistent: we use a helper trait and the `#[diagnostic]`
attribute to report to the user a helpful error message in this case.

There is also a type-level companion, `#[epserde(full_copy(T, U, …))]`, which pins
the listed type parameters to full-copy deserialization across the whole type.
Whereas the field-level `force_full_copy` _forces_ a field that could be ε-copy
to be full-copy instead, `full_copy(T)` _declares_ that `T` is genuinely
full-copy when the derive's local, syntactic analysis cannot determine it. It is
rejected on zero-copy types, on const parameters, and on identifiers that are
not declared type parameters. Note an important interplay between the two
features: if a field contains only parameters that are declared full-copy, then
the field will be considered full-copy (even if it contains parameters).

If the parameter is not merely full-copy but phantom throughout the type, use
the stronger type-level attribute `#[epserde(phantom(T, U, …))]` instead (see the
[`PhantomData`](#phantomdata) section).

## Example: Pinning associated types with `bound`

When a type parameter `B` has a trait bound with an associated type, and both
`B` and that associated type are used as a field type, the latter will be
replaced with the type associated to [`DeserType<'_, B>`], which might cause
problems as the compiler does not know how the new associated type is related to
the original one. You can sometimes solve this problem by adding trait bounds
using the `bound` attribute. In this case, we pin the associated type `Mask` of
the trait `HasMask` to be the same for both the original type `B` and its
deserialization type:

```rust
use epserde::prelude::*;

trait HasMask {
    type Mask: SerInner + DeserInner + Copy + 'static;
}

# impl HasMask for Vec<usize> { type Mask = usize; }
# impl HasMask for Box<[usize]> { type Mask = usize; }
# impl HasMask for &[usize] { type Mask = usize; }
#[derive(Epserde, Debug, Clone)]
#[epserde(bound(
    deser = "for<'a> <B as DeserInner>::DeserType<'a>: HasMask<Mask = B::Mask>"
))]
struct Data<B: HasMask> {
    bits: B,
    mask: B::Mask,
}
```

Without the `bound` attribute, this would fail to compile because the
deserialization type `Data<B::DeserType<'a>>` expects a field of type
`<B::DeserType<'a> as HasMask>::Mask`, but the generated code produces a value
of type `<B as HasMask>::Mask`. The bound tells the compiler these are the same
type. This works because we expect `Mask` to be a primitive type, whose
deserialization type is itself, but in general more complex bounds might be
needed.

## Example: `impl` blocks for nested types

When you write `impl` blocks that work both for the original type and for the
deserialization type, for type parameters that are replaced by a reference you need
to use a suitable trait bound, usually `AsRef<[T]>`. For example,

```rust
# use epserde::prelude::*;
// Intended usage: MyStruct<Vec<usize>> or MyStruct<Box<[usize]>>
#[derive(Epserde)]
struct MyStruct<A> {
    data: A,
}

/// This method can be called on both an original and an ε-copied structure
impl <A: AsRef<[usize]>> MyStruct<A> {
    fn sum(&self) -> usize {
        self.data.as_ref().iter().sum()
    }
}
```

However, if we start to nest opaque types, `impl` section needs to be written
by unrolling the nested type, as we need to bound the inner type parameters:

```rust
# use epserde::prelude::*;
# #[derive(Epserde)]
# struct MyStruct<A> {
#     data: A,
# }
# impl <A: AsRef<[usize]>> MyStruct<A> {
#     fn sum(&self) -> usize {
#         self.data.as_ref().iter().sum()
#     }
# }
#[derive(Epserde)]
struct MyNestedStruct<B> {
    inner: B,
}

/// Note how we had to unroll the nested type
impl <A: AsRef<[usize]>> MyNestedStruct<MyStruct<A>> {
    fn sum(&self) -> usize {
        self.inner.sum()
    }
}
```

## Example: (Structures containing) iterators

ε-serde can serialize exact-size iterators. The resulting field can be
deserialized as a vector/boxed slice. In this case we need to wrap the
iterator in a [`SerIter`], as ε-serde cannot implement the serialization traits
directly on [`Iterator`]. For example,

```rust
# use epserde::prelude::*;
# use core::slice::Iter;
let v = vec![0, 1, 2, 3];
// This is an iterator
let i: Iter<'_, i32> = v.iter();
// Serialize it by wrapping it in a SerIter
let mut file = std::env::temp_dir();
file.push("serialized11");
unsafe { SerIter::<i32, _>::from(i).store(&file)? };
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

let d = Data { s: SerIter::<i32, _>::from(v.iter()) };
// Serialize it
unsafe { d.store(&file)? };
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

## Example (advanced): Structures containing references

It is technically possible to serialize and ε-copy deserialize structures
containing references to (sequences of) zero-copy types, whereas such structures
are obviously not fully deserializable. The trait implementations, however, must
be handled manually, as the derive code does not at this time handle lifetimes.

```rust
# use epserde::{deser::deser_eps_slice_zero, prelude::*, ser::ser_slice_zero, ser::WriteWithNames, ser::SerType};
# use core::hash::Hash;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
struct S<'a>(&'a [u8]);

unsafe impl<'a> CopyType for S<'a> {
    type Copy = Deep;
}

impl<'a> TypeHash for S<'a> {
    fn type_hash(hasher: &mut impl core::hash::Hasher) {
        "DeepCopy".hash(hasher);
        "S".hash(hasher);
        "0".hash(hasher);
        <SerType<&[u8]>>::type_hash(hasher);
    }
}


impl<'a> AlignHash for S<'a> {
    fn align_hash(hasher: &mut impl core::hash::Hasher, _offset_of: &mut usize) {
        <SerType<&[u8]> as AlignHash>::align_hash(hasher, &mut 0);
    }
}

impl<'a> SerInner for S<'a> {
    type SerType = S<'a>;
    const IS_ZERO_COPY: bool = false;
    unsafe fn _ser_inner(&self, backend: &mut impl ser::WriteWithNames) -> ser::Result<()> {
        unsafe {
            WriteWithNames::write(backend, "0", &self.0)?;
        }
        Ok(())
    }
}

impl<'a> DeserInner for S<'a> {
    type DeserType<'b> = S<'b>;

    fn __check_covariance<'__long: '__short, '__short>(
        proof: epserde::deser::CovariantProof<Self::DeserType<'__long>>,
    ) -> epserde::deser::CovariantProof<Self::DeserType<'__short>> {
        proof
    }

    unsafe fn _deser_full_inner(backend: &mut impl ReadWithPos) -> deser::Result<Self> {
        unimplemented!();
    }

    unsafe fn _deser_eps_inner<'c>(
        backend: &mut SliceWithPos<'c>,
    ) -> deser::Result<Self::DeserType<'c>> {
        unsafe { Ok(S(deser_eps_slice_zero(backend)?)) }
    }
}

let s = [0_u8, 1, 2, 3];
let v = S(&s);
let mut file = std::env::temp_dir();
file.push("serialized12");
unsafe { v.store(&file)? };
let b = std::fs::read(&file)?;
let w = unsafe { <S>::deserialize_eps(&b)? };
assert_eq!(v.0, w.0);
#     Ok(())
# }
```

The code above follows closely the derive-generated implementation you could
obtain if the inner type was `Vec<u8>`, just replacing the inner type where
necessary, and keeping full-copy deserialization unimplemented, as there is no
type with an owned inner field to return.

## More Examples

The standard `examples` directory contains many worked-out examples. The
[`sux-rs`] crate contains several data structures that use ε-serde.

## References and Wrappers

You can serialize using just a (mutable) reference. Moreover, wrappers such as
[`Box`], [`Rc`], and [`Arc`] are supported by _erasure_—they are dynamically
removed from the type and dynamically reinstated at deserialization if you
require them.
Please see the documentation of the [`wrapper`] module for more details.

## Vectors and Boxed slices

Vectors and boxed slices are entirely interchangeable in ε-serde. In
particular, you can serialize a vector and deserialize it as a boxed slice, or
vice versa, even when they are fields of a type (given, of course, that they
are the concrete type of a type parameter).

## [`PhantomData`]

[`PhantomData`] undergoes a special treatment: its type parameter `T` does not
have to be (de)serializable or sized—it is sufficient that it implements
[`TypeHash`]. For this reason, we provide [`TypeHash`] implementations for
`*const T`, `str`, and `[T]`.

When `T` appears both as a parameter of a [`PhantomData`] field and as the type
of another field, the `Epserde` derive substitutes `T` inside [`PhantomData<T>`]
natively, so the following code compiles and round-trips correctly:

```rust
# use epserde::prelude::*;
# use std::marker::PhantomData;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq, Eq, Clone, Default)]
struct Data<T> {
    data: T,
    phantom: PhantomData<T>,
}

let s: Data<Vec<isize>> = Data { data: vec![0, 1, 2, 3], phantom: PhantomData };
let mut file = std::env::temp_dir();
file.push("serialized13");
unsafe { s.store(&file)? };
let b = std::fs::read(&file)?;

// The data field is substituted to &[isize] in the ε-copy deserialized form,
// and so is the phantom slot.
let t = unsafe { <Data<Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.data.as_slice(), t.data);
let _phantom_check: PhantomData<&[isize]> = t.phantom;
#     Ok(())
# }
```

Note how the phantom field originally of type `PhantomData<Vec<isize>>` becomes
`PhantomData<&[isize]>` in the ε-copy deserialized form, consistently with the
substitution applied to `data`.

This special treatment, however, works only when the derive can see the
[`PhantomData`] field. When a parameter is phantom throughout the type, but it
occurs as a bare generic argument of a field type, the derive's local, syntactic
analysis cannot know that the field type keeps it in a [`PhantomData`] slot. The
type-level attribute `#[epserde(phantom(T, U, …))]` declares that the listed type
parameters are phantom throughout the type: they are left completely untouched,
so they can be instantiated with types that are not serializable at all, such as
`str`:

```rust
# use epserde::prelude::*;
# use std::marker::PhantomData;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Inner<K: ?Sized, T> {
    data: T,
    phantom: PhantomData<K>,
}

// K appears as a bare generic argument of the field type, but Inner keeps it
// in a PhantomData slot: we declare it phantom throughout the type.
#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
#[epserde(phantom(K))]
struct Data<K: ?Sized, T> {
    inner: Inner<K, T>,
}

let s: Data<str, Vec<isize>> = Data {
    inner: Inner { data: vec![0, 1, 2, 3], phantom: PhantomData },
};
let mut file = std::env::temp_dir();
file.push("serialized14");
unsafe { s.store(&file)? };
let b = std::fs::read(&file)?;

// K stays str in the ε-copy deserialized form; only T is substituted.
let t = unsafe { <Data<str, Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.inner.data.as_slice(), t.inner.data);
#     Ok(())
# }
```

Like `full_copy(…)`, the attribute is rejected on zero-copy types, on const
parameters, and on identifiers that are not declared type parameters;
moreover, a parameter cannot be listed both in `phantom(…)` and in
`full_copy(…)`, as the former is a strictly stronger claim.

## MemDbg / MemSize

All ε-serde structures implement the [`MemDbg`] and [`MemSize`] traits if the
`mem_dbg` feature is enabled (which is the default).

## Design

Every type (de)serializable with ε-serde has three features that are in principle
orthogonal, but that in practice often condition one another:

- the type has a _serialization type_, which is the type we write when
  serializing;
- the type has a _deserialization associated type_, which is the type you
  obtain after an ε-copy deserialization invoked on the type;
- the type can be either deep-copy or zero-copy.

There is no constraint on the associated (de)serialization type: it can be
literally anything. In general, however, one tries to have a deserialization
type that is somewhat compatible with the original type, in the sense that they
both satisfy a trait for which implementations can be written: for example,
ε-copy deserialization turns vectors/boxed slices of zero-copy types into
references to slices, so implementations can be written for `AsRef<[·]>` and
will work both on the original and the deserialized instance. And, in general,
zero-copy types deserialize into themselves. The serialization type is obtained
after some normalization and erasure: references to slices and iterators are
serialized for convenience as boxed slices, and wrappers such as [`Rc`] are
erased.

Being zero-copy or deep-copy decides how the type will be treated upon
deserialization. Instances of zero-copy types are ε-copy deserialized as a
reference, whereas instances of deep-copy types are always recursively
deserialized in allocated memory.

Sequences of zero-copy types are ε-copy deserialized using a reference to a
slice, whereas sequences of deep-copy types are deserialized in allocated memory
(to sequences of the deserialization associated types). It is important to
remark that you cannot serialize a sequence whose elements are of a type that
implements neither [`ZeroCopy`] nor [`DeepCopy`], even if in that case
ε-serde considers the type as deep-copy (see the following section and the
[`CopyType`] documentation for a deeper explanation).

Logically, zero-copy types should be deserialized to references, and this indeed
happens in most cases, and certainly in the derived code: however, _primitive
types are always fully deserialized_. There are three reasons behind this
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
recursively, field by field. The basic idea in ε-serde is that during ε-copy
deserialization a type is replaced with its deserialization type. Since the
deserialization type is defined recursively, replacement can happen at any depth
level.

Replacement happens in all fields, including occurrences nested inside other
field types, as long as the field contains at least one type parameter. The
field-level [`#[epserde(force_full_copy)]`
attribute](#example-pinning-a-field-with-force_full_copy) opts a specific field
out of substitution: the field is deserialized via the full-copy path and its
type is preserved verbatim in the deserialization type.

Occurrences nested inside [`PhantomData<T>`] are transparent to the derive: the
derive just emits a `PhantomData` token, and type inference adjusts it to
the correct type (`T` or the deserialization type of `T`).

This approach makes it possible to write ε-serde-aware structures that hide from
the user the substitution. A good example is the [`BitFieldVec`] structure from
[`sux`], which exposes an array of fields of fixed bit width using (usually) a
`Vec<usize>` as a backend; except for extension methods, all methods of
[`BitFieldVec`] come from the trait [`BitFieldSlice`]. If you have your own
user-defined type and one of the fields is of type `A`, when serializing an
instance with `A` equal to `BitFieldVec<Vec<usize>>`, upon ε-copy
deserialization you will get a version of your instance with that field of type
`BitFieldVec<&[usize]>`. All this will happen under the hood because
[`BitFieldVec`] is ε-serde-aware, and in fact you will not even notice the
difference if you access both versions using the trait [`BitFieldSlice`].

## Specification

In this section we describe in a somewhat formal way the specification of
ε-serde. We suggest to get acquainted with the examples before reading it.

### The main traits

The two main traits of ε-serde are [`SerInner`] and [`DeserInner`], which
specify respectively how to serialize and deserialize a type. [`SerInner`] has
an associated [`SerType`], which is (approximately) the type that will be
actually serialized; this approach makes it possible, for example, to serialize
iterators as boxed slices. [`DeserInner`] has an associated _deserialization
type_ [`DeserType<'_>`], which has a lifetime and will be used to deserialize
instances from in-memory data, mostly referencing to such data instead of
copying it: we call this process _ε-copy deserialization_.

The approximation above is due to the fact that when user-defined types are
involved the serialization type might not be expressible in source code: for
example, `S(Rc<Vec<usize>>)` has [`SerType`] equal to `Self`, but its
serialization type is actually `S(Box<[usize]>)` as the [`Rc`] is erased and the
[`Vec`] is normalized to a boxed slice. The property we maintain is that the
[`TypeHash`] is computed on the actual serialization type, so user-defined types
with the same serialization type can be deserialized interchangeably.

### Types

Within ε-serde, types are classified as [_deep-copy_] or [_zero-copy_] by
implementing the unsafe [`CopyType`] trait with associated type
[`CopyType::Copy`] equal to [`Deep`] or [`Zero`]; types without such an
implementation are considered deep-copy.

There is a blanket implementation for the [`DeepCopy`] trait for all types
implementing [`CopyType`] with associated type [`CopyType::Copy`] equal to
[`Deep`] and also implementing [`SerInner`] with [`SerInner::SerType`]
implementing [`TypeHash`] and [`AlignHash`].

There is analogously a blanket implementation for the [`ZeroCopy`] trait for all
types implementing [`CopyType`] with associated type [`CopyType::Copy`] equal to
[`Zero`] and also implementing [`Copy`], [`TypeHash`], [`AlignHash`],
[`PadTo`] and [`SerInner`] with [`SerType`] equal to `Self`; they must also
outlive the `'static` lifetime, and be `repr(C)`. Note that in this case we
bound the original type, rather than the [`SerType`], because zero-copy types
are always serialized as themselves, and all fields of a zero-copy type are
zero-copy as well.

Zero-copy types cannot contain any reference, but this condition cannot be
checked by the compiler (`'static` does not prevent, say, references to string
constants), which is why [`CopyType`] is unsafe.

Deep-copy types are types that must be serialized and deserialized field by
field.

Zero-copy types instead have the property that their memory representation can
be serialized as a sequence of bytes; a reference to the sequence is then a
valid reference to an instance of the type. This happens because `repr(C)`
guarantees a fixed memory layout, and because the type does not contain any
reference to other data.

Note that all fields of a type you want to (de)serialize must implement
[`CopyType`] for the derive code to work.

### ε-copy / full-copy fields and parameters

Given a type `S` with generics, a field is _full-copy_ if it does not contain
type parameters outside those listed in the type-level attribute
`#[epserde(full_copy(T, U, …))]`, or it is marked with
`#[epserde(force_full_copy)]`. A field is _ε-copy_ otherwise. A type parameter
`T` is _ε-copy_ if it appears in an ε-copy field, and is _full-copy_ if it
appears in a full-copy field. Occurrences in a [`PhantomData`] or listed in the
type-level attribute `#[epserde(phantom(T, U, …))]` are not accounted for. No
type parameter can be both ε-copy and full-copy, and a special diagnostic is
emitted if this happens.

Note the interplay between the two attributes: `#[epserde(full_copy(T, U, …))]`
helps defining the type of fields by pinning certain parameters initially to
be full-copy, but ultimately the copy type of a type parameter depends on
where its occurrences appear.

The fundamental idea at the basis of ε-serde is that ε-copy parameters make it
possible to deserialize a value so that it refers to serialized zero-copy data
without copying it. For example, given a structure

```rust
struct Good<A> {
    data: A,
}
```

if `A` is `Vec<Z>` and `Z` is zero-copy, then we can deserialize an instance of
`Good<Vec<Z>>` from in-memory data by replacing `Vec<Z>` with `&[Z]`, and the
resulting structure will have deserialization type `Good<&[Z]>`: the slice will
refer directly to the serialized data, and only a small fraction of the
structure (a pointer and an integer) will need to be allocated—hence, the term
“ε-copy”.

This replacement happens recursively thanks to Rust's type system. As long as
`impl` sections are written in terms of traits implemented by both the original
type and on its deserialization associated type, the code will work
transparently on both types; in this case, methods should be written with the
bound `A: AsRef<[Z]>`.

The derive code substitutes every type parameter `T` with `<T as
SerInner>::SerType` in the [`SerType`], and every ε-copy type parameter `T` with
`<T as DeserInner>::DeserType<'_>` in the [`DeserType`] (full-copy parameters
are kept unchanged). Note that the projection on [`SerType`] normalizes all
sequence types (vectors and slices) to boxed slices and erases wrappers such as
smart pointers.

For the replacement to work, the type `S` must be _locally ε-copy stable_
with respect to the kind of parameters in `S`, meaning that its deserialization
type, obtained by replacing the concrete types of its ε-copy type parameters
with their deserialization associated types, is correctly ε-copy deserialized by
the recursive process described in the next section.

When this does not happen, the derive emits (when possible) code that fails
to type-check at the call site, or the compiler detects type inconsistencies.
The user can restore stability by adding `#[epserde(force_full_copy)]` to a
suitable field, a copy-kind bound (`T: DeepCopy` or `T: ZeroCopy`) on a
parameter, or add a parameter to the `#[epserde(full_copy(T, U, …))]` list. Note
that by making all fields full-copy, or all type parameters deep-copy, one can
always obtain stability, but losing the usefulness of ε-copy deserialization.

### Serialization and deserialization

An ε-serde serialization process involves two types:

- `S`, the _serializing type_, which is the type of the instance you want to
  serialize. It must implement [`SerInner`] with associated type
  [`SerInner::SerType`] implementing [`TypeHash`] and [`AlignHash`] (which
  implies [`Serialize`] on `S` by a blanket implementation).

- Its _serialization type_, which, as mentioned, is approximately [`<S as
  SerInner>::SerType`].

In general the serialization type of `S` is `S`, but there is some normalization
and erasure involved (e.g., vectors become boxed slices, and some wrappers such
as [`Rc`] are erased). Moreover, a few types that are not really serializable
have a convenience serialization type (e.g., iterators become boxed slices
through the wrapper [`SerIter`]).

When you invoke serialize on an instance of type `S`, ε-serde writes a magic
cookie containing version information, a [type hash] which is derived from
[`S::SerType`], and which represents the definition of the serialization type
(copy type, field names, types, etc.), an [alignment hash] which is derived from
the alignment of [`S::SerType`] (essentially, recording where padding had been
inserted in the zero-copy parts of the type), some debug information, and then
recursively the data contained in the instance.

Note that the [type hash] depends on the serialization type only, and not on
[`SerType`] itself. Recalling our example, `S(Rc<Vec<usize>>)` has [`SerType`]
equal to `Self`, but its serialization type is actually `S(Box<[usize]>)`, and
the type hash is computed on the latter, even if the type is not expressible in
source code.

An ε-serde deserialization process involves instead three types:

- `D`, the _deserializing type_, which must implement [`DeserInner`], and again
  [`SerInner`] with associated type [`SerInner::SerType`] implementing
  [`TypeHash`] and [`AlignHash`], so the blanket implementation for
  [`Deserialize`] applies. This is the type on which deserialization methods are
  invoked.

- The _serialization type_, which is approximately [`D::SerType`].

- The associated _deserialization type_ [`D::DeserType<'_>`].

In general `D` is the same as `S`, but the only relevant condition for using a
deserializing type `D` on a stored value serialized with serializing type `S` is
that they have the same serialization type, that is, the [type hash] of
[`D::SerType`] is equal to that of [`S::SerType`]. This gives some latitude in
the choice of the deserializing type—for example, a boxed array instead of a
vector for an ε-copy parameter, or creating wrapper types such as [`Rc`] on the
fly.

The deserialization type, instead, is the main technical novelty of ε-serde: it
is a reference, instead of an instance, for zero-copy types, and a reference to
a slice, rather than owned data, for vectors, boxed slices, and arrays of such
types. For vectors, boxed slices, and arrays of deep-copy types it is obtained
by replacing their type parameter with its deserialization type. For more
complex types, it is obtained by replacing ε-copy parameters with
their deserialization type.

Note that the deserialization type is an _actual associated type_ [`DeserType`],
contrary to the serialization type, of which [`SerType`] is only an
approximation.

For example:

- `T::DeserType<'_>` is `&T` if `T` is zero-copy, but
  `T` if `T` is deep-copy;

- `<Vec<T>>::DeserType<'_>` is `&[T]` if `T` is zero-copy, but
  `Vec<T>` if `T` is deep-copy;

- `<Good<T>>::DeserType<'_>` is `Good<&T>` if `T` is zero-copy, but
  `Good<T>` if `T` is deep-copy;

- `Good<Vec<T>>::DeserType<'_>` is `Good<&[T]>` if `T` is zero-copy, but
  `Good<Vec<T>>` if `T` is deep-copy;

- `Good<Vec<Vec<T>>>::DeserType<'_>` is `Good<Vec<&[T]>>` if `T` is zero-copy,
  but again `Good<Vec<Vec<T>>>` if `T` is deep-copy.

There are now two types of deserialization, to which two different _deserialized
types_ correspond:

- [`deserialize_full`] performs _full-copy deserialization_, which recursively
  full-copy deserializes the serialized data from a [`Read`] and builds an
  instance of `D`. This is basically a standard deserialization, except that it
  is usually much faster if you have large sequences of zero-copy types, as they
  are deserialized in a single [`read_exact`].

- [`deserialize_eps`] performs _ε-copy deserialization_, which accesses the
  serialized data as a byte slice, and builds an instance of `D::DeserType<'_>`
  that refers to the data inside the byte slice. In this case, if the
  deserializing type is zero-copy, [`deserialize_eps`] just returns a reference
  to the data; if it is a sequence of zero-copy types (e.g., a vector), it returns
  a reference to a slice; if it is a deep-copy type, it recursively ε-copy
  deserializes the ε-copy fields and full-copy deserializes the full-copy
  fields.

Whichever method you invoke on `D`, deserialization will happen only if the type
hash of [`D::SerType`] matches that of [`S::SerType`], and the same must happen
for the alignment hash: otherwise, you will get an error. Note that the
serialized data does not contain a structural copy of any type: it is the
responsibility of the code invoking the deserialization method to know the type
of the data it is reading.

We have previously introduced the notion of structural ε-copy stability: the first
and foremost cause of instability is ε-copy deserialization of (sequences of) zero-copy
types, but at the border of a type:

```ignore
# use epserde::prelude::*;
#[derive(Epserde)]
struct Unstable<X>(Vec<X>);
```

In this case the compiler will emit a very specific error, explaining that `X`
must be deep-copy, and that, alternatively, the error can also be solved with
the type attribute `#[epserde(full_copy(X))]`, or with the field attribute
`#[epserde(force_full_copy)]` on `Vec<X>`. The problem here is that since the
field is ε-copy, the deserialization type of `Vec<Z>` is `&[Z]` if `Z` is
zero-copy, but `Unstable(&[Z])` cannot be expressed by changing the type
parameters of `Unstable`; as a consequence, the result of the recursive
[`deserialize_eps`] is `&[Z]`, but the derive-generated code tries to assign it
to a field of type `Vec<X>`.

It is also worth noting that in ε-copy deserialization, once the recursion
traverses a full-copy field the rest of the deserialization process for that
field is full-copy, and no more ε-copy deserialization happens for the fields
nested inside it, even if they are ε-copy. This is the other reason for
instability.

For example:

```ignore
# use epserde::prelude::*;
#[derive(Epserde)]
struct Full<X>(#[epserde(force_full_copy)] X);
#[derive(Epserde)]
struct Eps<X>(Full<X>);
```

In this case, `X` is ε-copy, so the deserialization type of `Eps(Full(X))` is
`Eps(Full(X::DeserType<'_>))`, and the compiler complains that it expected
`Full<<X as DeserInner>::DeserType<'_>>` (the return value of
[`deserialize_eps`]), but found `Full<X>` (the type of the only field of `Eps`).

### Serialization and deserialization types

Given a user-defined type `T`:

- if `T` is zero-copy, the serialization type is `T`, and the deserialization
  type is `&T`;

- if `T` is a deep-copy concrete type obtained by resolving the type parameters
  `P₀`, `P₁`, `P₂`, … of a type definition (struct or enum) to concrete types
  `T₀`, `T₁`, `T₂`, …, then the serialization type is obtained by resolving each
  parameter `Pᵢ` with the serialization type of `Tᵢ`; the deserialization type
  is obtained instead by resolving each ε-copy type parameter `Pᵢ` with the
  deserialization type of `Tᵢ`. (Note that the first rule still applies,
  so if `Tᵢ` is zero-copy its deserialization type is `&Tᵢ`.) See [ε-copy and
  full-copy parameters](#ε-copy--full-copy-fields-and-parameters) for the definition of
  ε-copy parameters.

For standard types, we have:

- all primitive types, such as `u8`, `i32`, `f64`, `char`, `bool`, etc., and
  zero-sized types such as `()`, `RangeFull`, `PhantomData<T>`, etc., are
  zero-copy and their (de)serialization type is themselves; note however that
  when `T` is an ε-copy type parameter, the `Epserde` derive substitutes `T`
  inside `PhantomData<T>`, so the ε-copy deserialized form carries
  `PhantomData<T::DeserType<'_>>`;

- `Option<T>` is deep-copy and its (de)serialization type is itself, with `T`
  replaced by its (de)serialization type;

- `Vec<T>`, `Box<[T]>`, `&[T]` and `SerIter<T>` are deep-copy, and their
  serialization type is `Box<[T::SerType]>`; the deserialization type of
  `Vec<T>`/`Box<[T]>` is `&[T]` if `T` is zero-copy, and
  `Vec<T::DeserType<'_>>`/`Box<[T::DeserType<'_>]>` if `T` is deep-copy; `&[T]`
  and `SerIter<T>` are not deserializable.

- arrays `[T; N]` are zero-copy if and only if `T` is zero-copy: their
  serialization type is `[T::SerType; N]`, and their deserialization type is
  `&[T; N]` if `T` is zero-copy, but `[T::DeserType<'_>; N]` if `T` is
  deep-copy;

- tuples up to size 12 made of the same zero-copy type `T` are zero-copy, their
  serialization type is themselves, and their deserialization type is a
  reference to themselves (the other cases must be covered using [newtypes]);

- [`String`], `Box<str>` and `&str` are deep-copy, and their serialization type
  is `Box<str>`; the deserialization type of [`String`] and `Box<str>` is `&str`,
  whereas `&str` is not deserializable;

- ranges other than `RangeFull` and `ControlFlow<B, C>` behave like user-defined
  deep-copy types;

- `Box<T>`, `Rc<T>`, and `Arc<T>`, for sized `T`, are deep-copy, and their
  serialization/deserialization type are the same of `T` (e.g., they are
  _erased_).

Note that the normalization and erasure rules give some latitude in the choice of
the deserializing type: for example, if you serialized a `Vec<T>`, you can
deserialize it fully as a `Box<[T]>`, or an `Rc<Box<[T]>>`, or ε-copy
deserialize it as an `Arc<&[T]>`.

## Derived and hand-made implementations

We strongly suggest using the procedural macro [`Epserde`] to make your own
types serializable and deserializable. Just invoking the macro on your structure
will make it fully functional with ε-serde. The attribute `#[epserde(zero_copy)]`
can be used to make a structure zero-copy, albeit it must satisfy [a few
prerequisites].

The macro provides also an `#[epserde(bound(ser = ..., deser = ...))]` attribute,
which can be used to add trait bounds to the generated code (see the example
above on pinning associated types).

You can also implement manually the traits [`CopyType`], [`PadTo`],
[`TypeHash`], [`AlignHash`], [`SerInner`], and [`DeserInner`], but
the process is error-prone, and you must be fully aware of ε-serde's
conventions. The procedural macro [`TypeInfo`] can be used to generate
[`TypeHash`] and [`AlignHash`] automatically (plus [`PadTo`] for
zero-copy types).

## Acknowledgments

This software has been partially supported by project SERICS (PE00000014) under
the NRRP MUR program funded by the EU - NGEU, and by project ANR COREGRAPHIE,
grant ANR-20-CE23-0002 of the French Agence Nationale de la Recherche.
Views and opinions expressed are however those of the authors only and do not
necessarily reflect those of the European Union or the Italian MUR. Neither the
European Union nor the Italian MUR can be held responsible for them.

[`MemCase`]: https://docs.rs/epserde/latest/epserde/deser/mem_case/struct.MemCase.html
[`uncase`]: https://docs.rs/epserde/latest/epserde/deser/mem_case/struct.MemCase.html#method.uncase
[`ZeroCopy`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.ZeroCopy.html
[`DeepCopy`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.DeepCopy.html
[`CopyType`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html
[`PadTo`]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.PadTo.html
[alignment hash]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.AlignHash.html
[`TypeHash`]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.TypeHash.html
[`AlignHash`]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.AlignHash.html
[type hash]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.TypeHash.html
[`DeserInner`]: https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html
[`Deserialize`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html
[`SerInner`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html
[`Serialize`]: https://docs.rs/epserde/latest/epserde/ser/trait.Serialize.html
[`TypeInfo`]: https://docs.rs/epserde/latest/epserde/derive.TypeInfo.html
[`Epserde`]: https://docs.rs/epserde/latest/epserde/derive.Epserde.html
[`Deserialize::load_full`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_full
[`deserialize_full`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#tymethod.deserialize_full
[`deserialize_eps`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#tymethod.deserialize_eps
[`DeserType`]: https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html
[`Deserialize::load_mem`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mem
[`Deserialize::load_mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.load_mmap
[`Deserialize::read_mem`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mem
[`Deserialize::read_mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.read_mmap
[`Deserialize::mmap`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html#method.mmap
[a few prerequisites]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html
[deserialization type]: https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html#associatedtype.DeserType
[`DeserType<'_>`]: https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html
[`DeserType<'_,T>`]: https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html
[`DeserType<'_, B>`]: https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html
[`sux`]: https://crates.io/crates/sux
[serde]: https://serde.rs/
[Abomonation]: https://crates.io/crates/abomonation
[rkyv]: https://crates.io/crates/rkyv/
[zerovec]: https://crates.io/crates/zerovec
[mmap_rs]: https://crates.io/crates/mmap-rs
[`MemDbg`]: https://docs.rs/mem_dbg/latest/mem_dbg/trait.MemDbg.html
[`MemSize`]: https://docs.rs/mem_dbg/latest/mem_dbg/trait.MemSize.html
[`PhantomData`]: https://doc.rust-lang.org/std/marker/struct.PhantomData.html
[`PhantomData<T>`]: https://doc.rust-lang.org/core/marker/struct.PhantomData.html
[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[`SerIter`]: https://docs.rs/epserde/latest/epserde/impls/iter/struct.SerIter.html
[`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
[`Rc`]: https://doc.rust-lang.org/std/rc/struct.Rc.html
[`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
[`wrapper`]: https://docs.rs/epserde/latest/epserde/impls/wrapper/index.html
[`BitFieldVec`]: https://docs.rs/sux/latest/sux/bits/bit_field_vec/struct.BitFieldVec.html
[`BitFieldSlice`]: https://docs.rs/sux/latest/sux/traits/bit_field_slice/trait.BitFieldSlice.html
[`S::SerType`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType
[`D::SerType`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType
[`SerType`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType
[`D::DeserType<'_>`]: https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html#associatedtype.DeserType
[`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
[_deep-copy_]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.DeepCopy.html
[_zero-copy_]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.ZeroCopy.html
[`Deep`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/struct.Deep.html
[`Zero`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/struct.Zero.html
[newtypes]: https://docs.rs/epserde/latest/epserde/impls/tuple/index.html
[`Copy`]: https://doc.rust-lang.org/std/marker/trait.Copy.html
[`SerInner::SerType`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType
[`<S as SerInner>::SerType`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html#associatedtype.SerType
[`sux-rs`]: https://crates.io/crates/sux
[`CopyType::Copy`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html#associatedtype.Copy
[`String`]: https://doc.rust-lang.org/std/string/struct.String.html
[this example]: #example-advanced-structures-containing-references
