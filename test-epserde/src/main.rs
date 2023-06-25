use epserde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
struct Person {
    name: u64,
    age: u128,
}

use epserde_trait::{Deserialize, Serialize};

fn main() {
    let person0 = Person {
        name: 0xdeadbeed,
        age: 0xdeadbeefdeadf00d,
    };
    let mut v = vec![0; 100];
    let mut buf = std::io::Cursor::new(&mut v);
    person0.serialize(&mut buf).unwrap();
    let (person1, _rest) = Person::deserialize(&v).unwrap();
    assert_eq!(person0, person1);
}
