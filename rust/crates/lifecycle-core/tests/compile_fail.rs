#[test]
fn domain_identity_types_are_not_interchangeable() {
    trybuild::TestCases::new().compile_fail("tests/ui/wrong_identity.rs");
}
