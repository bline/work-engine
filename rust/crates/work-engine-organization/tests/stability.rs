//! 1A-1 determinism and 1A-8 golden vectors. These tests check **stability**: that the same
//! meaning gives the same bytes, and that the bytes do not drift. They do not show that the
//! bytes are right. The goldens were checked against the rules before they were frozen: the
//! derived candidates were read against the amendment 3 table and the F4 table, and
//! `golden/*.canonical.json` was re-derived independently with Python (framing with `hashlib`,
//! canonical form with `json.dumps(sort_keys=True, separators=(",", ":"))`); see the builder
//! custody record `p2-golden-check.txt and p3-golden-check.txt`. The rule tests are in `rules.rs`.

mod common;

use common::*;
use serde_json::{Value, json};
use work_engine_organization::{
    Digest, DigestKind, OrganizationalRequirements, Profile, derive, view,
};

/// Two obligations, four conditions, four authorities, and every set kind with 2-4 members.
fn rich() -> Value {
    let mut c1 = condition("c1", "ob-a", "builder_projection", "builder", "owner-a");
    c1["normative"]["independence"] = json!({"from": ["state.builder_context"]});
    c1["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox"]}
    ]);
    let mut c2 = condition(
        "c2",
        "ob-a",
        "campaign_terminalization",
        "campaign",
        "owner-a",
    );
    c2["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["anthropic"]}]);
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
    // obligations, conditions, authorities, boundaries (x2), realizationConstraints. The
    // `capabilities` and `allowed` sets have at most one recognized member (decision N2), so
    // they have no authored order to permute.
    assert!(tested_sets >= 6, "tested {tested_sets} sets");
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

// Draft candidate goldens, redefined by G32 (problemScope and obligations added) under the root's
// section 17.6 record ("§17.6 and this prequalification correction", gap-decisions-r1.md): no
// admission names the e0c949bf digests, so this does not bump `profile`. The exemption ends when
// IG1A accepts a candidate; after that, any change to these bytes bumps the profile.
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
            "e47e5dd9ddcf626bb960214278d8c5fb05fcd9f2e0522ede3dc918ceaa08fca3",
        ),
        (
            "heterogeneous conditions",
            parse(&two_condition_vantage()),
            golden!("hetero-requirements"),
            golden!("hetero-candidate"),
            "fcb192ccbcc219cefb4a04ec33f0f3d9e8b96b57444316b74345397218152625",
            "3c9eab7d701c077df11029dcc400bc6b803b004a8b6c5c8a9c014d2cfcea5655",
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
        // hashlib in `p2-golden-check.txt and p3-golden-check.txt`).
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

/// Every relationship of a candidate is validated by `view()` (IG1A-R1 follow-up). A valid
/// derived candidate must still project, to the same view as before the validation existed: the
/// digests are of the `Debug` rendering of the view as computed at `81bcf441`, before
/// `validate_relationships` was added, with the whole validation call disabled.
#[test]
fn views_of_the_fixture_candidates_are_unchanged_by_relationship_validation() {
    use sha2::{Digest as _, Sha256};
    let cases: Vec<(&str, OrganizationalRequirements, &str)> = vec![
        (
            "frozen vertical",
            OrganizationalRequirements::from_json_bytes(FROZEN_REQUIREMENTS).unwrap(),
            "868f0f06692c171a3bca370d4f98ed587f4804f6daa08b0f806de59570af60f9",
        ),
        (
            "heterogeneous conditions",
            parse(&two_condition_vantage()),
            "04de034f0c1790d57fa5ef15585ce11af3c6855cb1a29a5cb0eeb143e11909e4",
        ),
        (
            "builder and campaign consumers",
            parse(&base()),
            "d41e4f85be5ee23b6a90ff5b2eef8dc78a11ff7d4ec2d86e1941b2a08b224380",
        ),
    ];
    let mut observed = Vec::new();
    for (name, requirements, pinned) in cases {
        let result = derive(&requirements, Profile::FixedSliceReviewV1);
        assert!(
            result.unresolved.is_empty(),
            "{name}: {:?}",
            result.unresolved
        );
        for candidate in &result.candidates {
            let projected = view(candidate).unwrap_or_else(|e| panic!("{name}: {e}"));
            let hex: String = Sha256::digest(format!("{projected:?}").as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            observed.push((name, hex, pinned));
        }
    }
    for (name, hex, _) in &observed {
        eprintln!("VIEWDIGEST {name} {hex}");
    }
    for (name, hex, pinned) in observed {
        assert_eq!(hex, pinned, "{name}");
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
