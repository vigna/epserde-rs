#![cfg(test)]

use epserde::*;

macro_rules! impl_test {
    ($ty:ty, $val:expr) => {{
        let len = 1024;
        let a = $val;
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

        let a1 = <$ty>::deserialize_full_copy(&v).unwrap();
        assert_eq!(a, a1);

        let a2 = <$ty>::deserialize_eps_copy(&v).unwrap();
        assert_eq!(a, a2);
    }};
}

#[test]
fn test_array_usize() {
    let a = [1, 2, 3, 4, 5];

    let len = 1024;
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

    let a1 = <[usize; 5]>::deserialize_full_copy(&v).unwrap();
    assert_eq!(a, a1);

    let a2 = <[usize; 5]>::deserialize_eps_copy(&v).unwrap();
    assert_eq!(a, *a2);
}

#[test]
fn test_vec_usize() {
    impl_test!(Vec<usize>, vec![1, 2, 3, 4, 5])
}

#[test]
fn test_box_slice_usize() {
    let a = vec![1, 2, 3, 4, 5].into_boxed_slice();

    let len = 1024;
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

    let a1 = <Box<[usize]>>::deserialize_full_copy(&v).unwrap();
    assert_eq!(a, a1.into());

    let a2 = <Box<[usize]>>::deserialize_eps_copy(&v).unwrap();
    assert_eq!(a, a2.into());
}

#[test]
fn test_box_slice_string() {
    let a = vec!["A".to_string(), "V".to_string()].into_boxed_slice();

    let len = 1024;
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

    let a1 = <Box<[String]>>::deserialize_full_copy(&v).unwrap();
    assert_eq!(a, a1);

    let a2 = <Box<[String]>>::deserialize_eps_copy(&v).unwrap();
    assert_eq!(a.len(), a2.len());
    a.iter().zip(a2.iter()).for_each(|(a, a2)| {
        assert_eq!(a, a2);
    });
}

#[test]
fn test_vec_vec_usize() {
    impl_test!(Vec<Vec<usize>>, vec![vec![1, 2, 3], vec![4, 5]])
}

#[test]
fn test_vec_array_string() {
    impl_test!(
        Vec<[String; 2]>,
        vec![
            ["a".to_string(), "b".to_string()],
            ["c".to_string(), "aasfihjasomk".to_string()]
        ]
    )
}

#[test]
fn test_vec_vec_string() {
    impl_test!(
        Vec<Vec<String>>,
        vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "aasfihjasomk".to_string()]
        ]
    )
}

#[test]
fn test_vec_vec_array_array_string() {
    impl_test!(
        Vec<Vec<[[String; 2]; 2]>>,
        vec![
            vec![[
                ["a".to_string(), "b".to_string()],
                ["c".to_string(), "d".to_string()],
            ]],
            vec![[
                ["a".to_string(), "b".to_string()],
                ["c".to_string(), "d".to_string()],
            ]],
        ]
    )
}

#[test]
fn test_vec_vec_array_array_usize() {
    impl_test!(
        Vec<Vec<[[usize; 2]; 2]>>,
        vec![vec![[[1, 2], [3, 4],]], vec![[[5, 6], [7, 8],]],]
    )
}
