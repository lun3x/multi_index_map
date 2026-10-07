#[test]
fn generated_visibility_respects_struct_and_field_scopes() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/visibility_types.rs");
    tests.pass("tests/ui/visibility_paths.rs");
    tests.compile_fail("tests/ui/visibility_private_iterator.rs");
    tests.compile_fail("tests/ui/visibility_restricted_iterator.rs");
    tests.compile_fail("tests/ui/visibility_private_method.rs");
    tests.compile_fail("tests/ui/visibility_restricted_method.rs");
    tests.compile_fail("tests/ui/visibility_private_storage.rs");
}
