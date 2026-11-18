"""Write mutant-plan-1.json: pre-hashes, then apply / mapped test(s) / restore per mutant, then final hashes."""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
MUTATE = str(HERE / "mutate.py")
FILES = [
    "crates/work-engine-review-delivery/src/gate.rs",
    "crates/work-engine-review-delivery/src/native.rs",
    "crates/work-engine-review-preparation/src/fixture.rs",
]


def cargo(package, features, *rest):
    return ["cargo", "test", "--offline", "--locked", "-p", package, "--features", features, *rest]


def gate_test(name):
    return cargo("work-engine-review-delivery", "fixture", "--test", "gate", "--", name, "--exact")


def native_test(name):
    return cargo("work-engine-review-delivery", "test-support", "--test", "native_vertical", "--", name, "--exact")


def consumer_test(name):
    return cargo("work-engine-review-preparation", "fixture", "--test", "fixture_consumer", "--", name, "--exact")


DOC = cargo("work-engine-review-delivery", "fixture", "--doc")

MAP = [
    ("M1-admitted-family", [gate_test("discharge_is_total_admitted_kinds_name_their_check_ids")]),
    ("M2-assumed-family", [gate_test("discharge_is_total_admitted_kinds_name_their_check_ids")]),
    ("M3-realization-check", [gate_test("discharge_is_total_admitted_kinds_name_their_check_ids")]),
    ("M4-refusal-after-spawn", [native_test("sb_an_admission_refusal_refuses_before_spawn")]),
    ("M6-replay-admission-observation", [
        native_test("sb_revocation_supersession_or_missing_grant_between_boundaries_refuses_consumption"),
        gate_test("regate_reads_a_fresh_observation"),
    ]),
    ("M7-no-regate", [native_test("sb_revocation_supersession_or_missing_grant_between_boundaries_refuses_consumption")]),
    ("M8-unknown-permits", [native_test("sb_unknown_at_the_regate_fails_closed")]),
    ("M9-no-finality", [gate_test("the_first_non_current_state_other_than_unknown_is_final")]),
    ("M10-public-run-attempt", [DOC]),
    ("M11-public-assess", [DOC]),
    ("S1-P1-other-candidate", [consumer_test("s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused")]),
    ("S1-P2-other-subject", [consumer_test("s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused")]),
    ("S1-P3-other-boundary", [consumer_test("s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused")]),
    ("S1-P4-missing-proof-accepted", [consumer_test("s1_a_gate_proof_that_is_not_current_or_has_no_observation_is_refused")]),
    ("S1-P5-skip-proof-check", [consumer_test("s1_a_gate_proof_that_is_not_current_or_has_no_observation_is_refused")]),
    ("S1-P6-test-support-under-fixture", [DOC]),
    ("S1-P7-delivery-regate-revision-dropped", [native_test("s1_both_regate_revisions_are_carried_and_follow_their_admissions")]),
]

steps = [{"name": "pre-hashes", "argv": ["git", "hash-object", *FILES], "timeout": 60}]
for mutant, tests in MAP:
    steps.append({"name": f"{mutant}.apply", "argv": ["python3", "-I", MUTATE, "apply", mutant], "timeout": 60})
    for i, argv in enumerate(tests):
        steps.append({"name": f"{mutant}.test{i}", "argv": argv, "timeout": 1800, "kill_leftovers": True})
    steps.append({"name": f"{mutant}.restore", "argv": ["python3", "-I", MUTATE, "restore", mutant], "timeout": 60})
steps.append({"name": "final-hashes", "argv": ["git", "hash-object", *FILES], "timeout": 60})

plan = {
    "cwd": "/home/bline/cbm-work/wt-org-hardening-20261117/rust",
    "env": {
        "CARGO_TARGET_DIR": "/home/bline/cbm-work/org-r2-core-20261117/target",
        "WE_PI_SDK_ROOT": "/home/bline/cbm-work/pi-artifact-handoff-20261113/pi",
        "WE_PI_SDK_MANIFEST": "/home/bline/cbm-work/pi-artifact-handoff-20261113/pi.manifest.json",
    },
    "steps": steps,
}
(HERE / "mutant-plan-1.json").write_text(json.dumps(plan, indent=1) + "\n")
print(f"{len(MAP)} mutants, {len(steps)} steps")
