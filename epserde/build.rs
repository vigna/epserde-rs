use std::fmt::Write;

fn main() {
    let readme_file = env!("CARGO_PKG_README");
    let mut readme = String::from_utf8(std::fs::read(readme_file).unwrap()).unwrap();
    readme.write_str("IT WORKS").unwrap();
    dbg!(&readme);
    println!("cargo:rerun-if-changed={readme_file}");
    println!("cargo:rustc-env=README={readme}");
}
