use epserde_derive::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Eq, Default)]
struct Person {
    name: u64,
    age: u128,
}

fn main() {
    println!("Hello, Derive");
}
