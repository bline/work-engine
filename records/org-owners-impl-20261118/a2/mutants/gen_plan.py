"""Write mutant-plan-N.json: pre-hashes, then apply / mapped test(s) / restore per mutant, then final hashes."""
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
MUTATE = str(HERE / "mutate.py")
FILES = [
    "crates/work-engine-organization-authority/src/gate.rs",
    "crates/work-engine-review-delivery/src/gate.rs",
]


def cargo(package, features, *rest):
    return ["cargo", "test", "--offline", "--locked", "-p", package, "--features", features, *rest]


def sa_test(name):
    return cargo("work-engine-organization-authority", "fixture", "--test", "admission", "--", name, "--exact")


def gate_test(name):
    return cargo("work-engine-review-delivery", "fixture", "--test", "gate", "--", name, "--exact")


def native_test(name):
    return cargo("work-engine-review-delivery", "test-support", "--test", "native_vertical", "--", name, "--exact")


FIELDS = sa_test("g2_oa_an_admission_of_another_candidate_requirements_or_scope_does_not_admit")
MAP = [
    ("A2-M1-first-of-two-heads", [sa_test("g2_oa_two_committed_admissions_of_the_candidate_are_ambiguous_heads")]),
    ("A2-M2-skip-pin", [
        sa_test("fixture_admission_binds_principal_actor_subject_boundary_revision_and_realization"),
        sa_test("state_revoked_when_the_organization_admission_is_revoked_after_admission"),
    ]),
    ("A2-M3-ignore-scope", [FIELDS]),
    ("A2-M4-ignore-candidate", [FIELDS]),
    ("A2-M5-ignore-requirements", [FIELDS]),
    ("A2-M6-bindings-by-vantage-only", [sa_test("g2_oa_bindings_of_another_organizations_admission_do_not_bind_this_one")]),
    ("A2-M7-point-check-ignores-organization-pins", [
        sa_test("state_revoked_when_the_organization_admission_is_revoked_after_admission"),
        sa_test("state_stale_when_the_organization_admission_is_superseded_and_its_successor_needs_new_bindings"),
    ]),
    ("A2-M8-revoked-admission-counts", [sa_test("g2_oa_an_admission_revoked_before_admission_leaves_the_candidate_unadmitted")]),
    ("A2-M9-oa-dropped-from-all", [native_test("sb_an_admission_refusal_refuses_before_spawn")]),
    ("A2-M10-separation-discharge-drops-oa", [gate_test("discharge_is_total_admitted_kinds_name_their_check_ids")]),
    ("A2-M11-regate-ignores-organization-refusals", [sa_test("state_refused_when_a_second_admission_of_the_candidate_is_committed_after_admission")]),
]

n = sys.argv[1]
steps = [{"name": "pre-hashes", "argv": ["git", "hash-object", *FILES], "timeout": 60}]
for mutant, tests in MAP:
    steps.append({"name": f"{mutant}.apply", "argv": ["python3", "-I", MUTATE, "apply", mutant], "timeout": 60})
    for i, argv in enumerate(tests):
        steps.append({"name": f"{mutant}.test{i}", "argv": argv, "timeout": 1800, "kill_leftovers": True})
    steps.append({"name": f"{mutant}.restore", "argv": ["python3", "-I", MUTATE, "restore", mutant], "timeout": 60})
steps.append({"name": "post-hashes", "argv": ["git", "hash-object", *FILES], "timeout": 60})
plan = {
    "cwd": "/home/bline/cbm-work/wt-org-owners-a2-20261118/rust",
    "env": {
        "CARGO_TARGET_DIR": "/home/bline/cbm-work/org-owners-impl-20261118/a2/target",
        "WE_PI_SDK_ROOT": "/home/bline/cbm-work/pi-artifact-handoff-20261113/pi",
        "WE_PI_SDK_MANIFEST": "/home/bline/cbm-work/pi-artifact-handoff-20261113/pi.manifest.json",
    },
    "steps": steps,
}
(HERE / f"mutant-plan-{n}.json").write_text(json.dumps(plan, indent=1) + "\n")
