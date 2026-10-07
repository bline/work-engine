#[test]
fn read_client_cannot_submit_mutations() {
    trybuild::TestCases::new().compile_fail("tests/ui/read_client_mutation.rs");
}
