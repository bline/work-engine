//! Default-feature tests: freezing and the exact-correspondence checks. The observations here are
//! CONSTRUCTED inputs for the checking layer; they are not native evidence. Native evidence is
//! produced only by `native_vertical.rs` (feature `fixture`).
mod common;

use common::*;
use serde_json::{Value, json};
use work_engine_review_delivery::{
    Allocation, BoundAttempt, Counters, DeliveryRefusal, FrozenDelivery, NativeObservation,
    Transport,
};
use work_engine_review_preparation::{ReviewPreparation, prepare};

fn preparation() -> ReviewPreparation {
    prepare(&candidate(), &request(None)).expect("prepares")
}

fn transport() -> Transport {
    Transport {
        attempt_id: "attempt-1".into(),
        generation: 1,
    }
}

fn frozen() -> FrozenDelivery {
    FrozenDelivery::freeze(preparation(), transport()).unwrap()
}

fn allocation() -> Allocation {
    Allocation {
        session_id: "sess-A".into(),
        sdk_version: "1.5.0".into(),
        node_version: "v-test".into(),
        model: json!({}),
        system_prompt: "SYS".into(),
        active_tools: vec![],
        loader: json!({}),
        preamble: "P".into(),
    }
}

const RESULT: &str = "{\"synthetic\":true}";

fn system() -> Value {
    json!({"role": "system", "content": "", "sections": {"preamble": "P", "cwd": "<cwd>"}})
}

fn message(role: &str, text: &str) -> Value {
    json!({"role": role, "content": [{"type": "text", "text": text}], "stopReason": "stop"})
}

/// A constructed observation that agrees with `f` in every respect.
fn good(f: &FrozenDelivery) -> NativeObservation {
    NativeObservation {
        session_id: "sess-A".into(),
        attempt_id: "attempt-1".into(),
        generation: 1,
        request_count: 1,
        context: Some(json!({"messages": [system(), message("user", f.contract())], "tools": []})),
        history: vec![
            system(),
            message("user", f.contract()),
            message("assistant", RESULT),
        ],
        active_tools: vec![],
        terminal_idle: true,
        counters: Counters::default(),
        prompt_error: None,
        system_prompt: "SYS".into(),
        loader_calls: json!({}),
    }
}

fn bound(f: &FrozenDelivery) -> BoundAttempt {
    BoundAttempt::bind(f.clone(), allocation()).unwrap()
}

#[test]
fn the_envelope_carries_the_complete_preparation_and_adds_transport_only() {
    let f = frozen();
    let envelope: Value = serde_json::from_str(f.contract()).unwrap();
    let keys: Vec<_> = envelope.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys.len(), 3, "{keys:?}");
    for key in ["label", "transport", "preparation"] {
        assert!(keys.contains(&key.to_owned()), "{key}");
    }
    assert_eq!(
        envelope["label"],
        "experimental_review_delivery_not_authority"
    );
    // Complete, not a projection: identical to the owner's own serialization.
    assert_eq!(
        envelope["preparation"],
        serde_json::to_value(preparation()).unwrap()
    );
    for part in [
        "model_context",
        "host_enforcement",
        "provenance",
        "blockers",
    ] {
        assert!(!envelope["preparation"][part].is_null(), "{part}");
    }
    assert!(
        !f.preparation().blockers.is_empty(),
        "production blockers are carried, not cleared"
    );
    assert_eq!(
        envelope["preparation"]["blockers"]
            .as_array()
            .unwrap()
            .len(),
        f.preparation().blockers.len()
    );
}

#[test]
fn freezing_is_deterministic_and_transport_changes_only_transport() {
    let a = frozen();
    let b = frozen();
    assert_eq!(a.contract(), b.contract());
    assert_eq!(a.contract_sha256(), b.contract_sha256());
    let c = FrozenDelivery::freeze(
        preparation(),
        Transport {
            attempt_id: "attempt-1".into(),
            generation: 2,
        },
    )
    .unwrap();
    assert_ne!(a.contract_sha256(), c.contract_sha256());
    let (ea, ec): (Value, Value) = (
        serde_json::from_str(a.contract()).unwrap(),
        serde_json::from_str(c.contract()).unwrap(),
    );
    assert_eq!(ea["preparation"], ec["preparation"]);
    assert_ne!(ea["transport"], ec["transport"]);
}

#[test]
fn a_fully_consistent_observation_verifies_and_returns_the_observed_text() {
    let f = frozen();
    let v = bound(&f).verify(&good(&f), RESULT).expect("verifies");
    assert_eq!(v.result_text, RESULT);
    assert_eq!(v.session_id, "sess-A");
    assert_eq!(v.contract_sha256, f.contract_sha256());
}

#[test]
fn each_wrong_identity_or_byte_is_refused_by_its_owner_check() {
    let f = frozen();
    let b = bound(&f);
    let refuse = |edit: &dyn Fn(&mut NativeObservation)| {
        let mut o = good(&f);
        edit(&mut o);
        b.verify(&o, RESULT).expect_err("must be refused")
    };
    assert!(matches!(
        refuse(&|o| o.session_id = "sess-B".into()),
        DeliveryRefusal::SessionMismatch { .. }
    ));
    assert!(matches!(
        refuse(&|o| o.attempt_id = "attempt-2".into()),
        DeliveryRefusal::AttemptMismatch { .. }
    ));
    assert!(matches!(
        refuse(&|o| o.generation = 2),
        DeliveryRefusal::GenerationMismatch {
            expected: 1,
            observed: 2
        }
    ));
    // One byte of the delivered contract differs, with a self-consistent history: still refused.
    let mut altered = f.contract().to_owned();
    altered.push(' ');
    let e = refuse(&|o| {
        o.context = Some(json!({"messages": [system(), message("user", &altered)], "tools": []}));
    });
    assert_eq!(
        e,
        DeliveryRefusal::ContractMismatch("request-boundary text")
    );
    let e = refuse(&|o| o.history[1] = message("user", &altered));
    assert_eq!(e, DeliveryRefusal::ContractMismatch("history user text"));
    // Wrong role at the context boundary.
    let e = refuse(&|o| {
        o.context =
            Some(json!({"messages": [system(), message("assistant", f.contract())], "tools": []}))
    });
    assert!(matches!(e, DeliveryRefusal::Structure(_)), "{e:?}");
    // Extra message.
    let e = refuse(&|o| {
        o.context = Some(
            json!({"messages": [system(), message("user", f.contract()), message("user", "x")], "tools": []}),
        )
    });
    assert!(matches!(e, DeliveryRefusal::Structure(_)), "{e:?}");
    // System context absent, or not carrying the fixed fixture preamble.
    let e = refuse(&|o| {
        o.context = Some(json!({"messages": [message("user", f.contract())], "tools": []}))
    });
    assert!(matches!(e, DeliveryRefusal::Structure(_)), "{e:?}");
    let e = refuse(&|o| {
        o.context = Some(
            json!({"messages": [json!({"role": "system", "sections": {"preamble": "OTHER"}}), message("user", f.contract())], "tools": []}),
        )
    });
    assert!(matches!(e, DeliveryRefusal::Structure(_)), "{e:?}");
    // Final assistant text differs from the bound synthetic result.
    assert_eq!(
        refuse(&|o| o.history[2] = message("assistant", "{\"synthetic\":false}")),
        DeliveryRefusal::ResultMismatch
    );
    let mut abnormal = message("assistant", RESULT);
    abnormal["stopReason"] = json!("error");
    let e = refuse(&|o| o.history[2] = abnormal.clone());
    assert!(matches!(e, DeliveryRefusal::History(_)), "{e:?}");
    assert!(matches!(
        refuse(&|o| o.history.truncate(2)),
        DeliveryRefusal::History(_)
    ));
    // History system context that differs from what the request boundary carried.
    let e = refuse(&|o| {
        o.history[0] =
            json!({"role": "system", "sections": {"preamble": "P", "cwd": "<elsewhere>"}})
    });
    assert!(matches!(e, DeliveryRefusal::History(_)), "{e:?}");
}

#[test]
fn toolsets_and_provider_entry_are_refused() {
    let f = frozen();
    let b = bound(&f);
    let refuse = |edit: &dyn Fn(&mut NativeObservation)| {
        let mut o = good(&f);
        edit(&mut o);
        b.verify(&o, RESULT).expect_err("must be refused")
    };
    assert!(matches!(
        refuse(&|o| o.active_tools = vec!["read".into()]),
        DeliveryRefusal::Toolset(_)
    ));
    assert!(matches!(
        refuse(&|o| o.context = Some(
            json!({"messages": [system(), message("user", f.contract())], "tools": [{"name": "read"}]})
        )),
        DeliveryRefusal::Toolset(_)
    ));
    assert_eq!(
        refuse(&|o| o.counters.ordinary_stream = 1),
        DeliveryRefusal::ProviderEntry {
            ordinary_stream: 1,
            fetch: 0
        }
    );
    assert_eq!(
        refuse(&|o| o.counters.fetch = 1),
        DeliveryRefusal::ProviderEntry {
            ordinary_stream: 0,
            fetch: 1
        }
    );
    assert_eq!(
        refuse(&|o| o.request_count = 2),
        DeliveryRefusal::RequestCount(2)
    );
    let e = BoundAttempt::bind(
        f.clone(),
        Allocation {
            active_tools: vec!["bash".into()],
            ..allocation()
        },
    )
    .expect_err("tools at allocation");
    assert!(matches!(e, DeliveryRefusal::AllocationTools(_)));
}

#[test]
fn possible_delivery_without_a_complete_observation_is_uncertain_not_refused_as_wrong() {
    let f = frozen();
    let b = bound(&f);
    let uncertain = |edit: &dyn Fn(&mut NativeObservation)| {
        let mut o = good(&f);
        edit(&mut o);
        let e = b.verify(&o, RESULT).expect_err("must not verify");
        assert!(e.is_uncertain(), "{e:?}");
    };
    uncertain(&|o| o.terminal_idle = false);
    uncertain(&|o| o.context = None);
    // An SDK error after a request was made: the send may have happened.
    uncertain(&|o| {
        o.prompt_error = Some("boom".into());
        o.request_count = 1;
    });
    // An SDK refusal before any request is a plain refusal (nothing was sent).
    let mut o = good(&f);
    o.prompt_error = Some("No API key found".into());
    o.request_count = 0;
    o.context = None;
    let e = b.verify(&o, RESULT).expect_err("refused");
    assert!(
        matches!(&e, DeliveryRefusal::PromptRefused(m) if m.contains("API key")),
        "{e:?}"
    );
    assert!(!e.is_uncertain());
}

#[test]
fn strict_wire_representation_keeps_json_like_strings_and_rejects_recursive_duplicates() {
    use work_engine_review_delivery::wire::parse;
    let raw = br#"{"text":"{\"a\":1,\"a\":2}","other":["duplicate key"]}"#;
    assert_eq!(parse(raw).unwrap()["text"], r#"{"a":1,"a":2}"#);
    for raw in [
        br#"{"a":1,"a":1}"#.as_slice(),
        br#"{"a":1,"a":2}"#,
        br#"{"a":[{"x":{},"x":null}]}"#,
    ] {
        assert!(parse(raw).unwrap_err().contains("duplicate key"));
    }
}
