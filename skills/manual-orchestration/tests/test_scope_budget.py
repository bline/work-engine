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

    def test_v1_meter_authority_and_upgrade(self):
        ro = self.d / "rollout.jsonl"
        rollout(ro, [35.0])
        call("init", "--record", str(self.rec), "--objective", "x", "--item", "a", "--rollout", str(ro),
             "--max-hours", "100", "--set-by", "coordinator")
        doc = json.loads(self.rec.read_text()); sb = doc["scope_budget"]
        sb["schema"] = S.SCHEMA_V1
        for k in ("objective_id", "sessions", "bindings", "cost_limits"): sb.pop(k)
        sb["limits"]["max_weekly_points"] = 2
        sb["limit_history"] = [{"set_by": "user", "limits": {"max_hours": 100}}]
        self.rec.write_text(json.dumps(doc)); rollout(ro, [40.0])
        rc, r = call("check", "--record", str(self.rec), "--rollout", str(ro))
        self.assertEqual((rc, r["account_weekly_points"], r["advisory"], r["account_wide"]), (0, 5, ["max_weekly_points"], True))
        sb["limit_history"].append({"set_by": "user", "limits": {"max_weekly_points": 2}})
        self.rec.write_text(json.dumps(doc))
        self.assertEqual(call("check", "--record", str(self.rec), "--rollout", str(ro))[0], 4)
        sb["limits"].pop("max_weekly_points"); sb["limits"]["max_weekly_percent"] = 39
        self.rec.write_text(json.dumps(doc))
        self.assertEqual(call("check", "--record", str(self.rec), "--rollout", str(ro))[0], 4)
        call("add", "--record", str(self.rec), "--item", "b", "--reason", "test")
        sb = json.loads(self.rec.read_text())["scope_budget"]
        self.assertEqual(sb["schema"], S.SCHEMA)
        self.assertTrue(sb["objective_id"]); self.assertEqual(sb["sessions"], [])

    def test_v2_binding_and_cost_limits(self):
        rc, _ = call("init", "--record", str(self.rec), "--objective", "x", "--item", "a",
                     "--max-hours", "3", "--set-by", "user", "--session", "root", "--provider", "codex")
        self.assertEqual(rc, 0)
        args = ("add-session", "--record", str(self.rec), "--session", "next", "--provider", "claude",
                "--from", "2026-10-30T00:00:00Z", "--bound-by", "rotation", "--evidence", "adoption")
        self.assertEqual(call(*args)[0], 0); before = self.rec.read_bytes()
        self.assertEqual(call(*args)[0], 0); self.assertEqual(self.rec.read_bytes(), before)
        self.assertEqual(call(*args[:-1], "different")[0], 2)
        self.assertEqual(call("bind", "--record", str(self.rec), "--session", "review", "--provider", "codex",
                              "--parent", "root", "--bound-by", "coordinator", "--reason", "review")[0], 0)
        base = ("cost-limit", "--record", str(self.rec), "--pool", "codex", "--set-by", "user", "--reason", "limit")
        self.assertEqual(call(*base, "--max-usd", "10", "--mark-usd", "5", "--mark-usd", "2")[0], 0)
        self.assertEqual(call(*base, "--max-usd", "20")[0], 0)
        sb = json.loads(self.rec.read_text())["scope_budget"]
        self.assertEqual(len(sb["sessions"]), 2); self.assertEqual(sb["bindings"][0]["parent_session_id"], "root")
        self.assertEqual(sb["cost_limits"][0]["max_usd"], 20)
        self.assertEqual(sb["limit_history"][-1]["previous_cost_limit"]["marks_usd"], [2, 5])

    def test_cost_limits_preserve_other_setters_and_history(self):
        call("init", "--record", str(self.rec), "--objective", "x", "--item", "a", "--max-hours", "3", "--set-by", "user")
        for node in (None, "child"):
            base = ["cost-limit", "--record", str(self.rec), "--pool", "codex", "--reason", "test"]
            if node: base += ["--node", node]
            for setter, option, value in (("user", "--max-usd", "10"), ("coordinator", "--mark-usd", "5"),
                                          ("coordinator", "--max-usd", "7"), ("user", "--max-usd", "20")):
                self.assertEqual(call(*base, "--set-by", setter, option, value)[0], 0)
            sb = json.loads(self.rec.read_text())["scope_budget"]
            entries = {e["set_by"]: e for e in sb["cost_limits"] if e["node"] == node}
            self.assertEqual(entries["user"]["max_usd"], 20)
            self.assertEqual(entries["coordinator"]["max_usd"], 7)
            history = [h for h in sb["limit_history"] if h["cost_limit"]["node"] == node]
            self.assertEqual(len(history), 4)
            self.assertIsNone(history[1]["previous_cost_limit"])
            self.assertEqual(history[2]["previous_cost_limit"]["marks_usd"], [5])
            self.assertEqual(history[3]["previous_cost_limit"]["max_usd"], 10)

    def test_input_validation_and_empty_root(self):
        base = ("init", "--record", str(self.rec), "--objective", "x", "--item", "a", "--max-hours", "2", "--set-by", "user")
        for extra in (("--session", "root"), ("--provider", "codex")):
            self.assertEqual(call(*base, *extra)[0], 2)
        self.assertEqual(call(*base)[0], 0)
        self.assertEqual(json.loads(self.rec.read_text())["scope_budget"]["sessions"], [])
        cost = ("cost-limit", "--record", str(self.rec), "--pool", "codex", "--set-by", "user", "--reason", "x")
        before = self.rec.read_bytes()
        for extra in ((), ("--max-usd", "-1"), ("--max-usd", "nan"), ("--mark-usd", "inf"), ("--mark-usd", "-2")):
            self.assertEqual(call(*cost, *extra)[0], 2)
            self.assertEqual(self.rec.read_bytes(), before)
        self.assertEqual(call(*cost, "--max-usd", "0")[0], 0)
        self.assertIn("cost-limit", call("revise", "--record", str(self.rec), "--max-weekly-points", "2", "--set-by", "user")[1])

    def test_legacy_authority_and_missing_meter(self):
        ro = self.d / "meter.jsonl"
        rollout(ro, [35])
        call("init", "--record", str(self.rec), "--objective", "x", "--item", "a", "--rollout", str(ro), "--max-hours", "100", "--set-by", "user")
        doc = json.loads(self.rec.read_text()); sb = doc["scope_budget"]
        sb["schema"] = S.SCHEMA_V1; sb["limits"]["max_weekly_points"] = 2
        rollout(ro, [40])
        self.rec.write_text(json.dumps(doc))
        self.assertEqual(call("check", "--record", str(self.rec), "--rollout", str(ro))[0], 4)
        rc, result = call("check", "--record", str(self.rec))
        self.assertEqual((rc, result["account_weekly_points"], result["unknown"], result["advisory"]), (0, None, ["max_weekly_points"], []))
        sb["limits_set_by"] = "coordinator"
        sb["limit_history"] = [{"set_by": "user", "limits": {"max_weekly_points": 3}}]
        self.rec.write_text(json.dumps(doc)); before = self.rec.read_bytes()
        rc, result = call("check", "--record", str(self.rec), "--rollout", str(ro))
        self.assertEqual((rc, result["advisory"]), (0, ["max_weekly_points"]))
        rc, result = call("check", "--record", str(self.rec))
        self.assertEqual((rc, result["account_weekly_points"], result["unknown"], result["advisory"]), (0, None, [], ["max_weekly_points"]))
        self.assertEqual(self.rec.read_bytes(), before)

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
        self.assertIn("cost-limit", err)
        call("init", "--record", str(self.rec), "--objective", "x", "--item", "a", "--max-hours", "1", "--set-by", "user")
        self.assertEqual(call("revise", "--record", str(self.rec), "--max-weekly-points", "2", "--set-by", "user")[0], 2)


if __name__ == "__main__":
    unittest.main()
