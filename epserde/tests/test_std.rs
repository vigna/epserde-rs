use epserde::prelude::*;


fn test_generic<T>(s: T) 
where
    T: Serialize + Deserialize + PartialEq + core::fmt::Debug,
    for<'a> <T as DeserializeInner>::DeserType<'a>: PartialEq<T> + core::fmt::Debug,
{
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);

        let mut schema = s.serialize_with_schema(&mut cursor).unwrap();
        schema.0.sort_by_key(|a| a.offset);

        cursor.set_position(0);
        let full_copy = <T>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
        assert_eq!(s, full_copy);

        let full_copy = <T>::deserialize_eps(&v).unwrap();
        assert_eq!(full_copy, s);

        let _ = schema.to_csv();
        let _ = schema.debug(&v);
    }
    {
        let mut v = vec![];
        let mut cursor = std::io::Cursor::new(&mut v);
        s.serialize(&mut cursor).unwrap();

        cursor.set_position(0);
        let full_copy = <T>::deserialize_full(&mut std::io::Cursor::new(&v)).unwrap();
        assert_eq!(s, full_copy);

        let full_copy = <T>::deserialize_eps(&v).unwrap();
        assert_eq!(full_copy, s);
    }
}

#[test]
fn test_range() {
    test_generic::<std::ops::Range<i32>>(0..10);

    #[derive(Epserde, PartialEq, Debug)]
    struct Data(std::ops::Range<i32>);
    test_generic(Data(0..10));
    
}