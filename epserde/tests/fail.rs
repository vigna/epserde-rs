use anyhow::Result;

#[test]
fn fail() -> Result<()> {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
    Ok(())
}
