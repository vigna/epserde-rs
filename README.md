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
[Abomonation], [rkiv], and [zerovec], which provide _zero-copy_ deserialization:
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
memory-mapped region). A [`MemCase`] provides a [`uncase`] method that yields
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
  tree.

- While we provide procedural macros that implement serialization and
  deserialization, they require that your type is written and used in a specific
  way for ε-copy deserialization to work properly; in particular, the fields you
  want to ε-copy must be type parameters implementing [`DeserInner`], to which a
  [deserialization type] is associated. For example, we provide implementations
  for `Vec<T>`/`Box<[T]>`, where `T` is zero-copy, or `String`/`Box<str>`, which
  have associated deserialization type `&[T]` or `&str`, respectively. Vectors
  and boxed slices whose elements are not zero-copy will be deserialized
  recursively in memory instead.

- After deserialization of an instance of type `T`, you will obtain an instance
  of an associated deserialization type [`DeserType<'_,T>`], which will usually
  reference the underlying serialized support (e.g., a memory-mapped region);
  hence the need for a lifetime. If you need to store the deserialized instance
  in a field of a new structure you will need to couple permanently the instance
  with its serialization support, which is obtained by putting it in a
  [`MemCase`] using the convenience methods [`Deserialize::load_mem`],
  [`Deserialize::read_mem`], [`Deserialize::load_mmap`],
  [`Deserialize::read_mmap`], and [`Deserialize::mmap`].

- You must write `impl` blocks that works both for `T` and for
  `DeserType<'_,T>`. For example, if you have a field of type `Vec<T>`, you will
  get a field of type `&[T]` after deserialization, so you must write your code
  in a way that works for both types (e.g., by using `AsRef<[T]>`).

- No validation or padding cleaning is performed on deserialized instances. If
  you plan to serialize data and distribute it, you must take care of these
  issues.

## Pros

- Almost instant deserialization with minimal allocation provided that you
  designed your type following the ε-serde guidelines or that you use standard
  types.

- The instance you get by deserialization has essentially the same type
  as the one you serialized, except that type parameters will be replaced by
  their associated deserialization type (e.g., vectors will become references to
  slices). This is not the case with [rkiv], which requires you to reimplement
  all methods on a new, different deserialization type.

- The structure you get by deserialization has exactly the same performance as
  the structure you serialized. This is not the case with [zerovec] or [rkiv].

- You can serialize instances containing references to slices, or even
  exact-size iterators, and they will be deserialized as if you had written a
  vector. It is thus possible to serialize instances larger than the available
  memory.

- You can deserialize from read-only supports, as all dynamic information
  generated at deserialization time is stored in newly allocated memory. This is
  not the case with [Abomonation].

## Warning to Previous Users

The attributes `#[epserde_zero_copy]` and `#[epserde_deep_copy]` have been
renamed to `#[epserde(zero_copy)]` and `#[epserde(deep_copy)]`, respectively.
The old names will continue to work and raise a deprecation warning, but we
plan to remove them in the next major release.

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

// In this case we map the data structure into memory
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

The type alias [`DeserType`] can be used to derive the deserialization type
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

// In this case we map the data structure into memory
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
type replacing its type parameters with the associated deserialization type.

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
associated deserialization type; we can also use `type` to reduce the clutter:

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
mark that field with the [`#[epserde(force_full)]`
attribute](#example-pinning-a-field-with-force_full).

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
file.push("serialized5");
unsafe { s.store(&file) };
// Load the serialized form in a buffer
let b = std::fs::read(&file)?;

// The type of t is unchanged
let t: MyStruct = unsafe { <MyStruct>::deserialize_eps(b.as_ref())? };

# Ok::<(), Box<dyn std::error::Error>>(())
```

Note how the field of type `Vec<isize>` remains of the same type. To keep an
internal parameter untouched in the deserialization type of a _generic_
structure (e.g., a `Vec<A>` field where you do not want `A` to be substituted
across the structure), use [`#[epserde(force_full)]`
attribute](#example-pinning-a-field-with-force_full) on the field.

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
# use epserde::prelude::*;
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

## Example: Pinning a field with `force_full`

By default the derive substitutes every occurrence of a type parameter with its
associated deserialization type, and deserializes every field by ε-copy
deserialization if it contains some type parameter. The field-level attribute
`#[epserde(force_full)]` opts a specific field out of that default: the field is
fully deserialized.

```rust
# use epserde::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Epserde, Debug, PartialEq)]
#[epserde(deep_copy)]
struct Inner<T>(T);

#[derive(Epserde, Debug, PartialEq)]
struct Outer<T>(#[epserde(force_full)] Inner<T>);

let s: Outer<Vec<isize>> = Outer(Inner(vec![0, 1, 2, 3]));
let mut file = std::env::temp_dir();
file.push("serialized_force_full");
unsafe { s.store(&file) };
let b = std::fs::read(&file)?;

// `force_full` pins the inner field: its type stays Inner<Vec<isize>> in the
// deserialization type rather than being substituted to Inner<&[isize]>.
let t: Outer<Vec<isize>> =
    unsafe { <Outer<Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.0.0, t.0.0);

# Ok::<(), Box<dyn std::error::Error>>(())
# }
```

Typical use cases:

- A wrapper whose deserialization type cannot be obtained by uniformly
  substituting its parameters (the derive's default would emit code that
  fails to type-check).
- A `Vec<T>`, `Box<[T]>`, `[T; N]`, or `String` field at an internal
  zero-copy parameter: these constructors substitute their parameter only
  when it is deep-copy, so at a zero-copy kind the marker is the only way to
  keep the field round-tripping.

`force_full` takes no arguments and affects only deserialization. It is
rejected if it appears anywhere inside a type marked
`#[epserde(zero_copy)]`: zero-copy structs are (de)serialized as a
sequence of raw bytes with no field-level choice between
`_deser_full_inner` and `_deser_eps_inner`, so the marker has no
operational meaning there. On a deep-copy field whose type contains no
variable position the marker is a silent no-op: the field is already
deserialized full-copy by default, since there is nothing to substitute.

Marking a field whose type contains a parameter that also appears at a
variable position in another unmarked field leaves the per-occurrence
equations inconsistent: the parameter is substituted across the structure's
deserialization type (because of the unmarked occurrence) but the marked
field returns its type verbatim, so the two forms disagree. The derive does
not diagnose this; the generated code fails to type-check at the macro use
site.

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
file.push("serialized9");
unsafe { SerIter::<i32, _>::from(i).store(&file) };
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

## More Examples

The standard `examples` directory contains many worked-out examples. The
[`sux-rs`] crate contains several data structures that use ε-serde.

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
file.push("serialized_phantom");
unsafe { s.store(&file)? };
let b = std::fs::read(&file)?;

// The data field is substituted to &[isize] in the ε-deserialized form,
// and so is the phantom slot.
let t = unsafe { <Data<Vec<isize>>>::deserialize_eps(b.as_ref())? };
assert_eq!(s.data.as_slice(), t.data);
let _phantom_check: PhantomData<&[isize]> = t.phantom;
#     Ok(())
# }
```

Note how the phantom field originally of type `PhantomData<Vec<isize>>` becomes
`PhantomData<&[isize]>` in the ε-deserialized form, consistently with the
substitution applied to `data`.

## MemDbg / MemSize

All ε-serde structures implement the [`MemDbg`] and [`MemSize`] traits.

## Design

Every type (de)serializable with ε-serde has three features that are in principle
orthogonal, but that in practice often condition one another:

- the type has an _associated serialization type_, which is the type we
  write when serializing;
- the type has an _associated deserialization type_, which is the type you
  obtain after an ε-copy deserialization;
- the type can be either deep-copy or zero-copy.

There is no constraint on the associated (de)serialization type: it can be
literally anything. In general, however, one tries to have a deserialization
type that is somewhat compatible with the original type, in the sense that they
both satisfy a trait for which implementations can be written: for example,
ε-copy deserialization turns vectors/boxed slices of zero-copy types into
references to slices, so implementations can be written for `AsRef<[·]>` and
will work both on the original and the deserialized instance. And, in general,
zero-copy types deserialize into themselves. Presently the associated
serialization type is almost always `Self`, with the notable exception of
references to slices and iterators, which are serialized for convenience as
vectors/boxed slices.

Being zero-copy or deep-copy decides how the type will be treated upon
deserialization. Instances of zero-copy types are ε-copy deserialized as a
reference, whereas instances of deep-copy types are are always recursively
deserialized in allocated memory.

Sequences of zero-copy types are ε-copy deserialized using a reference to a
slice, whereas sequences of deep-copy types are deserialized in allocated memory
(to sequences of the associated deserialization types). It is important to
remark that you cannot serialize a sequence whose elements are of a type that
implements neither [`ZeroCopy`] nor [`DeepCopy`], even if in that case
ε-considers the type as deep-copy (see the following section and the
[`CopyType`] documentation for a deeper explanation).

Logically, zero-copy types should be deserialized to references, and this indeed
happens in most cases, and certainly in the derived code: however, _primitive
types are always fully deserialized_. There are two reasons behind this
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
recursively, field by field. The basic idea in ε-serde is that if the type of a
field is a type parameter, during ε-copy deserialization the type will be
replaced with its deserialization types. Since the deserialization type is
defined recursively, replacement can happen at any depth level.

Replacement happens everywhere, including occurrences nested inside other field
types. The field-level [`#[epserde(force_full)]`
attribute](#example-pinning-a-field-with-force_full) opts a specific field out
of substitution: the field is deserialized via the full-copy path and its type
is preserved verbatim in the deserialization type.

Occurrences nested inside [`PhantomData<T>`] are transparent to the derive: they
do not contribute to replaceability, and the derive substitutes `T` natively
inside the phantom slot.

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

It this section we describe in a somewhat formal way the specification of
ε-serde. We suggest to get acquainted with the examples before reading it.

### The main traits

The two main traits of ε-serde are [`SerInner`] and [`DeserInner`], which
specify respectively how to serialize and deserialize a type. [`SerInner`] has
an associated _serialization type_ [`SerType`], which is the type that will be
actually serialized; this approach makes it possible, for example, to serialize
iterators as boxed slices. [`DeserInner`] has an associated _deserialization
type_ [`DeserType<'_>`], which has a lifetime and will be used to deserialize
instances from in-memory data, mostly referencing to such data instead of
copying it: we call this process _ε-copy deserialization_.

### Types

Within ε-serde, types are classified as [_deep-copy_] or [_zero-copy_], usually
by implementing the unsafe [`CopyType`] trait with associated type
[`CopyType::Copy`] equal to [`Deep`] or [`Zero`]; types without such an
implementation are considered deep-copy.

There is a blanket implementation for the [`DeepCopy`] trait for all types
implementing [`CopyType`] with associated type [`CopyType::Copy`] equal to
[`Deep`] and also implementing [`SerInner`] with [`SerInner::SerType`]
implementing [`TypeHash`] and [`AlignHash`] (i.e., the serialization type must
implement such traits).

There is analogously a blanket implementation for the [`ZeroCopy`] trait for all
types implementing [`CopyType`] with associated type [`CopyType::Copy`] equal to
[`Zero`] and also implementing [`Copy`], [`TypeHash`], [`AlignHash`],
[`AlignTo`] and [`SerInner`] with [`SerInner::SerType`] equal to `Self`; they
must also outlive the `'static` lifetime, and be `repr(C)`. Note that in this
case we bound the original type, rather than the serialization type,
because zero-copy types are always serialized as themselves, and all fields
of a zero-copy type are zero-copy as well.

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

### Replaceable and irreplaceable parameters

Given a type `S` with generics, a type parameter `T` is _replaceable_ if it
appears in any field of `S` that does not carry `#[epserde(force_full)]`. It is
_irreplaceable_ if it appears in any field of `S` that carries
`#[epserde(force_full)]`. Occurences in a [`PhantomData`] are not accounted for.

No type parameter can be both replaceable and irreplaceable.

The derive substitutes every replaceable `T` with `<T as
DeserInner>::DeserType<'_>` in the deserialization type, and with `<T as
SerInner>::SerType` in the serialization type. Irreplaceable parameters are
kept unchanged.

For the replacement to work, field not carring `#[epserde(force_full)]` must
have a type that is δ-_stable_ with respect to the kind of parameters in `S`,
meaning that its deserialization type is obtained by replacing the concrete
types of its type parameters with their associated deserialization types if the
are replaceable in `S`. Standard wrappers that are δ-_stable_ are `Box<T>`, `Rc<T>`,
`Arc<T>`, `Option<T>`, the `Range<T>` family, and tuples. `Vec<T>`, `Box<[T]>`,
`[T; N]`, are δ-_stable_ it only when their parameter is deep-copy, and `String`
is never. `Epserde`-derived types satisfy it for their replaceable parameters.

When the contract is not satisfied, the derive emits code that fails to
type-check at the call site, and the user can restore well-formedness by
adding `#[epserde(force_full)]` to the offending field. Adding a kind bound
(`T: DeepCopy` or `T: ZeroCopy`) on the parameter can also resolve cases
where the wrapper's δ-stability depends on `T`'s kind, as in
the example above with `MyStructParam<A: DeepCopy>`.

A field marked `#[epserde(force_full)]` is deserialized via the full-copy
path and its type is preserved verbatim in the deserialization type; the
field's type-parameter occurrences do not contribute to the replaceable set.
A field whose type contains no variable position (e.g., `Vec<i32>`,
`String`, `u32`, `[u8; 16]`) is also deserialized full-copy by default: its
slot in the deserialization type has nothing to substitute, so the derive
falls back to full-copy automatically.

[`PhantomData<T>`] is handled natively by the derive: occurrences of `T`
inside `PhantomData<T>` are transparent for the classification, but the
derive substitutes `T` inside the phantom slot of the deserialization type,
so the phantom remains consistent with however `T` is classified elsewhere in
the structure. You can also use the `bound` attribute to solve some cases
(e.g., when [`DeserType<'_, A>`] is equal to `A` — see the example above about
pinning associated types).

The fundamental idea at the basis of ε-serde is that replaceable parameters make
it possible for an instance of a deserialization type to refer to serialized
zero-copy data without copying it. For example, given a structure

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
type and on its associated deserialization type, the code will work
transparently on both types; in this case, methods should be written with the
bound `A: AsRef<[Z]>`.

### Serialization and deserialization

An ε-serde serialization process involves two types:

- `S`, the _serializable type_, which is the type of the instance you want to
  serialize. It must implement [`SerInner`] with associated type
  [`SerInner::SerType`] implementing [`TypeHash`] and [`AlignHash`] (which
  implies [`Serialize`] on `S` by a blanket implementation).

- Its associated _serialization type_ [`<S as SerInner>::SerType`].

In general the serialization type of `S` is `S`, but there is some normalization
and erasure involved (e.g., vectors become boxed slices, and some smart pointers
such as [`Rc`] are erased). Moreover, a few types that are not really
serializable have a convenience serialization type (e.g., iterators become boxed
slices).

When you invoke serialize on an instance of type `S`, ε-serde writes a magic
cookie containing version information, a [type hash] which is derived from
[`S::SerType`], and which represents the definition of the type (copy type,
field names, types, etc.), an [alignment hash] which is derived from the
alignment of [`S::SerType`] (essentially, recording where padding had been
inserted in the zero-copy parts of the type), some debug information, and then
recursively the data contained in the instance.

An ε-serde deserialization process involves instead three types:

- `D`, the _deserializable type_, which must implement [`DeserInner`], and again
  [`SerInner`] with associated type [`SerInner::SerType`] implementing
  [`TypeHash`] and [`AlignHash`], so the blanket implementation for
  [`Deserialize`] applies. This is the type on which deserialization methods are
  invoked.

- The associated _serialization type_ [`D::SerType`].

- The associated _deserialization type_ [`D::DeserType<'_>`].

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

There are now two types of deserialization:

- [`deserialize_full`] performs _full-copy deserialization_, which reads recursively
  the serialized data from a [`Read`] and builds an instance of `D`. This is
  basically a standard deserialization, except that it is usually much faster if
  you have large sequences of zero-copy types, as they are deserialized in a
  single [`read_exact`].

- [`deserialize_eps`] perform _ε-copy deserialization_, which accesses the
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

- if `T` is a deep-copy concrete type obtained by resolving the type parameters
  `P₀`, `P₁`, `P₂`, … of a type definition (struct or enum) to concrete types
  `T₀`, `T₁`, `T₂`, …, then the deserialization type is obtained by resolving
  each replaceable type parameter `Pᵢ` with the deserialization type of `Tᵢ`
  instead. (Note that the first rule still applies, so if `Tᵢ` is zero-copy
  its deserialization type is `&Tᵢ`.). See [Replaceable and irreplaceable
  parameters](#replaceable-and-irreplaceable-parameters) for the definition
  of replaceable parameters.

Finally, a deep-copy type is δ-_stable_ (“stable by deserialization“) if all its
type parameters are replaceable. This means that (de)serialization type is
obtained by replacing all type parameters with their (de)serialization types:
said otherwise, the projection to the (de)serialization type and the type
constructor _commute_.

For standard types, we have:

- all primitive types, such as `u8`, `i32`, `f64`, `char`, `bool`, etc., `()`,
  and `PhantomData<T>` are zero-copy and their (de)serialization type is
  themselves; note however that when `T` is a replaceable type parameter,
  the `Epserde` derive substitutes `T` inside `PhantomData<T>` natively,
  so the ε-deserialized form carries `PhantomData<T::DeserType<'_>>`;

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

- ranges and `ControlFlow<B, C>` behave like user-defined deep-copy types;

- `Box<T>`, `Rc<T>`, and `Arc<T>`, for sized `T`, are deep-copy, and their
  serialization/deserialization type are the same of `T` (e.g., they are
  _erased_).

Note that the normalization and erasure rule give some latitude in the choice of
the deserializable type: for example, if you serialized a `Vec<T>`, you can
deserialize it fully as a `Box<[T]>`, or an `Rc<Box<[T]>>`, or ε-copy
deserialize it as an `Arc<&[T]>`, as all these types have the same serialization
type `Box<[T]>`.

We can describe the replacements leading to the deserialization type in a
non-recursive way as follows: consider the syntax tree of the type `D`, in which
the root, labeled by `D`, is connected to the root of the syntax trees of
its fields, and each children is further labeled by the name of the field.
Replacement happens in two cases:

- There is a path starting at the root, traversing only fields whose type is a
  replaceable parameter, and ending at node that is a vector/boxed slice/array
  whose elements are zero-copy: it will be replaced with a reference to a
  slice.

- There is a _shortest_ path starting at the root, traversing only fields whose
  type is a replaceable parameter, and ending at a node that is zero-copy: it
  will be replaced with a reference to the same type.

Note the shortest-path condition: this is necessary because when you reach a
zero-copy type the recursion in the definition of the deserialization type
stops. Note also that if `D` is zero-copy the empty path satisfies the
second condition, and indeed `D::DeserType<'_>` is `&D`.

## Derived and hand-made implementations

We strongly suggest using the procedural macro [`Epserde`] to make your own
types serializable and deserializable. Just invoking the macro on your structure
will make it fully functional with ε-serde. The attribute `#[epserde(zero_copy)]`
can be used to make a structure zero-copy, albeit it must satisfy [a few
prerequisites].

The macro provides also an #[`epserde(bound(ser = ..., deser = ...)`] attribute,
which can be used to add trait bounds to the generated code (see the example
above on pinning associated types).

You can also implement manually the traits [`CopyType`], [`AlignTo`],
[`TypeHash`], [`AlignHash`], [`SerInner`], and [`DeserInner`], but
the process is error-prone, and you must be fully aware of ε-serde's
conventions. The procedural macro [`TypeInfo`] can be used to generate
automatically at least [`CopyType`], [`AlignTo`], [`TypeHash`], and
[`AlignHash`] automatically.

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
[`AlignTo`]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.AlignTo.html
[alignment hash]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.AlignTo.html
[`TypeHash`]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.TypeHash.html
[`AlignHash`]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.AlignHash.html
[type hash]: https://docs.rs/epserde/latest/epserde/traits/type_info/trait.TypeHash.html
[`DeserInner`]: https://docs.rs/epserde/latest/epserde/deser/trait.DeserInner.html
[`Deserialize`]: https://docs.rs/epserde/latest/epserde/deser/trait.Deserialize.html
[`SerInner`]: https://docs.rs/epserde/latest/epserde/ser/trait.SerInner.html
[`Serialize`]: https://docs.rs/epserde/latest/epserde/ser/trait.Serialize.html
[`TypeInfo`]: https://docs.rs/epserde/latest/epserde/derive.TypeInfo.html
[`Epserde`]: https://docs.rs/epserde/latest/epserde_derive/derive.Epserde.html
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
[`DeserType<'_,B>`]: https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html
[`DeserType<'_,A>`]: https://docs.rs/epserde/latest/epserde/deser/type.DeserType.html
[`sux`]: http://crates.io/sux/
[serde]: https://serde.rs/
[Abomonation]: https://crates.io/crates/abomonation
[rkiv]: https://crates.io/crates/rkyv/
[zerovec]: https://crates.io/crates/zerovec
[mmap_rs]: https://crates.io/crates/mmap-rs
[`MemDbg`]: https://docs.rs/mem_dbg/latest/mem_dbg/trait.MemDbg.html
[`MemSize`]: https://docs.rs/mem_dbg/latest/mem_dbg/trait.MemSize.html
[`PhantomData`]: https://doc.rust-lang.org/std/marker/struct.PhantomData.html
[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[`SerIter`]: https://docs.rs/epserde/latest/epserde/impls/iter/struct.SerIter.html
[`PhantomDeserData`]: https://docs.rs/epserde/latest/epserde/struct.PhantomDeserData.html
[`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
[`Rc`]: https://doc.rust-lang.org/std/rc/struct.Rc.html
[`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
[`pointer`]: https://docs.rs/epserde/latest/epserde/impls/pointer/index.html
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
[`sux-rs`]: https://crates.io/crates/sux-rs
[`CopyType::Copy`]: https://docs.rs/epserde/latest/epserde/traits/copy_type/trait.CopyType.html#associatedtype.Copy
[`String`]: https://doc.rust-lang.org/std/string/struct.String.html
