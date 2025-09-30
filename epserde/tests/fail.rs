#[test]
#[cfg_attr(miri, ignore)]
fn fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}
