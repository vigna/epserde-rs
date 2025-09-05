use epserde::prelude::*;
use std::io::Cursor;

fn main() {
    let v = vec![0, 10, 20, 30, 40];

    let mut buffer = Vec::new();
    unsafe { v.serialize(&mut buffer).unwrap() };
    let cursor = Cursor::new(&buffer);
    let mem_case = unsafe { Vec::<i32>::read_mem(cursor, buffer.len()).unwrap() };
    let v = mem_case.uncase();
    drop(mem_case);
    let _vv = v;
}
