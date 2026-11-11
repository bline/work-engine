"""Successor accounting: a declared successor's calls leave the predecessor's post-close tail, once each."""
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


def call(ts, sid="root"):
    return dict(provider="codex", thread=sid, parent_thread=None, ts=ts, model="gpt-6.1-sol",
                service_tier="default", quota_multiplier=1, usd=1.0, pools=["codex"])


# root continues from the predecessor into the successor; kid is root's codex child; side is a predecessor binding.
CALLS = [call("2026-01-01T01:10:00Z"), call("2026-01-01T01:20:00Z"), call("2026-01-01T01:25:00Z", "kid"),
         call("2026-01-01T01:40:00Z"), call("2026-01-01T01:45:00Z", "kid"), call("2026-01-01T01:45:00Z", "side"),
         call("2026-01-01T01:50:00Z")]


class Successor(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup); self.d = self.tmp.name

    def write(self, name, **sb):
        path = os.path.join(self.d, name)
        with open(path, "w") as fh:
            json.dump(dict(scope_budget=sb), fh)
        return path

    def pair(self, succ_start="2026-01-01T01:30:00Z", declared=None, oid="succ", pred_limit=10, closed="2026-01-01T01:30:00Z"):
        succ = self.write("succ.json", objective_id="succ", started_at=succ_start, bindings=[],
                          sessions=[{"session_id": "root", "provider": "codex", "from": succ_start}])
        pred = dict(objective_id="pred", started_at="2026-01-01T01:00:00Z",
                    sessions=[{"session_id": "root", "provider": "codex", "from": "2026-01-01T01:00:00Z"}],
                    bindings=[{"session_id": "side", "provider": "codex", "parent_session_id": "root"}],
                    cost_limits=[dict(pool="codex", node=None, max_usd=pred_limit, marks_usd=[], set_by="user")],
                    successor=dict(objective_id=oid, record=succ, authorized_start=declared or succ_start))
        if closed:
            pred["closed_at"] = closed
        return self.write("pred.json", **pred), succ

    def tree(self, path):
        def codex(*a, **kw):
            kw["metadata"].update({"kid": {"parent_thread_id": "root"}})
            return CALLS, [], [], 0
        with patch.object(L, "build_codex", side_effect=codex), patch.object(L, "build_claude", return_value=([], [], 0)), \
                patch.object(L, "unrecorded_claude", return_value=[]), patch.object(L, "transcript_available", return_value=True):
            return L.cost_tree(path, role_ledger=os.path.join(self.d, "absent"))[0]

    def without_successor(self, path):
        with open(path) as fh: doc = json.load(fh)
        doc["scope_budget"].pop("successor")
        return self.write("plain.json", **doc["scope_budget"])

    def test_old_behaviour_counts_successor_calls_as_predecessor_tail(self):
        pred, _ = self.pair(pred_limit=4)
        plain = self.tree(self.without_successor(pred))
        self.assertEqual(plain["post_close"]["candidate_totals"]["codex"]["calls"], 4)
        self.assertEqual(plain["reconciled_all_in"]["hard_limit_breaches"][0]["status"], "exceeded")

    def test_valid_successor_assigns_each_call_once(self):
        pred, succ = self.pair(pred_limit=4)
        p, s = self.tree(pred), self.tree(succ)
        tail = p["post_close"]
        self.assertTrue(tail["successor"]["valid"], tail["successor"])
        self.assertEqual(p["totals"]["codex"]["calls"], 3)                     # 01:10, 01:20, kid 01:25
        self.assertEqual(tail["candidate_totals"]["codex"]["calls"], 1)        # predecessor binding stays
        self.assertEqual(tail["successor_excluded"]["calls"], 3)               # root 01:40/01:50 + kid 01:45
        self.assertEqual((tail["successor_excluded"]["totals"]["codex"]["calls"], tail["successor_excluded"]["totals"]["codex"]["quota_usd_high"]), (3, 3.0))
        self.assertEqual(s["totals"]["codex"]["calls"], 3)                     # = what the predecessor excluded
        self.assertEqual(3 + 1 + 3, len(CALLS))                                # conservation: nothing lost
        self.assertEqual(p["reconciled_all_in"]["totals"]["codex"]["calls"], 4)
        self.assertEqual(p["reconciled_all_in"]["hard_limit_breaches"], [])
        self.assertEqual(p["reconciled_all_in"]["status"], "complete")

    def test_successor_does_not_hide_a_prior_overrun(self):
        pred, _ = self.pair(pred_limit=2.5)
        [b] = self.tree(pred)["reconciled_all_in"]["hard_limit_breaches"]
        self.assertEqual((b["status"], b["in_window_quota_usd_high"]), ("exceeded", 3.0))

    def assert_error(self, pred, text):
        t = self.tree(pred)
        self.assertIn(text, t["post_close"]["successor"]["error"])
        self.assertEqual(t["reconciled_all_in"]["status"], "accounting_error")
        self.assertEqual(t["post_close"]["candidate_totals"]["codex"]["calls"], 4)   # full tail kept, never the cheaper one
        self.assertNotIn("successor_excluded", t["post_close"])
        self.assertIn("successor_boundary_error", [i["kind"] for i in t["issues"]])

    def test_overlapping_boundary_is_an_error(self):
        self.assert_error(self.pair(succ_start="2026-01-01T01:15:00Z")[0], "overlaps")

    def test_declared_start_must_match_the_successor_record(self):
        self.assert_error(self.pair(declared="2026-01-01T01:35:00Z")[0], "differs from the successor record start")

    def test_objective_id_must_match(self):
        self.assert_error(self.pair(oid="someone-else")[0], "objective_id differs")

    def test_missing_successor_record_is_an_error(self):
        pred, succ = self.pair(); os.remove(succ)
        self.assert_error(pred, "unreadable")

    def test_open_predecessor_with_successor_is_an_error(self):
        t = self.tree(self.pair(closed=None)[0])
        self.assertIn("successor_boundary_error", [i["kind"] for i in t["issues"]])
        self.assertEqual(t["status"], "partial")

    def test_scope_budget_declares_without_touching_the_closure(self):
        pred, succ = self.pair()
        doc = json.load(open(pred)); sb = doc["scope_budget"]; sb.pop("successor"); sb["close"] = {"at": sb["closed_at"], "disposition": "accepted"}
        json.dump(doc, open(pred, "w"))
        r = subprocess.run([sys.executable, SCOPE, "successor", "--record", pred, "--successor-record", succ, "--set-by", "user", "--reason", "r"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        after = json.load(open(pred))["scope_budget"]
        self.assertEqual((after["close"], after["closed_at"]), (sb["close"], sb["closed_at"]))
        self.assertEqual((after["successor"]["objective_id"], after["successor"]["authorized_start"]), ("succ", "2026-01-01T01:30:00Z"))
        again = subprocess.run([sys.executable, SCOPE, "successor", "--record", pred, "--successor-record", succ, "--set-by", "user", "--reason", "r"], capture_output=True, text=True)
        self.assertNotEqual(again.returncode, 0)

    def test_scope_budget_refuses_an_overlapping_successor(self):
        pred, succ = self.pair(succ_start="2026-01-01T01:15:00Z")
        doc = json.load(open(pred)); doc["scope_budget"].pop("successor"); json.dump(doc, open(pred, "w"))
        r = subprocess.run([sys.executable, SCOPE, "successor", "--record", pred, "--successor-record", succ, "--set-by", "user", "--reason", "r"], capture_output=True, text=True)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("must not be before", r.stderr)


if __name__ == "__main__":
    unittest.main()
