/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::*;

#[derive(MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
/// Create a new type around `Vec<Vec<T>>` because for orphan rule you can't
/// implement `SerializeInner` and the other traits directly.
struct Vec2D<T> {
    data: Vec<Vec<T>>,
}

/// But add a deref (or AsRef) to be able to use it as a `Vec<Vec<T>>`.
impl<T> std::ops::Deref for Vec2D<T> {
    type Target = Vec<Vec<T>>;
    fn deref(&self) -> &Vec<Vec<T>> {
        &self.data
    }
}

/// Implement the serialization. `IsEpCopy` is needed so we can safely
/// deserialize as slice the inner pieces.
impl<T: SerializeInner + IsZeroCopy + TypeName> SerializeInner for Vec2D<T> {
    /// This type cannot be serialized just by writing its bytes
    const WRITE_ALL_OPTIMIZATION: bool = false;
    /// We will read back this as a vec of slices
    type SerType<'a> = Vec<&'a [T::SerType<'a>]>;

    fn _serialize_inner<F: WriteWithPosNoStd>(&self, mut backend: F) -> Result<F> {
        // write the number of sub-fields
        backend = backend.add_field("len", &self.data.len())?;
        for (i, sub_vec) in self.data.iter().enumerate() {
            // serialize each sub-vector
            backend = backend.add_field(&format!("sub_vec_{}", i), sub_vec)?;
        }

        Ok(backend)
    }
}

/// Implement the full-copy deserialization
impl<T> DeserializeInner for Vec2D<T>
where
    Vec<T>: DeserializeInner,
{
    #[inline(always)]
    fn deserialize_inner<'a>(
        backend: Cursor<'a>,
    ) -> core::result::Result<(Self, Cursor<'a>), DeserializeError> {
        // read the len
        let (len, mut backend) = usize::deserialize_inner(backend)?;
        let mut data = Vec::with_capacity(len);
        // deserialize every subvector
        for _ in 0..len {
            let (sub_vec, tmp) = <Vec<T>>::deserialize_inner(backend)?;
            backend = tmp;
            data.push(sub_vec);
        }

        Ok((Vec2D { data }, backend))
    }
}

/// Implement the zero-copy deserialization
impl<T: DeserializeZeroCopyInner + IsZeroCopy + 'static> DeserializeZeroCopyInner for Vec2D<T>
where
    Vec<T>: DeserializeZeroCopyInner,
{
    /// This is the return type of the zero-copy deserialization.
    /// It HAS TO match the `SerType` of the serialization.
    type DesType<'a> = Vec<<Vec<T> as DeserializeZeroCopyInner>::DesType<'a>>;

    fn deserialize_zc_inner<'a>(
        backend: Cursor<'a>,
    ) -> std::result::Result<(Self::DesType<'a>, Cursor<'a>), DeserializeError> {
        // read the len
        let (len, mut backend) = usize::deserialize_inner(backend)?;
        let mut data = Vec::with_capacity(len);
        // deserialize every subvector but using zero-copy!
        for _ in 0..len {
            let (sub_vec, tmp) = <Vec<T>>::deserialize_zc_inner(backend)?;
            backend = tmp;
            data.push(sub_vec);
        }

        Ok((data, backend))
    }
}

#[derive(Serialize, Deserialize, MemSize, TypeName, Debug, PartialEq, Eq, Default, Clone)]
/// Random struct we will use to test the nested serialization and deserialization.
struct Data<A> {
    a: A,
    test: isize,
}

fn main() {
    // create a new value to serialize
    let data = Data {
        a: Vec2D {
            data: vec![vec![0x89; 6]; 9],
        },
        test: -0xbadf00d,
    };

    // create an aligned vector to serialize into so we can do a zero-copy
    // deserialization safely
    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    // wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // serialize
    let mut schema = data.serialize_with_schema(&mut buf).unwrap();
    // sort the schema by offset so we can print it in order
    schema.0.sort_by_key(|a| a.offset);
    let buf = buf.into_inner();
    println!("{}", schema.debug(buf));

    // do a full-copy deserialization
    let data1 = <Data<Vec2D<i32>>>::deserialize(&v).unwrap();
    println!("{:02x?}", data1);

    println!("\n");

    // do a zero-copy deserialization
    let data2 = <Data<Vec2D<i32>>>::deserialize_eps_copy(&v).unwrap();
    println!("{:x?}", data2);
}
