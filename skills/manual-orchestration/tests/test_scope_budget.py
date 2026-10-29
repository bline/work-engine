from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
spec = importlib.util.spec_from_file_location("scope_budget", ROOT / "scripts" / "scope_budget.py")
assert spec and spec.loader
S = importlib.util.module_from_spec(spec)
spec.loader.exec_module(S)


def call(*argv: str) -> tuple[int, dict | str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        rc = S.main(list(argv))
    return rc, (json.loads(out.getvalue()) if out.getvalue() else err.getvalue())


def rollout(path: Path, percents: list[float], filler: int = 0) -> None:
    lines = []
    for i, p in enumerate(percents):
        lines.extend(json.dumps({"type": "response_item", "payload": {"x": "y" * 50}}) for _ in range(filler))
        lines.append(json.dumps({"timestamp": f"2026-10-29T0{i}:00:00Z", "type": "event_msg", "payload": {
            "type": "token_count", "rate_limits": {
                "primary": {"used_percent": p, "window_minutes": 10080, "resets_at": 1},
                "secondary": None}}}))
    lines.append(json.dumps({"type": "response_item", "payload": {"type": "message"}}))
    path.write_text("\n".join(lines) + "\n")


class ScopeBudgetTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)
        self.rec = self.d / "assignment.json"
        self.rec.write_text(json.dumps({"owners": {"a": "b"}}))

    def tearDown(self):
        self.tmp.cleanup()

    def test_additions_trigger_reconsideration_and_keep_other_keys(self):
        rc, r = call("init", "--record", str(self.rec), "--objective", "S10", "--item", "one", "--item", "two",
                     "--max-hours", "100", "--reconsider-after-additions", "2", "--set-by", "user")
        self.assertEqual(rc, 0)
        self.assertEqual(call("check", "--record", str(self.rec))[0], 0)
        call("add", "--record", str(self.rec), "--item", "three", "--reason", "gap found in review")
        rc, r = call("add", "--record", str(self.rec), "--item", "four", "--reason", "consumer validation")
        self.assertTrue(r["reconsider"])
        rc, r = call("check", "--record", str(self.rec))
        self.assertEqual(rc, 3)
        self.assertEqual((r["additions"], r["original_items"], r["stop"]), (2, 2, False))
        doc = json.loads(self.rec.read_text())
        self.assertEqual(doc["owners"], {"a": "b"})
        self.assertEqual([a["item"] for a in doc["scope_budget"]["additions"]], ["three", "four"])

    def test_hours_limit_stops_and_revise_keeps_history(self):
        call("init", "--record", str(self.rec), "--objective", "x", "--item", "a",
             "--max-hours", "0", "--set-by", "coordinator", "--reason", "proposed in first report")
        rc, r = call("check", "--record", str(self.rec))
        self.assertEqual(rc, 4)
        self.assertEqual(r["reached"], ["hours"])
        rc, r = call("revise", "--record", str(self.rec), "--max-hours", "8", "--set-by", "user", "--reason", "owner")
        self.assertEqual(r["limits"]["max_hours"], 8)
        sb = json.loads(self.rec.read_text())["scope_budget"]
        self.assertEqual(sb["limit_history"][0]["limits"]["max_hours"], 0)
        self.assertEqual(sb["limits_set_by"], "user")
        self.assertEqual(call("check", "--record", str(self.rec))[0], 0)

    def test_weekly_meter_points_from_rollout(self):
        ro = self.d / "rollout.jsonl"
        rollout(ro, [35.0])
        call("init", "--record", str(self.rec), "--objective", "x", "--item", "a", "--rollout", str(ro),
             "--max-weekly-points", "20", "--set-by", "user")
        rollout(ro, [35.0, 50.0])
        rc, r = call("check", "--record", str(self.rec), "--rollout", str(ro))
        self.assertEqual((rc, r["weekly_points_used"]), (0, 15.0))
        rollout(ro, [35.0, 50.0, 79.0])
        rc, r = call("check", "--record", str(self.rec), "--rollout", str(ro))
        self.assertEqual((rc, r["reached"]), (4, ["max_weekly_points"]))
        rc, r = call("check", "--record", str(self.rec))
        self.assertEqual((rc, r["unknown"]), (0, ["max_weekly_points"]))

    def test_meter_read_across_block_boundaries(self):
        ro = self.d / "rollout.jsonl"
        rollout(ro, [10.0, 42.5], filler=40)
        self.assertEqual(S.weekly_percent(ro, block=97)["used_percent"], 42.5)
        (self.d / "empty.jsonl").write_text("")
        self.assertIsNone(S.weekly_percent(self.d / "empty.jsonl"))

    def test_usage_errors(self):
        rc, err = call("check", "--record", str(self.rec))
        self.assertEqual(rc, 2)
        rc, err = call("init", "--record", str(self.rec), "--objective", "x", "--item", "a",
                       "--reconsider-after-additions", "3", "--set-by", "user")
        self.assertEqual(rc, 2)
        self.assertIn("at least one", err)
        rc, err = call("init", "--record", str(self.rec), "--objective", "x", "--item", "a",
                       "--max-weekly-points", "5", "--set-by", "user")
        self.assertEqual(rc, 2)
        self.assertIn("--rollout", err)


if __name__ == "__main__":
    unittest.main()
