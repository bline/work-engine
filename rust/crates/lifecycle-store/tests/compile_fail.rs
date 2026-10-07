#[test]
fn public_code_cannot_forge_an_entry_permit() {
    trybuild::TestCases::new().compile_fail("tests/ui/forge_entry_permit.rs");
}
