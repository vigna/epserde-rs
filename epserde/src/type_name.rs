use core::hash::Hash;

/// A simple trait to get the name of a type recursively solving generics.
///
/// This is closely related to [`core::any::type_name`] but as it's noted on its
/// documentation:
/// > The returned string must not be considered to be a unique identifier of a
/// > type as multiple types may map to the same type name. Similarly, there is
/// > no guarantee that all parts of a type will appear in the returned string:
/// > for example, lifetime specifiers are currently not included. In addition,
/// > the output may change between versions of the compiler.
///
/// And we need a stable way to get the name of a type for both dbg info and
/// serialization.
pub trait TypeName {
    /// Just the type name, without the module path.
    fn type_name() -> String;
    /// Hash the type, this considers the name, order, and type of the fields
    /// and the type of the struct.  
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H);

    /// Call type_name on a value
    #[inline(always)]
    fn type_name_val(&self) -> String {
        Self::type_name()
    }
    /// Call type_hash on a value
    #[inline(always)]
    fn type_hash_val<H: core::hash::Hasher>(&self, hasher: &mut H) {
        Self::type_hash(hasher)
    }
}

// Blanket impls

impl<T: TypeName + ?Sized> TypeName for &'_ T {
    #[inline(always)]
    fn type_name() -> String {
        format!("&{}", T::type_name())
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        '&'.hash(hasher);
        T::type_hash(hasher);
    }
}

// Core types

impl<T: TypeName> TypeName for Option<T> {
    #[inline(always)]
    fn type_name() -> String {
        format!("Option<{}>", T::type_name())
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "Option".hash(hasher);
        T::type_hash(hasher);
    }
}

impl<S: TypeName, E: TypeName> TypeName for Result<S, E> {
    #[inline(always)]
    fn type_name() -> String {
        format!("Result<{}, {}>", S::type_name(), E::type_name())
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "Result".hash(hasher);
        S::type_hash(hasher);
        E::type_hash(hasher);
    }
}

// Primitive types

impl<T: TypeName, const N: usize> TypeName for [T; N] {
    #[inline(always)]
    fn type_name() -> String {
        format!("[{}; {}]", T::type_name(), N)
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "[;]".hash(hasher);
        T::type_hash(hasher);
        N.hash(hasher);
    }
}

impl<T: TypeName> TypeName for [T] {
    #[inline(always)]
    fn type_name() -> String {
        format!("[{}]", T::type_name())
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "[]".hash(hasher);
        T::type_hash(hasher);
    }
}

macro_rules! impl_primitives {
    ($($ty:ty),*) => {$(
impl TypeName for $ty {
    #[inline(always)]
    fn type_name() -> String {stringify!($ty).into()}
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        stringify!($ty).hash(hasher);
    }
}
    )*};
}

impl_primitives! {
    char, bool, str, f32, f64, (),
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize
}

// Alloc related types

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

#[cfg(feature = "alloc")]
impl TypeName for String {
    #[inline(always)]
    fn type_name() -> String {
        "String".into()
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "String".hash(hasher);
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
impl<T: TypeName> TypeName for Vec<T> {
    #[inline(always)]
    fn type_name() -> String {
        format!("Vec<{}>", T::type_name())
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "Vec".hash(hasher);
        T::type_hash(hasher);
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
impl<T: TypeName + ?Sized> TypeName for Box<T> {
    #[inline(always)]
    fn type_name() -> String {
        format!("Box<{}>", T::type_name())
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "Box".hash(hasher);
        T::type_hash(hasher);
    }
}

// foreign types

#[cfg(feature = "mmap_rs")]
impl TypeName for mmap_rs::Mmap {
    #[inline(always)]
    fn type_name() -> String {
        "Mmap".into()
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "Mmap".hash(hasher);
    }
}

#[cfg(feature = "mmap_rs")]
impl TypeName for mmap_rs::MmapMut {
    #[inline(always)]
    fn type_name() -> String {
        "MmapMut".into()
    }
    #[inline(always)]
    fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
        "MmapMut".hash(hasher);
    }
}

// tuples

macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl<$($t: TypeName,)*> TypeName for ($($t,)*)
        {
            #[inline(always)]
            fn type_name() -> String {
                let mut res = "(".to_string();
                $(
                    res.push_str(&<$t>::type_name());
                )*
                res.push(')');
                res
            }
            #[inline(always)]
            fn type_hash<H: core::hash::Hasher>(hasher: &mut H) {
                "()".hash(hasher);
                let mut len = 0;
                $(
                    <$t>::type_hash(hasher);
                    len += 1;
                )*
                len.hash(hasher);
            }
        }
    };
}

macro_rules! impl_tuples_muncher {
    ($ty:ident, $($t:ident),*) => {
        impl_tuples!($ty, $($t),*);
        impl_tuples_muncher!($($t),*);
    };
    ($ty:ident) => {
        impl_tuples!($ty);
    };
    () => {};
}

impl_tuples_muncher!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
