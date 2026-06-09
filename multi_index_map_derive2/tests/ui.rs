#[test]
fn rejects_unsupported_and_malformed_inputs() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
