use review_workflow::protocol::Command;
use serde_json::json;
#[test]
fn protocol_cannot_supply_authority_descriptors_programs_or_roots() {
    let base = json!({"version":1,"requestId":"one","operation":"execute_initial","obligationId":"review-0","operationId":"initial","expectedRevision":"a".repeat(64)});
    assert!(Command::parse(&serde_json::to_vec(&base).unwrap()).is_ok());
    for name in [
        "descriptor",
        "authority",
        "principal",
        "root",
        "executable",
        "profile",
        "selection",
        "session",
        "permit",
    ] {
        let mut altered = base.clone();
        altered[name] = json!("forged");
        assert!(
            Command::parse(&serde_json::to_vec(&altered).unwrap()).is_err(),
            "{name}"
        );
    }
    for operation in [
        "retry",
        "terminalize",
        "remediate",
        "evaluate",
        "history",
        "qualified_provider",
    ] {
        let mut altered = base.clone();
        altered["operation"] = json!(operation);
        assert!(
            Command::parse(&serde_json::to_vec(&altered).unwrap()).is_err(),
            "{operation}"
        );
    }
}
#[test]
fn command_limits_and_control_scope_are_closed() {
    assert!(Command::parse(&vec![b' '; 64 * 1024 + 1]).is_err());
    assert!(
        Command::parse(br#"{"version":1,"requestId":"s","operation":"status","authority":{}}"#)
            .is_err()
    );
    assert!(
        Command::parse(
            br#"{"version":1,"requestId":"s","operation":"status","operationId":"initial"}"#
        )
        .is_err()
    );
    assert!(Command::parse(br#"{"version":2,"requestId":"s","operation":"status"}"#).is_err());
}
