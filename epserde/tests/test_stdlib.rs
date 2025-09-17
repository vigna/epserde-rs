/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use epserde::prelude::*;
use maligned::A16;
use anyhow::Result;

fn test_generic<T>(s: T) -> Result<()>
where
    T: Serialize + Deserialize + PartialEq + core::fmt::Debug,
    for<'a> <T as DeserializeInner>::DeserType<'a>: PartialEq<T> + core::fmt::Debug,
{
    {
        let mut cursor = <AlignedCursor<A16>>::new();

        let mut schema = unsafe { s.serialize_with_schema(&mut cursor)? };
        schema.0.sort_by_key(|a| a.offset);

        cursor.set_position(0);
        let full_copy = unsafe {
            <T>::deserialize_full(&mut std::io::Cursor::new(&cursor.as_bytes()))?
        };
        assert_eq!(s, full_copy);

        let bytes = cursor.as_bytes();
        let full_copy = unsafe { <T>::deserialize_eps(bytes)? };
        assert_eq!(full_copy, s);

        let _ = schema.to_csv();
        let _ = schema.debug(bytes);
    }
    {
        let mut cursor = <AlignedCursor<A16>>::new();
        unsafe { s.serialize(&mut cursor)? };

        cursor.set_position(0);
        let full_copy =
            unsafe { <T>::deserialize_full(&mut std::io::Cursor::new(cursor.as_bytes()))? };
        assert_eq!(s, full_copy);

        let full_copy = unsafe { <T>::deserialize_eps(cursor.as_bytes())? };
        assert_eq!(full_copy, s);
    }
    Ok(())
}

#[test]
fn test_range() -> Result<()> {
    test_generic::<std::ops::Range<i32>>(0..10)?;

    #[derive(Epserde, PartialEq, Debug)]
    struct Data(std::ops::Range<i32>);
    test_generic(Data(0..10))?;
    Ok(())
}
