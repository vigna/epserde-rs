use epserde_derive::Serialize;
use epserde_trait::Serialize;

#[derive(Serialize, Debug, PartialEq, Eq)]
struct Person {
    name: u64,
    age: u128,
}

fn main() {
    println!("Hello, Derive");
}
