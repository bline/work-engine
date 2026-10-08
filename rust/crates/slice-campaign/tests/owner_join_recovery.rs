mod support;

use serde_json::{Value, json};
use slice_campaign::Campaign;
use std::fs;

#[test]
fn schema_one_and_two_roots_are_refused_without_a_write_or_migration() {
    for historical_schema in [1, 2] {
        let (temp, repo, workspace, identity, baseline, boundary) = support::setup();
        let root = temp.path().join("campaign");
        let config = || {
            support::config(
                root.clone(),
                repo.clone(),
                workspace.clone(),
                identity.clone(),
                baseline.clone(),
                boundary.clone(),
            )
        };
        let app = Campaign::initialize(config()).unwrap();
        drop(app);
        let marker_path = root.join(".campaign-native-initial-v3");
        let mut marker: Value = serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        marker["schema_version"] = json!(historical_schema);
        let old_path = root.join(format!(".campaign-native-initial-v{historical_schema}"));
        let old_bytes = serde_json::to_vec(&marker).unwrap();
        fs::write(&old_path, &old_bytes).unwrap();
        fs::remove_file(&marker_path).unwrap();
        let database = root.join("slice-campaign.sqlite");
        let before = fs::read(&database).unwrap();
        let sidecars = [
            "slice-campaign.sqlite-wal",
            "slice-campaign.sqlite-shm",
            "slice-campaign.sqlite-journal",
        ];
        let before_sidecars: Vec<_> = sidecars
            .iter()
            .map(|name| fs::read(root.join(name)).ok())
            .collect();
        assert!(Campaign::open(config()).is_err());
        assert_eq!(fs::read(&database).unwrap(), before);
        assert_eq!(fs::read(&old_path).unwrap(), old_bytes);
        assert!(!marker_path.exists());
        assert_eq!(
            sidecars
                .iter()
                .map(|name| fs::read(root.join(name)).ok())
                .collect::<Vec<_>>(),
            before_sidecars
        );
    }
}
