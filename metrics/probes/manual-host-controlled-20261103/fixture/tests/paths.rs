use manual_host_pilot_fixture::is_portable_relative_path;

#[test]
fn public_contract_examples() {
    for s in ["file", "dir/file.txt", "résumé/猫", "a-b/c_d"] {
        assert!(is_portable_relative_path(s), "{s:?}");
    }
    for s in [
        "", "/a", "a/", "a//b", ".", "..", "a/../b", "a/./b", "a\\b", "C:/a", "a:b", "a\0b", "a\nb",
    ] {
        assert!(!is_portable_relative_path(s), "{s:?}");
    }
}
