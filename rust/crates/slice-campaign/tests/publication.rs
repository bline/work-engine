#![cfg(unix)]
#[path = "support/mod.rs"]
mod support;

use slice_campaign::publication::CampaignPublication;

#[test]
fn completion_root_pins_operator_uid_and_authority_mode() {
    let (_temp, mut campaign, _identity, _selected, _config) =
        support::selected_campaign_with_profile(0, true, true);
    let root = campaign.anchored_root().join("publication-install-test");
    let digest = "a".repeat(64);
    let uid = rustix::process::getuid().as_raw();
    let installed =
        CampaignPublication::initialize(&mut campaign, &root, uid, digest.clone(), true).unwrap();
    drop(installed);
    assert!(
        CampaignPublication::open(&mut campaign, &root, uid + 1, digest.clone(), true).is_err()
    );
    assert!(CampaignPublication::open(&mut campaign, &root, uid, digest.clone(), false).is_err());
    CampaignPublication::open(&mut campaign, &root, uid, digest, true).unwrap();
}
