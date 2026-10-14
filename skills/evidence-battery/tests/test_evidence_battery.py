from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


BATTERY = load("evidence_battery", ROOT / "scripts" / "evidence_battery.py")


class EvidenceBatteryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = Path(tempfile.mkdtemp())
        self.addCleanup(subprocess.run, ["rm", "-rf", str(self.directory)])
        (self.directory / "owned.txt").write_text("owned\n")
        (self.directory / "receipt.json").write_text(json.dumps(
            {"status": "accepted", "nested": {"episode": "e1"}, "items": [7, 8]}))
        (self.directory / "source.rs").write_text("fn a() { b.unwrap() }\nfn c() {}\n")

    def plan(self, **overrides):
        plan = {"root": str(self.directory), "checks": [
            {"id": "owned", "sha256": ["owned.txt", "missing.txt"]},
            {"id": "receipt", "json": {"path": "receipt.json",
                                       "select": ["status", "nested.episode", "items.1"]}},
            {"id": "lints", "grep": {"pattern": r"unwrap\(", "paths": ["source.rs"]}},
        ]}
        plan.update(overrides)
        return plan

    def test_one_battery_answers_every_check(self) -> None:
        result = BATTERY.run(self.plan())
        self.assertEqual(result["check_count"], 3)
        self.assertEqual(result["failed"], [])
        owned = result["checks"]["owned"]
        self.assertEqual(owned["path_count"], 2)
        self.assertEqual(owned["entries"][0]["sha256"], BATTERY.digest(b"owned\n"))
        self.assertEqual(owned["entries"][1], {"path": "missing.txt", "type": "absent"})
        self.assertEqual(result["checks"]["receipt"]["selected"],
                         {"status": "accepted", "nested.episode": "e1", "items.1": 8})
        self.assertEqual(result["checks"]["lints"]["match_count"], 1)

    def test_digest_is_stable_and_moves_only_on_change(self) -> None:
        first = BATTERY.run(self.plan())
        self.assertEqual(first["battery_sha256"], BATTERY.run(self.plan())["battery_sha256"])
        (self.directory / "owned.txt").write_text("changed\n")
        second = BATTERY.run(self.plan())
        self.assertNotEqual(first["battery_sha256"], second["battery_sha256"])

    def test_compare_reports_unchanged_without_restating_evidence(self) -> None:
        first = BATTERY.run(self.plan())
        unchanged = BATTERY.compare(BATTERY.run(self.plan()), first)
        self.assertEqual(unchanged["status"], "unchanged")
        self.assertNotIn("changed", unchanged)
        (self.directory / "owned.txt").write_text("changed\n")
        changed = BATTERY.compare(BATTERY.run(self.plan()), first)
        self.assertEqual(changed["status"], "changed")
        self.assertEqual(changed["changed_ids"], ["owned"])

    def test_symlinks_are_recorded_not_followed(self) -> None:
        os.symlink("owned.txt", self.directory / "link.txt")
        entry = BATTERY.run(self.plan(checks=[{"id": "l", "sha256": ["link.txt"]}]))
        value = entry["checks"]["l"]["entries"][0]
        self.assertEqual(value["type"], "symlink")
        self.assertEqual(value["link_target"], "owned.txt")

    def test_paths_outside_every_root_are_refused(self) -> None:
        result = BATTERY.run(self.plan(checks=[{"id": "escape", "sha256": ["../../etc/hostname"]}]))
        self.assertEqual(result["failed"], ["escape"])
        self.assertIn("escapes every declared root", result["checks"]["escape"]["error"])

    def test_extra_roots_admit_declared_evidence_directories(self) -> None:
        outside = Path(tempfile.mkdtemp())
        self.addCleanup(subprocess.run, ["rm", "-rf", str(outside)])
        (outside / "plan.md").write_text("plan\n")
        result = BATTERY.run(self.plan(extra_roots=[str(outside)],
                                       checks=[{"id": "p", "sha256": [str(outside / "plan.md")]}]))
        self.assertEqual(result["failed"], [])
        self.assertEqual(result["checks"]["p"]["entries"][0]["sha256"], BATTERY.digest(b"plan\n"))

    def test_only_read_only_git_subcommands_run(self) -> None:
        subprocess.run(["git", "init", "-q"], cwd=self.directory, check=True)
        ok = BATTERY.run(self.plan(checks=[{"id": "status", "git": ["status", "--porcelain=v1"]}]))
        self.assertEqual(ok["failed"], [])
        self.assertIn("output_sha256", ok["checks"]["status"])
        for argv in (["commit", "-m", "no"], ["push"], ["-c", "x=y", "status"]):
            blocked = BATTERY.run(self.plan(checks=[{"id": "blocked", "git": argv}]))
            self.assertEqual(blocked["failed"], ["blocked"], argv)

    def test_one_failed_check_does_not_lose_the_others(self) -> None:
        result = BATTERY.run(self.plan(checks=[
            {"id": "good", "sha256": ["owned.txt"]},
            {"id": "bad", "json": {"path": "source.rs"}},
        ]))
        self.assertEqual(result["failed"], ["bad"])
        self.assertIn("entries", result["checks"]["good"])

    def test_a_check_must_declare_exactly_one_kind(self) -> None:
        for checks in ([{"id": "a"}], [{"id": "a", "sha256": [], "git": ["status"]}],
                       [{"id": "a", "sha256": ["owned.txt"]}, {"id": "a", "sha256": []}]):
            with self.assertRaises(ValueError):
                BATTERY.run(self.plan(checks=checks))

    def test_command_line_compare_round_trip(self) -> None:
        plan = self.directory / "plan.json"
        plan.write_text(json.dumps(self.plan()))
        saved = self.directory / "first.json"
        script = str(ROOT / "scripts" / "evidence_battery.py")
        subprocess.run(["python3", "-I", script, "--plan", str(plan), "--save", str(saved),
                        "--quiet"], check=True, stdout=subprocess.DEVNULL)
        again = subprocess.run(["python3", "-I", script, "--plan", str(plan),
                                "--compare", str(saved)], check=True, stdout=subprocess.PIPE)
        self.assertEqual(json.loads(again.stdout)["status"], "unchanged")


if __name__ == "__main__":
    unittest.main()
