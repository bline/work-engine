use manual_host_pilot_fixture::{CountOverflow, merge_counts};
use std::collections::BTreeMap;

#[test]
fn public_contract_examples() {
    let a = BTreeMap::from([("a".into(), 2), ("z".into(), 0)]);
    let b = BTreeMap::from([("a".into(), 3), ("猫".into(), 1)]);
    assert_eq!(
        merge_counts(&a, &b),
        Ok(BTreeMap::from([
            ("a".into(), 5),
            ("z".into(), 0),
            ("猫".into(), 1)
        ]))
    );
    assert_eq!(a["a"], 2);
    assert_eq!(b["a"], 3);
    assert_eq!(
        merge_counts(
            &BTreeMap::from([("a".into(), u64::MAX)]),
            &BTreeMap::from([("a".into(), 1)])
        ),
        Err(CountOverflow)
    );
    assert_eq!(
        merge_counts(&BTreeMap::new(), &BTreeMap::new()),
        Ok(BTreeMap::new())
    );
}
