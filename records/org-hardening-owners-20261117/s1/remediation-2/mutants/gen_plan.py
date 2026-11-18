"""Write mutant-plan.json for S1 remediation 2: S1-P8 (vantage comparison neutralized) against the whole
fixture_consumer suite, which now has the two-obligation case. S1-P1..P5 anchors (fixture.rs lines) and their
mapped tests are unchanged, so they are not re-run (hashes of fixture.rs before and after bracket the run)."""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
MUTATE = str(HERE / "mutate.py")
FILES = ["crates/work-engine-review-preparation/src/fixture.rs"]
BASE = ["cargo", "test", "--offline", "--locked", "-p", "work-engine-review-preparation",
        "--features", "fixture", "--test", "fixture_consumer"]
M = "S1-P8-vantage-unchecked"
steps = [
    {"name": "pre-hashes", "argv": ["git", "hash-object", *FILES], "timeout": 60},
    {"name": f"{M}.apply", "argv": ["python3", "-I", MUTATE, "apply", M], "timeout": 60},
    {"name": f"{M}.test-suite", "argv": BASE, "timeout": 1800, "kill_leftovers": True},
    {"name": f"{M}.restore", "argv": ["python3", "-I", MUTATE, "restore", M], "timeout": 60},
    {"name": "final-hashes", "argv": ["git", "hash-object", *FILES], "timeout": 60},
]
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
print(f"1 mutant, {len(steps)} steps")
