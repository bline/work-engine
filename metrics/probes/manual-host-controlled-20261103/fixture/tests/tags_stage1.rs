use manual_host_pilot_fixture::tag_counts;
use std::collections::BTreeMap;

#[test]
fn public_contract_examples() {
    assert_eq!(tag_counts(&[]), BTreeMap::new());
    assert_eq!(
        tag_counts(&["", "a", "A", "a", "猫", ""]),
        BTreeMap::from([("A".into(), 1), ("a".into(), 2), ("猫".into(), 1)])
    );
}
