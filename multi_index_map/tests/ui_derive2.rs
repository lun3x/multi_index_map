#[test]
fn generated_map_hides_direct_internal_members() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui_derive2/direct_internals.rs");
    tests.compile_fail("tests/ui_derive2/generated_names.rs");
}
