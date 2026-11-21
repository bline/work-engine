//! #6308 synthetic Host→Org correspondence exercise. Compiled only in tests.
//! No publisher, real observation, custody class, checked result or entrypoint.

use serde::Deserialize;
use serde_json::{Value, json};
use slice_campaign::codec::{campaign_digest, raw_sha256};

use super::{
    HostedInitialTransport, ReportedHostedInitialTransport,
    tests::{ROUTE, projection},
};

const SCHEMA: &str = "d7-offline-observation-v1";
const METHOD: &str = "synthetic-observer-v1";
const MAX_OBSERVATION_BYTES: usize = 16 * 1024;

struct Expected<'a> {
    transport: &'a HostedInitialTransport,
    actor: &'a str,
    channel: &'a str,
    exchange: &'a str,
    method: &'a str,
    bytes: &'a Artifacts<'a>,
}

// Independently held synthetic artifact/readback bytes, not fields copied out of
// the observation. Terminal and settlement are opaque in this fixture only.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifacts<'a> {
    bootstrap: &'a str,
    executable: &'a str,
    task: &'a str,
    response: &'a str,
    answer: &'a str,
    terminal: &'a str,
    settlement: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Observation {
    fixture_schema: String,
    binding_sha256: String,
    job_id: String,
    generation: u64,
    route_revision: String,
    method: String,
    events: Vec<Cut>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Source {
    ObserverFixture,
    ActorAssertion,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Observed {
    Observed,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Release {
    Sent,
    ConfirmedNoSend,
    UnknownAfterPossibleSend,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Terminal {
    Completed,
    Failed,
    UnresolvedSettlement,
}

#[derive(Deserialize)]
#[serde(
    tag = "cut",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Cut {
    PublicBootstrap {
        source: Source,
        actor: String,
        channel: String,
        seq: u64,
        outcome: Observed,
        public_probe_sha256: String,
        executable_sha256: String,
        locked_fixture: bool,
    },
    TaskRelease {
        source: Source,
        actor: String,
        channel: String,
        seq: u64,
        outcome: Release,
        bootstrap_seq: u64,
        task_sha256: String,
    },
    Response {
        source: Source,
        actor: String,
        channel: String,
        seq: u64,
        outcome: Observed,
        release_seq: u64,
        exchange: String,
        response_sha256: String,
    },
    Answer {
        source: Source,
        actor: String,
        channel: String,
        seq: u64,
        outcome: Observed,
        response_seq: u64,
        response_sha256: String,
        answer_sha256: String,
    },
    Terminal {
        source: Source,
        actor: String,
        channel: String,
        seq: u64,
        outcome: Terminal,
        answer_seq: u64,
        answer_sha256: String,
        terminal_sha256: String,
        settlement_sha256: Option<String>,
    },
}

impl Cut {
    fn frame(&self) -> (&Source, &str, &str, u64) {
        match self {
            Self::PublicBootstrap {
                source,
                actor,
                channel,
                seq,
                ..
            }
            | Self::TaskRelease {
                source,
                actor,
                channel,
                seq,
                ..
            }
            | Self::Response {
                source,
                actor,
                channel,
                seq,
                ..
            }
            | Self::Answer {
                source,
                actor,
                channel,
                seq,
                ..
            }
            | Self::Terminal {
                source,
                actor,
                channel,
                seq,
                ..
            } => (source, actor, channel, *seq),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Disposition {
    FixtureCorrespondence,
    ConfirmedNoSend,
    HeldPossibleSend,
    FailedTerminal { settled: bool },
    HeldUnresolvedSettlement,
}

#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Bound,
    UnestablishedCustody,
    Shape,
    Transport,
    Method,
    UnobservedCut,
    ActorOrChannel,
    Order,
    Bootstrap,
    Release,
    Response,
    Answer,
    Terminal,
    Settlement,
}

fn is_digest(reported: &str, bytes: &str) -> bool {
    reported == raw_sha256(bytes.as_bytes())
}

// The test harness supplies the byte pin out of band. It can explicitly re-pin a
// negative to reach semantic checks. Nothing in the document can supply this pin.
// Ok means a well-bound fixture disposition, NOT positive correspondence.
fn read(
    expected: &Expected<'_>,
    raw: &[u8],
    fixture_pin: Option<&str>,
) -> Result<Disposition, Refusal> {
    if raw.len() > MAX_OBSERVATION_BYTES {
        return Err(Refusal::Bound);
    }
    if fixture_pin != Some(raw_sha256(raw).as_str()) {
        return Err(Refusal::UnestablishedCustody);
    }
    // Decode directly into closed structs, not into a Value that could erase
    // duplicate fields before typed parsing.
    let record: Observation = serde_json::from_slice(raw).map_err(|_| Refusal::Shape)?;
    if record.fixture_schema != SCHEMA {
        return Err(Refusal::Shape);
    }
    if expected.transport.job_id().strip_prefix("review-") != Some(record.binding_sha256.as_str()) {
        return Err(Refusal::Transport);
    }
    expected
        .transport
        .compare_report(&ReportedHostedInitialTransport {
            job_id: record.job_id,
            generation: record.generation,
            route_revision: record.route_revision,
        })
        .map_err(|_| Refusal::Transport)?;
    if record.method != expected.method || record.method != METHOD {
        return Err(Refusal::Method);
    }
    for (index, cut) in record.events.iter().enumerate() {
        let (source, actor, channel, seq) = cut.frame();
        if *source != Source::ObserverFixture {
            return Err(Refusal::UnobservedCut);
        }
        if actor != expected.actor || channel != expected.channel {
            return Err(Refusal::ActorOrChannel);
        }
        if seq != index as u64 + 1 {
            return Err(Refusal::Order);
        }
    }
    let [
        Cut::PublicBootstrap {
            outcome: Observed::Observed,
            public_probe_sha256,
            executable_sha256,
            locked_fixture,
            ..
        },
        Cut::TaskRelease {
            outcome: release,
            bootstrap_seq: 1,
            task_sha256,
            ..
        },
        tail @ ..,
    ] = record.events.as_slice()
    else {
        return Err(Refusal::Order);
    };
    let bytes = expected.bytes;
    if !locked_fixture
        || !is_digest(public_probe_sha256, bytes.bootstrap)
        || !is_digest(executable_sha256, bytes.executable)
    {
        return Err(Refusal::Bootstrap);
    }
    if !is_digest(task_sha256, bytes.task) {
        return Err(Refusal::Release);
    }
    match release {
        Release::ConfirmedNoSend if tail.is_empty() => return Ok(Disposition::ConfirmedNoSend),
        Release::UnknownAfterPossibleSend if tail.is_empty() => {
            return Ok(Disposition::HeldPossibleSend);
        }
        Release::Sent => {}
        _ => return Err(Refusal::Order),
    }
    let [
        Cut::Response {
            outcome: Observed::Observed,
            release_seq: 2,
            exchange,
            response_sha256,
            ..
        },
        Cut::Answer {
            outcome: Observed::Observed,
            response_seq: 3,
            response_sha256: prior_response,
            answer_sha256,
            ..
        },
        Cut::Terminal {
            outcome,
            answer_seq: 4,
            answer_sha256: prior_answer,
            terminal_sha256,
            settlement_sha256,
            ..
        },
    ] = tail
    else {
        return Err(Refusal::Order);
    };
    if exchange != expected.exchange || !is_digest(response_sha256, bytes.response) {
        return Err(Refusal::Response);
    }
    if prior_response != response_sha256 || !is_digest(answer_sha256, bytes.answer) {
        return Err(Refusal::Answer);
    }
    if prior_answer != answer_sha256 || !is_digest(terminal_sha256, bytes.terminal) {
        return Err(Refusal::Terminal);
    }
    let settled = settlement_sha256
        .as_deref()
        .is_some_and(|sha| is_digest(sha, bytes.settlement));
    match outcome {
        Terminal::Completed if settled => Ok(Disposition::FixtureCorrespondence),
        // Failure can be recorded while settlement is still unresolved. A wrong
        // supplied settlement digest is a contradiction, not an absent readback.
        Terminal::Failed if settled || settlement_sha256.is_none() => {
            Ok(Disposition::FailedTerminal { settled })
        }
        Terminal::UnresolvedSettlement if settlement_sha256.is_none() => {
            Ok(Disposition::HeldUnresolvedSettlement)
        }
        _ => Err(Refusal::Settlement),
    }
}

const ADAPTED_EXPECTED: &[u8] = include_bytes!("../tests/fixtures/hosted-custody/expected.json");
const ADAPTED_OBSERVATION: &[u8] =
    include_bytes!("../tests/fixtures/hosted-custody/observation.json");
const ORIGINAL_HOST_OBSERVATION: &[u8] =
    include_bytes!("../tests/fixtures/hosted-custody/prior-observation.json");
const ADAPTED_EXPECTED_PIN: &str =
    "b74d56c8872ede85c11aad91e35086cd7fde5939ce3f9f4af2e05657e329245d";
const ADAPTED_OBSERVATION_PIN: &str =
    "88959ac2b9b043b05378b95134c4db801bbf31c5fa028b6f716cdbe1e634fbf5";
const OLD_PIN: &str = "251aacdb152dd526329d6ca7a3f4aea835a60de56d11973b12a8a4df563f1454";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AdaptedExpected<'a> {
    fixture_schema: &'a str,
    transport_input: Value,
    binding_sha256: &'a str,
    job_id: &'a str,
    generation: u64,
    route_revision: &'a str,
    expected_method: &'a str,
    expected_actor: &'a str,
    expected_channel: &'a str,
    expected_exchange: &'a str,
    expected_executable_sha256: &'a str,
    expected_task_sha256: &'a str,
    #[serde(borrow)]
    byte_fixtures: Artifacts<'a>,
}

fn with_expected<T>(check: impl FnOnce(Expected<'_>) -> T) -> T {
    assert_eq!(raw_sha256(ADAPTED_EXPECTED), ADAPTED_EXPECTED_PIN);
    let input: AdaptedExpected<'_> = serde_json::from_slice(ADAPTED_EXPECTED).unwrap();
    assert_eq!(input.fixture_schema, "d7-offline-expected-v1");
    // Fixture owner supplies these separately from the returned observation.
    // Use the real projection codec, not its reported job name or input hash.
    let transport =
        HostedInitialTransport::derive(&projection("root-a", "attempt-a"), ROUTE).unwrap();
    assert_eq!(
        campaign_digest(&input.transport_input).unwrap(),
        input.binding_sha256
    );
    assert_eq!(
        transport.job_id().strip_prefix("review-"),
        Some(input.binding_sha256)
    );
    assert_eq!(transport.job_id(), input.job_id);
    assert_eq!(transport.generation(), input.generation);
    assert_eq!(transport.route_revision(), input.route_revision);
    assert!(is_digest(
        input.expected_executable_sha256,
        input.byte_fixtures.executable
    ));
    assert!(is_digest(
        input.expected_task_sha256,
        input.byte_fixtures.task
    ));
    check(Expected {
        transport: &transport,
        actor: input.expected_actor,
        channel: input.expected_channel,
        exchange: input.expected_exchange,
        method: input.expected_method,
        bytes: &input.byte_fixtures,
    })
}

fn from_repinned_control(expected: &Expected<'_>, record: &Value) -> Result<Disposition, Refusal> {
    let raw = serde_json::to_vec(record).unwrap();
    read(expected, &raw, Some(&raw_sha256(&raw)))
}

#[test]
fn observation_adaptation_changes_only_the_transport_envelope() {
    assert_eq!(raw_sha256(ORIGINAL_HOST_OBSERVATION), OLD_PIN);
    assert_eq!(raw_sha256(ADAPTED_OBSERVATION), ADAPTED_OBSERVATION_PIN);
    let mut original: Value = serde_json::from_slice(ORIGINAL_HOST_OBSERVATION).unwrap();
    let mut adapted: Value = serde_json::from_slice(ADAPTED_OBSERVATION).unwrap();
    for field in ["bindingSha256", "jobId"] {
        let old = original.as_object_mut().unwrap().remove(field).unwrap();
        let new = adapted.as_object_mut().unwrap().remove(field).unwrap();
        assert_ne!(old, new);
    }
    assert_eq!(original, adapted);
}

#[test]
fn org_adapted_pair_matches_only_this_applied_transport() {
    with_expected(|expected| {
        assert_eq!(
            read(
                &expected,
                ADAPTED_OBSERVATION,
                Some(ADAPTED_OBSERVATION_PIN)
            ),
            Ok(Disposition::FixtureCorrespondence)
        );
        assert_eq!(
            read(&expected, ORIGINAL_HOST_OBSERVATION, Some(OLD_PIN)),
            Err(Refusal::Transport)
        );
        let second =
            HostedInitialTransport::derive(&projection("root-b", "attempt-a"), ROUTE).unwrap();
        let other_expected = Expected {
            transport: &second,
            ..expected
        };
        assert_eq!(
            read(
                &other_expected,
                ADAPTED_OBSERVATION,
                Some(ADAPTED_OBSERVATION_PIN)
            ),
            Err(Refusal::Transport)
        );
        // Separately bound synthetic job, keeping every fixture artifact byte
        // equal. Its observation is minted by this control, not the Host.
        let mut other: Value = serde_json::from_slice(ADAPTED_OBSERVATION).unwrap();
        other["jobId"] = json!(second.job_id());
        other["bindingSha256"] = json!(second.job_id().strip_prefix("review-").unwrap());
        assert_eq!(
            from_repinned_control(&other_expected, &other),
            Ok(Disposition::FixtureCorrespondence)
        );
        assert_eq!(
            from_repinned_control(&expected, &other),
            Err(Refusal::Transport)
        );
    });
}

#[test]
fn unpinned_oversized_duplicate_or_unknown_record_does_not_enter_comparison() {
    with_expected(|expected| {
        assert_eq!(
            read(&expected, ADAPTED_OBSERVATION, None),
            Err(Refusal::UnestablishedCustody)
        );
        let mut boundary = ADAPTED_OBSERVATION.to_vec();
        boundary.resize(MAX_OBSERVATION_BYTES, b' ');
        assert_eq!(
            read(&expected, &boundary, Some(ADAPTED_OBSERVATION_PIN)),
            Err(Refusal::UnestablishedCustody)
        );
        assert_eq!(
            read(&expected, &boundary, Some(&raw_sha256(&boundary))),
            Ok(Disposition::FixtureCorrespondence)
        );
        boundary.push(b' ');
        assert_eq!(
            read(&expected, &boundary, Some(&raw_sha256(&boundary))),
            Err(Refusal::Bound)
        );
        let text = std::str::from_utf8(ADAPTED_OBSERVATION).unwrap();
        for (needle, inserted) in [
            ("\"generation\":", "\"generation\":1,\"generation\":"),
            ("\"seq\":", "\"seq\":1,\"seq\":"),
            ("\"cut\":", "\"cut\":\"public_bootstrap\",\"cut\":"),
            ("\"source\":", "\"unexpected\":1,\"source\":"),
        ] {
            let bad = text.replacen(needle, inserted, 1);
            assert_ne!(bad, text);
            assert_eq!(
                read(&expected, bad.as_bytes(), Some(&raw_sha256(bad.as_bytes()))),
                Err(Refusal::Shape),
                "{needle}"
            );
        }
    });
}

#[test]
fn individual_cut_bindings_cannot_be_replaced_by_assertions_or_crossed_values() {
    with_expected(|expected| {
        let original: Value = serde_json::from_slice(ADAPTED_OBSERVATION).unwrap();
        for (pointer, different, refused) in [
            ("/bindingSha256", json!("wrong-binding"), Refusal::Transport),
            (
                "/fixtureSchema",
                json!("unsupported-schema"),
                Refusal::Shape,
            ),
            ("/jobId", json!("review-other"), Refusal::Transport),
            ("/generation", json!(2), Refusal::Transport),
            ("/routeRevision", json!("other-route"), Refusal::Transport),
            ("/method", json!("other-method"), Refusal::Method),
            (
                "/events/0/source",
                json!("actor_assertion"),
                Refusal::UnobservedCut,
            ),
            (
                "/events/2/actor",
                json!("other-actor"),
                Refusal::ActorOrChannel,
            ),
            (
                "/events/3/channel",
                json!("other-channel"),
                Refusal::ActorOrChannel,
            ),
            ("/events/0/lockedFixture", json!(false), Refusal::Bootstrap),
            (
                "/events/0/publicProbeSha256",
                json!("other"),
                Refusal::Bootstrap,
            ),
            ("/events/1/bootstrapSeq", json!(2), Refusal::Order),
            ("/events/3/responseSeq", json!(2), Refusal::Order),
            ("/events/4/answerSeq", json!(3), Refusal::Order),
            (
                "/events/0/executableSha256",
                json!("other"),
                Refusal::Bootstrap,
            ),
            ("/events/1/taskSha256", json!("other"), Refusal::Release),
            ("/events/2/releaseSeq", json!(1), Refusal::Order),
            ("/events/2/exchange", json!("other"), Refusal::Response),
            (
                "/events/2/responseSha256",
                json!("other"),
                Refusal::Response,
            ),
            ("/events/3/responseSha256", json!("other"), Refusal::Answer),
            ("/events/3/answerSha256", json!("other"), Refusal::Answer),
            ("/events/4/answerSha256", json!("other"), Refusal::Terminal),
            (
                "/events/4/terminalSha256",
                json!("other"),
                Refusal::Terminal,
            ),
            (
                "/events/4/settlementSha256",
                json!("other"),
                Refusal::Settlement,
            ),
            (
                "/events/4/settlementSha256",
                Value::Null,
                Refusal::Settlement,
            ),
        ] {
            let mut crossed = original.clone();
            let field = crossed.pointer_mut(pointer).unwrap();
            assert_ne!(*field, different);
            *field = different;
            assert_eq!(
                from_repinned_control(&expected, &crossed),
                Err(refused),
                "{pointer}"
            );
        }
        let mut unsupported = original;
        unsupported["method"] = json!("unknown-even-if-agreed");
        let expectation = Expected {
            method: "unknown-even-if-agreed",
            ..expected
        };
        assert_eq!(
            from_repinned_control(&expectation, &unsupported),
            Err(Refusal::Method)
        );
    });
}

#[test]
fn exactly_one_in_order_chain_and_nonaccepted_dispositions_are_distinct() {
    with_expected(|expected| {
        let original: Value = serde_json::from_slice(ADAPTED_OBSERVATION).unwrap();
        let variants: [fn(&mut Vec<Value>); 3] = [
            |events| {
                events.pop();
            },
            |events| {
                events.swap(2, 3);
            },
            |events| {
                let mut duplicate = events.last().unwrap().clone();
                duplicate["seq"] = json!(6);
                events.push(duplicate);
            },
        ];
        for altered in variants {
            let mut broken = original.clone();
            altered(broken["events"].as_array_mut().unwrap());
            assert_eq!(
                from_repinned_control(&expected, &broken),
                Err(Refusal::Order)
            );
        }
        for (outcome, disposition) in [
            ("confirmed_no_send", Disposition::ConfirmedNoSend),
            ("unknown_after_possible_send", Disposition::HeldPossibleSend),
        ] {
            let mut stopped = original.clone();
            stopped["events"][1]["outcome"] = json!(outcome);
            assert_eq!(
                from_repinned_control(&expected, &stopped),
                Err(Refusal::Order)
            );
            stopped["events"].as_array_mut().unwrap().truncate(2);
            assert_eq!(from_repinned_control(&expected, &stopped), Ok(disposition));
        }
        let mut failed = original.clone();
        failed["events"][4]["outcome"] = json!("failed");
        assert_eq!(
            from_repinned_control(&expected, &failed),
            Ok(Disposition::FailedTerminal { settled: true })
        );
        let mut failed_unsettled = failed.clone();
        failed_unsettled["events"][4]
            .as_object_mut()
            .unwrap()
            .remove("settlementSha256");
        assert_eq!(
            from_repinned_control(&expected, &failed_unsettled),
            Ok(Disposition::FailedTerminal { settled: false })
        );
        failed["events"][4]["outcome"] = json!("unresolved_settlement");
        assert_eq!(
            from_repinned_control(&expected, &failed),
            Err(Refusal::Settlement)
        );
        failed["events"][4]
            .as_object_mut()
            .unwrap()
            .remove("settlementSha256");
        assert_eq!(
            from_repinned_control(&expected, &failed),
            Ok(Disposition::HeldUnresolvedSettlement)
        );
    });
}
