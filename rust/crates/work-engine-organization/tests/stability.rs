//! 1A-1 determinism and 1A-8 golden vectors. These tests check **stability**: that the same
//! meaning gives the same bytes, and that the bytes do not drift. They do not show that the
//! bytes are right. The goldens were checked against the rules before they were frozen: the
//! derived candidates were read against the amendment 3 table and the F4 table, and
//! `golden/*.canonical.json` was re-derived independently with Python (framing with `hashlib`,
//! canonical form with `json.dumps(sort_keys=True, separators=(",", ":"))`); see the builder
//! custody record `p2-golden-check.txt`. The rule tests are in `rules.rs`.

mod common;

use common::*;
use serde_json::{Value, json};
use work_engine_organization::{Digest, DigestKind, OrganizationalRequirements, Profile, derive};

/// Two obligations, four conditions, four authorities, and every set kind with 2-4 members.
fn rich() -> Value {
    let mut c1 = condition("c1", "ob-a", "builder_projection", "builder", "owner-a");
    c1["normative"]["capabilities"] = json!(["cap-a", "cap-b", "cap-c", "cap-d"]);
    c1["normative"]["independence"] = json!({"from": ["state.builder_context"]});
    c1["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic", "openai", "other"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox", "host_process"]}
    ]);
    let mut c2 = condition(
        "c2",
        "ob-a",
        "campaign_terminalization",
        "campaign",
        "owner-a",
    );
    c2["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["anthropic", "openai"]}]);
    let c3 = condition("c3", "ob-b", "builder_projection", "builder", "owner-b");
    let c4 = condition(
        "c4",
        "ob-b",
        "campaign_terminalization",
        "campaign",
        "owner-b",
    );
    document(
        vec![obligation("ob-a", DIGEST_A), obligation("ob-b", DIGEST_B)],
        vec![c1, c2, c3, c4],
        vec![
            authority(
                "a1",
                "owner-a",
                "accept_claim",
                Some(&["builder_projection", "campaign_terminalization"]),
            ),
            authority("a2", "owner-b", "accept_claim", None),
            authority("a3", "owner-a", "audit_read", None),
            authority(
                "a4",
                "episode_host",
                "write_episode",
                Some(&["x", "y", "z"]),
            ),
        ],
    )
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            items.swap(index, self.below(index + 1));
        }
    }
}

/// Writes `value` with every array shuffled and every object's keys in a shuffled order. v0 has
/// no semantically ordered array, so every array is a set (contract 17.2).
fn shuffled_text(value: &Value, rng: &mut Rng, out: &mut String) {
    match value {
        Value::Array(items) => {
            let mut order: Vec<&Value> = items.iter().collect();
            rng.shuffle(&mut order);
            out.push('[');
            for (index, item) in order.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                shuffled_text(item, rng, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            let mut order: Vec<(&String, &Value)> = members.iter().collect();
            rng.shuffle(&mut order);
            out.push('{');
            for (index, (key, member)) in order.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap());
                out.push(':');
                shuffled_text(member, rng, out);
            }
            out.push('}');
        }
        scalar => out.push_str(&serde_json::to_string(scalar).unwrap()),
    }
}

fn array_pointers(value: &Value, here: &str, out: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            out.push(here.to_owned());
            for (index, item) in items.iter().enumerate() {
                array_pointers(item, &format!("{here}/{index}"), out);
            }
        }
        Value::Object(members) => {
            for (key, member) in members {
                array_pointers(member, &format!("{here}/{key}"), out);
            }
        }
        _ => {}
    }
}

fn permutations(length: usize) -> Vec<Vec<usize>> {
    fn heap(k: usize, items: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if k <= 1 {
            out.push(items.clone());
            return;
        }
        for index in 0..k {
            heap(k - 1, items, out);
            let swap = if k.is_multiple_of(2) { index } else { 0 };
            items.swap(swap, k - 1);
        }
    }
    let mut out = Vec::new();
    heap(length, &mut (0..length).collect(), &mut out);
    out
}

struct Identity {
    requirements: Digest,
    content: Digest,
    candidate_bytes: Vec<u8>,
    trace: Vec<work_engine_organization::TraceRow>,
}

fn identity_of(text: &[u8]) -> Identity {
    let requirements = OrganizationalRequirements::from_json_bytes(text)
        .unwrap_or_else(|error| panic!("refused: {error}"));
    let mut result = derive(&requirements, Profile::FixedSliceReviewV1);
    assert!(result.unresolved.is_empty(), "{:?}", result.unresolved);
    let candidate = result.candidates.remove(0);
    Identity {
        requirements: requirements.requirements_digest(),
        content: candidate.content_digest(),
        candidate_bytes: candidate.canonical_bytes().to_vec(),
        trace: candidate.requirement_trace().to_vec(),
    }
}

fn assert_same(reference: &Identity, other: &Identity, what: &str) {
    assert_eq!(
        reference.requirements, other.requirements,
        "{what}: requirementsDigest"
    );
    assert_eq!(reference.content, other.content, "{what}: contentDigest");
    assert_eq!(reference.trace, other.trace, "{what}: trace");
    assert_eq!(
        reference.candidate_bytes, other.candidate_bytes,
        "{what}: candidate bytes"
    );
}

#[test]
fn repeated_derivation_gives_identical_bytes_and_digests() {
    let input = bytes(&rich());
    let first = identity_of(&input);
    for _ in 0..5 {
        assert_same(&first, &identity_of(&input), "repeat");
    }
}

#[test]
fn permuting_any_one_set_gives_identical_digests_and_trace() {
    let base_value = rich();
    let reference = identity_of(&bytes(&base_value));
    let mut pointers = Vec::new();
    array_pointers(&base_value, "", &mut pointers);
    let mut tested_sets = 0;
    let mut largest = 0;
    for pointer in pointers {
        let length = base_value
            .pointer(&pointer)
            .unwrap()
            .as_array()
            .unwrap()
            .len();
        if length < 2 {
            continue;
        }
        assert!(
            length <= 4,
            "{pointer}: all-permutation test covers sets up to 4"
        );
        tested_sets += 1;
        largest = largest.max(length);
        for order in permutations(length) {
            let mut changed = base_value.clone();
            let original = base_value
                .pointer(&pointer)
                .unwrap()
                .as_array()
                .unwrap()
                .clone();
            *changed.pointer_mut(&pointer).unwrap() =
                Value::Array(order.iter().map(|&index| original[index].clone()).collect());
            assert_same(&reference, &identity_of(&bytes(&changed)), &pointer);
        }
    }
    // obligations, conditions, authorities, capabilities, allowed (x2), boundaries (x2).
    assert!(tested_sets >= 7, "tested {tested_sets} sets");
    assert_eq!(largest, 4);
}

#[test]
fn seeded_shuffles_of_every_array_and_object_key_order_give_identical_digests() {
    let base_value = rich();
    let reference = identity_of(&bytes(&base_value));
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for round in 0..128 {
        let mut text = String::new();
        shuffled_text(&base_value, &mut rng, &mut text);
        assert_same(
            &reference,
            &identity_of(text.as_bytes()),
            &format!("shuffle {round}"),
        );
    }
    // A larger set: 12 conditions, 150 shuffles.
    let mut big = rich();
    let extra = (5..13).map(|n| {
        condition(
            &format!("c{n}"),
            if n % 2 == 0 { "ob-a" } else { "ob-b" },
            "campaign_terminalization",
            "campaign",
            if n % 2 == 0 { "owner-a" } else { "owner-b" },
        )
    });
    big["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .extend(extra);
    let reference = identity_of(&bytes(&big));
    for round in 0..150 {
        let mut text = String::new();
        shuffled_text(&big, &mut rng, &mut text);
        assert_same(
            &reference,
            &identity_of(text.as_bytes()),
            &format!("big {round}"),
        );
    }
}

#[test]
fn authored_whitespace_and_escapes_do_not_change_identity() {
    let compact = bytes(&base());
    let spaced = serde_json::to_string_pretty(&base()).unwrap();
    assert_same(
        &identity_of(&compact),
        &identity_of(spaced.as_bytes()),
        "whitespace",
    );
    // `a` and `a` are the same string; the authored escape is not identity.
    let escaped = String::from_utf8(compact.clone())
        .unwrap()
        .replace("\"ob-a\"", "\"ob-\\u0061\"");
    assert_same(
        &identity_of(&compact),
        &identity_of(escaped.as_bytes()),
        "escapes",
    );
}

// ---------------------------------------------------------------------------------------------
// 1A-8 golden vectors

const FROZEN_REQUIREMENTS: &[u8] = include_bytes!("fixtures/frozen/requirements.json");

macro_rules! golden {
    ($name:literal) => {
        include_bytes!(concat!("fixtures/golden/", $name, ".canonical.json")) as &[u8]
    };
}

#[test]
fn golden_vectors_fix_canonical_bytes_and_digests() {
    type Case<'a> = (
        &'a str,
        OrganizationalRequirements,
        &'a [u8],
        &'a [u8],
        &'a str,
        &'a str,
    );
    let cases: Vec<Case> = vec![
        (
            "frozen vertical",
            OrganizationalRequirements::from_json_bytes(FROZEN_REQUIREMENTS).unwrap(),
            golden!("frozen-requirements"),
            golden!("frozen-candidate"),
            "81b0e582a9376caeb6b491e2fc1730276252649ec83d1326100aec46d5b351b0",
            "67641c2219d02090f81042720f150bebaf2bab7ef301be69e9b9ae6c04f731a4",
        ),
        (
            "heterogeneous conditions",
            parse(&two_condition_vantage()),
            golden!("hetero-requirements"),
            golden!("hetero-candidate"),
            "dd94d9f4a8064d18b2541d11ee13feae2f78cad2841630c2a5ff27816cc88fc0",
            "9053f60b41ccc1e941d47a87f3b851a3eb745b57f0891e994db5a402984d8f8f",
        ),
    ];
    for (name, requirements, requirements_bytes, candidate_bytes, requirements_hex, content_hex) in
        cases
    {
        assert_eq!(requirements.canonical_bytes(), requirements_bytes, "{name}");
        assert_eq!(
            requirements.requirements_digest().to_hex(),
            requirements_hex,
            "{name}"
        );
        let result = derive(&requirements, Profile::FixedSliceReviewV1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.canonical_bytes(), candidate_bytes, "{name}");
        assert_eq!(candidate.content_digest().to_hex(), content_hex, "{name}");
        assert_eq!(
            candidate.requirements_digest(),
            requirements.requirements_digest(),
            "{name}"
        );
        // The digests are the framed hashes of the golden bytes (also checked with Python
        // hashlib in `p2-golden-check.txt`).
        assert_eq!(
            framed_hex(DigestKind::Candidate, candidate_bytes),
            content_hex,
            "{name}"
        );
        assert_eq!(
            framed_hex(DigestKind::Requirements, requirements_bytes),
            requirements_hex,
            "{name}"
        );
    }
}

/// The framing of contract 17.5, written out again here so the golden check does not call the
/// crate's own framing function.
fn framed_hex(kind: DigestKind, bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"work-engine.organization\0");
    hasher.update(kind.as_str().as_bytes());
    hasher.update(b"\0v0\0");
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
