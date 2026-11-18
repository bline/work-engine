"""Write mutant-plan.json for S1 remediation 1: pre-hashes, then apply / mapped test(s) / restore per mutant, then final hashes.

S1-P1..P5 are re-run because the mapped test file changed; S1-P8 (vantage comparison neutralized) is new. S1-P8 maps to the
whole fixture_consumer suite: no test is expected to fail (the vantage comparison is not reachable by an S-A-admitted proof
in the test world), and a pass is recorded as survival, not as a kill.
"""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
MUTATE = str(HERE / "mutate.py")
FILES = [
    "crates/work-engine-review-delivery/src/gate.rs",
    "crates/work-engine-review-delivery/src/native.rs",
    "crates/work-engine-review-preparation/src/fixture.rs",
]


def consumer_test(*names):
    base = ["cargo", "test", "--offline", "--locked", "-p", "work-engine-review-preparation",
            "--features", "fixture", "--test", "fixture_consumer"]
    return base + (["--", *names, "--exact"] if names else [])


LOOP = "s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused"
NOT_CURRENT = "s1_a_gate_proof_that_is_not_current_or_has_no_observation_is_refused"
MAP = [
    ("S1-P1-other-candidate", [consumer_test(LOOP)]),
    ("S1-P2-other-subject", [consumer_test(LOOP)]),
    ("S1-P3-other-boundary", [consumer_test(LOOP)]),
    ("S1-P4-missing-proof-accepted", [consumer_test(NOT_CURRENT)]),
    ("S1-P5-skip-proof-check", [consumer_test(NOT_CURRENT)]),
    ("S1-P8-vantage-unchecked", [consumer_test()]),
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
(HERE / "mutant-plan.json").write_text(json.dumps(plan, indent=1) + "\n")
print(f"{len(MAP)} mutants, {len(steps)} steps")
