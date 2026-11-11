"""S1: an objective is counted from its earliest authorized session start, and a closed record reports a
reconciled all-in figure (in-window + post-close) with hard-limit breaches flagged, without changing the record."""
import argparse
import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
SCOPE = os.path.join(REPO, "skills", "manual-orchestration", "scripts", "scope_budget.py")


def call(ts, usd=1.0, sid="root"):
    return dict(provider="codex", thread=sid, parent_thread=None, ts=ts, model="gpt-6.1-sol",
                service_tier="default", quota_multiplier=1, usd=usd, pools=["codex"])


def record(d, session_from, closed_at=None, max_usd=None):
    sb = dict(started_at="2026-01-01T01:00:00Z", objective_id="obj", bindings=[],
              sessions=[{"session_id": "root", "provider": "codex", "from": session_from}])
    if closed_at:
        sb["closed_at"] = closed_at
    if max_usd is not None:
        sb["cost_limits"] = [dict(pool="codex", node=None, max_usd=max_usd, marks_usd=[], set_by="user")]
    path = os.path.join(d, "record.json")
    with open(path, "w") as fh:
        json.dump(dict(scope_budget=sb), fh)
    return path


class Window(unittest.TestCase):
    CALLS = [call("2026-01-01T00:20:00Z"), call("2026-01-01T00:40:00Z"), call("2026-01-01T01:10:00Z")]

    def tree(self, path, calls=None, **kw):
        calls = self.CALLS if calls is None else calls
        with patch.object(L, "build_codex", return_value=(calls, [], [], 0)), \
                patch.object(L, "build_claude", return_value=([], [], 0)), \
                patch.object(L, "unrecorded_claude", return_value=[]), \
                patch.object(L, "transcript_available", return_value=True):
            return L.cost_tree(path, role_ledger=os.path.join(os.path.dirname(path), "absent"), **kw)[0]

    def test_session_from_before_started_at_is_counted(self):
        with tempfile.TemporaryDirectory() as d:
            tree = self.tree(record(d, "2026-01-01T00:30:00Z"))
        self.assertEqual(tree["window"]["since"], "2026-01-01T00:30:00Z")
        self.assertEqual(tree["window"]["since_basis"], "earliest_session_from")
        self.assertEqual(tree["window"]["started_at"], "2026-01-01T01:00:00Z")
        self.assertEqual(tree["totals"]["codex"]["calls"], 2)          # 00:40 orientation + 01:10
        self.assertEqual(tree["window_totals"]["codex"]["calls"], 2)   # 00:20 is before every authorized start

    def test_started_at_is_the_start_when_no_session_is_earlier(self):
        with tempfile.TemporaryDirectory() as d:
            tree = self.tree(record(d, "2026-01-01T01:00:00Z"))
        self.assertEqual((tree["window"]["since"], tree["window"]["since_basis"]), ("2026-01-01T01:00:00Z", "started_at"))
        self.assertEqual(tree["totals"]["codex"]["calls"], 1)

    def test_explicit_since_still_wins(self):
        with tempfile.TemporaryDirectory() as d:
            tree = self.tree(record(d, "2026-01-01T00:30:00Z"), since_arg="2026-01-01T00:10:00Z")
        self.assertEqual(tree["window"]["since_basis"], "argument")

    def test_limit_trips_on_pre_init_spend(self):
        with tempfile.TemporaryDirectory() as d:
            path = record(d, "2026-01-01T00:30:00Z", max_usd=1.5)
            args = argparse.Namespace(record=path, cursor=None, limit_fractions="1", role_host_ledger=os.path.join(d, "absent"))
            out = io.StringIO()
            with patch.object(L, "build_codex", return_value=(self.CALLS, [], [], 0)), \
                    patch.object(L, "build_claude", return_value=([], [], 0)), \
                    patch.object(L, "unrecorded_claude", return_value=[]), \
                    patch.object(L, "transcript_available", return_value=True), contextlib.redirect_stdout(out):
                L.cmd_control(args)
        doc = json.loads(out.getvalue())
        control = doc["snapshot"]["cost_controls"][0]
        self.assertEqual(control["observed"]["quota_usd_high"], 2.0)
        self.assertTrue(any(e.get("kind") == "limit_reached" for e in doc["events"]), doc["events"])

    def test_closed_record_reconciles_all_in_and_flags_breach_without_editing_record(self):
        calls = self.CALLS + [call("2026-01-01T01:40:00Z")]
        with tempfile.TemporaryDirectory() as d:
            path = record(d, "2026-01-01T00:30:00Z", closed_at="2026-01-01T01:30:00Z", max_usd=2.5)
            before = L.digest(path)
            tree = self.tree(path, calls)
            after = L.digest(path)
        self.assertEqual(before, after)
        self.assertEqual(tree["totals"]["codex"]["calls"], 2)
        self.assertEqual(tree["post_close"]["candidate_totals"]["codex"]["calls"], 1)
        allin = tree["reconciled_all_in"]
        self.assertEqual(allin["totals"]["codex"]["calls"], 3)
        self.assertEqual(allin["totals"]["codex"]["quota_usd_high"], 3.0)
        self.assertEqual(allin["status"], "complete")
        [b] = allin["hard_limit_breaches"]
        self.assertEqual((b["status"], b["in_window_quota_usd_high"], b["post_close_quota_usd_high"]), ("exceeded", 2.0, 1.0))

    def test_no_breach_when_all_in_is_within_the_limit(self):
        with tempfile.TemporaryDirectory() as d:
            tree = self.tree(record(d, "2026-01-01T00:30:00Z", closed_at="2026-01-01T01:30:00Z", max_usd=5))
        self.assertEqual(tree["reconciled_all_in"]["hard_limit_breaches"], [])

    def test_unpriced_post_close_call_is_not_established_not_zero(self):
        calls = self.CALLS + [call("2026-01-01T01:40:00Z", usd=None)]
        with tempfile.TemporaryDirectory() as d:
            tree = self.tree(record(d, "2026-01-01T00:30:00Z", closed_at="2026-01-01T01:30:00Z", max_usd=5), calls)
        [b] = tree["reconciled_all_in"]["hard_limit_breaches"]
        self.assertEqual(b["status"], "not_established")


class InitSessionFrom(unittest.TestCase):
    def run_init(self, d, *extra):
        return subprocess.run([sys.executable, SCOPE, "init", "--record", os.path.join(d, "r.json"), "--objective", "o",
                               "--item", "i", "--session", "11111111-1111-4111-8111-111111111111", "--provider", "claude",
                               "--pool", "claude_all_models", "--max-usd", "1", "--set-by", "user", *extra],
                              capture_output=True, text=True)

    def test_orientation_start_is_recorded(self):
        with tempfile.TemporaryDirectory() as d:
            r = self.run_init(d, "--session-from", "2026-01-01T00:00:00Z", "--session-from-evidence", "first transcript entry")
            self.assertEqual(r.returncode, 0, r.stderr)
            with open(os.path.join(d, "r.json")) as fh:
                sb = json.load(fh)["scope_budget"]
        self.assertEqual(sb["sessions"][0]["from"], "2026-01-01T00:00:00Z")
        self.assertEqual(sb["sessions"][0]["evidence"], "first transcript entry")
        self.assertEqual(L.window_start(sb), ("2026-01-01T00:00:00Z", "earliest_session_from"))

    def test_future_session_from_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            r = self.run_init(d, "--session-from", "2999-01-01T00:00:00Z")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("must not be after", r.stderr)


if __name__ == "__main__":
    unittest.main()
