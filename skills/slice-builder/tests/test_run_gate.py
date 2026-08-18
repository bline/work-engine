from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "run_gate.py"
SPEC = importlib.util.spec_from_file_location("run_gate", SCRIPT)
assert SPEC and SPEC.loader
RUN_GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUN_GATE)


class RunGateTest(unittest.TestCase):
    def check(self, identity: str, code: str) -> dict[str, object]:
        return {
            "requirement": "focused_checks",
            "identity": identity,
            "command": [sys.executable, "-c", code],
        }

    def test_success_reports_every_check_compactly(self) -> None:
        manifest = {
            "checks": [
                self.check("vertical-proof", "pass"),
                self.check("focused", "pass"),
            ]
        }
        result = RUN_GATE.run(RUN_GATE.load_manifest(json.dumps(manifest)), Path.cwd())
        self.assertEqual("passed", result["status"])
        self.assertEqual(
            {"configured": 2, "executed": 2, "passed": 2, "failed": 0},
            result["totals"],
        )
        self.assertIsNone(result["failed_check"])

    def test_failure_stops_before_later_checks_and_bounds_excerpt(self) -> None:
        manifest = {
            "checks": [
                self.check("vertical-proof", "pass"),
                self.check("failure", "import sys; print('x' * 4000); sys.exit(7)"),
                self.check("must-not-run", "raise SystemExit(99)"),
            ]
        }
        result = RUN_GATE.run(RUN_GATE.load_manifest(json.dumps(manifest)), Path.cwd())
        self.assertEqual("failed", result["status"])
        self.assertEqual("failure", result["failed_check"])
        self.assertEqual(2, result["totals"]["executed"])
        self.assertEqual(7, result["checks"][-1]["exit_status"])
        self.assertLessEqual(len(result["checks"][-1]["error_excerpt"]), RUN_GATE.MAX_EXCERPT_CHARS)

    def test_rejects_shell_string_commands(self) -> None:
        with self.assertRaisesRegex(ValueError, "string array"):
            RUN_GATE.load_manifest(
                '{"checks":[{"requirement":"focused_checks","identity":"bad","command":"false"}]}'
            )

    def test_missing_executable_is_a_bounded_check_failure(self) -> None:
        manifest = {
            "checks": [
                {
                    "requirement": "focused_checks",
                    "identity": "missing-command",
                    "command": ["definitely-not-a-real-work-engine-command"],
                }
            ]
        }
        result = RUN_GATE.run(RUN_GATE.load_manifest(json.dumps(manifest)), Path.cwd())
        self.assertEqual("failed", result["status"])
        self.assertEqual("missing-command", result["failed_check"])
        self.assertIsNone(result["checks"][0]["exit_status"])


if __name__ == "__main__":
    unittest.main()
