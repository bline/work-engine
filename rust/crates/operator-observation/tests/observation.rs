use operator_observation::{Observation, Snapshot, collect, terminal_text};
use serde_json::json;
const ID: &str = "11111111-1111-4111-8111-111111111111";
fn owner(id: &str) -> serde_json::Value {
    json!({"sessionId":id,"host":"codex","sessionTitle":null,"entries":[]})
}
#[test]
fn missing_sources_are_unavailable_and_known_empty_is_available() {
    let ids = vec![ID.into(), "22222222-2222-4222-8222-222222222222".into()];
    let snapshot = collect(&"a".repeat(64), "Selected run", &ids, |args| {
        assert_eq!(args["tail"], 10);
        assert!(
            !args["roles"]
                .as_array()
                .unwrap()
                .contains(&json!("reasoning"))
        );
        if args["session_id"] == ID {
            Ok(owner(ID))
        } else {
            Err("secret internal diagnostic")
        }
    })
    .unwrap();
    assert!(
        matches!(&snapshot.sessions[0].observation,Observation::Available{entries,..} if entries.is_empty())
    );
    assert_eq!(
        snapshot.sessions[1].observation,
        Observation::Unavailable {
            code: "source_unavailable".into()
        }
    );
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("secret internal diagnostic")
    );
}
#[test]
fn wrong_identity_or_unsupported_payload_cannot_become_observed_activity() {
    for value in [
        owner("22222222-2222-4222-8222-222222222222"),
        json!({"sessionId":ID,"host":"unknown","entries":[]}),
    ] {
        let s = collect(&"a".repeat(64), "Selected run", &[ID.into()], |_| {
            Ok(value.clone())
        })
        .unwrap();
        assert_eq!(
            s.sessions[0].observation,
            Observation::Unavailable {
                code: "invalid_owner_observation".into()
            }
        );
    }
}
#[test]
fn consumer_refuses_unknown_versions_scope_changes_and_duplicate_identity() {
    let s = collect(&"a".repeat(64), "Selected run", &[ID.into()], |_| {
        Ok(owner(ID))
    })
    .unwrap();
    let bytes = serde_json::to_vec(&s).unwrap();
    assert_eq!(Snapshot::parse(&bytes).unwrap(), s);
    for field in ["schema", "scope_id", "label"] {
        let mut v = serde_json::to_value(&s).unwrap();
        v[field] = json!("changed");
        assert!(Snapshot::parse(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    assert!(
        collect(
            &"a".repeat(64),
            "Selected run",
            &[ID.into(), ID.into()],
            |_| panic!()
        )
        .is_err()
    );
}
#[test]
fn terminal_transcript_cannot_emit_control_sequences() {
    assert_eq!(terminal_text("safe\x1b[2J\r\nnext\x07"), "safe [2J \nnext ");
}
#[test]
fn shared_fixture_is_strictly_accepted() {
    let s = Snapshot::parse(include_bytes!("fixtures/observation.json")).unwrap();
    assert_eq!(s.sessions.len(), 2);
    assert!(matches!(
        s.sessions[1].observation,
        Observation::Unavailable { .. }
    ));
}

#[test]
fn nullable_wire_fields_must_remain_explicit_for_both_consumers() {
    let fixture = include_bytes!("fixtures/observation.json");
    for field in ["title", "tool", "timestamp"] {
        let mut value: serde_json::Value = serde_json::from_slice(fixture).unwrap();
        let observation = &mut value["sessions"][0]["observation"];
        if field == "title" {
            observation.as_object_mut().unwrap().remove(field);
        } else {
            observation["entries"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
        }
        assert!(Snapshot::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
