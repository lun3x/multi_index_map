#[test]
fn generated_types_follow_element_visibility() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/visibility_types.rs");
    tests.compile_fail("tests/ui/visibility_private_iterator.rs");
    tests.compile_fail("tests/ui/visibility_restricted_iterator.rs");
    tests.compile_fail("tests/ui/visibility_private_method.rs");
}
