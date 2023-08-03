#![feature(test)]
#![cfg(test)]

use epserde::*;

type A = Vec<usize>;

#[test]
fn test_vec() {
    let a: A = vec![1, 2, 3, 4, 5];

    let len = 100;
    let mut v = unsafe {
        Vec::from_raw_parts(
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(len, 4096).unwrap()),
            len,
            len,
        )
    };
    assert!(v.as_ptr() as usize % 4096 == 0, "{:p}", v.as_ptr());
    let mut buf = std::io::Cursor::new(&mut v);

    let mut schema = a.serialize_with_schema(&mut buf).unwrap();
    schema.0.sort_by_key(|a| a.offset);
    println!("{}", schema.to_csv());

    let a1 = A::deserialize(&v).unwrap();
    println!("a1: {}", a1.type_name_val());
    assert_eq!(a, a1);

    let a2 = <A>::deserialize_eps_copy(&v).unwrap();
    println!("a2: {}", a2.type_name_val());
    assert_eq!(a, a2);

    // check that the type names are different between full serialization vs
    // zero-copy deserialization
    assert_ne!(a1.type_name_val(), a2.type_name_val());
}
