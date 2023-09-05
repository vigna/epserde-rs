#![cfg(test)]

use epserde::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct Data<A, const Q: usize> {
    a: A,
    b: [i32; Q],
}

#[test]
fn test_mem_case() {
    // Create a new value to serialize
    let person = Data {
        a: vec![0x89; 6],
        b: [0xbadf00d; 2],
    };
    // Create an aligned vector to serialize into so we can do a zero-copy
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
    // Wrap the vector in a cursor so we can serialize into it
    let mut buf = std::io::Cursor::new(&mut v);

    // Serialize
    let _bytes_written = person.serialize(&mut buf).unwrap();

    // Do a full-copy deserialization
    let full = <Data<Vec<usize>, 2>>::deserialize_full_copy(&v).unwrap();
    assert_eq!(person, full);

    println!();

    // Do an ε-copy deserialization
    let eps = <Data<Vec<usize>, 2>>::deserialize_eps_copy(&v).unwrap();
    assert_eq!(person.a, eps.a);
    assert_eq!(person.b, eps.b);
}
