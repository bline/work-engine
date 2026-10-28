"""Tests for the F1.1 additions to metrics/scripts/usage_ledger.py: service tier, latency and turns.
Run: python3 -m unittest discover -s metrics/scripts/tests -p 'test_usage_ledger_f11.py'
"""
import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import usage_ledger as L  # noqa: E402


def line(ts, typ, payload, ordinal=[0]):
    ordinal[0] += 1
    return json.dumps({"timestamp": ts, "ordinal": ordinal[0], "type": typ, "payload": payload}, separators=(",", ":"))


def usage(out):
    return {"input_tokens": 1000, "cached_input_tokens": 800, "cache_write_input_tokens": 0,
            "output_tokens": out, "reasoning_output_tokens": 0, "total_tokens": 1000 + out}


def record(ts, rid, out, thread="T1"):
    return line(ts, "token_usage_record", {"thread_id": thread, "response_id": rid, "usage": usage(out)})


def settings(ts, tier, thread="T1"):
    return line(ts, "event_msg", {"type": "thread_settings_applied", "thread_id": thread,
                                  "thread_settings": {"model": "gpt-6.1-sol", "service_tier": tier}})


ROLLOUT = [
    json.dumps({"timestamp": "2026-10-28T10:00:00.000Z", "type": "session_meta", "payload": {"id": "T1"}}, separators=(",", ":")),
    settings("2026-10-28T10:00:00.500Z", "default"),
    line("2026-10-28T10:00:01.000Z", "turn_context", {"turn_id": "u1", "model": "gpt-6.1-sol", "effort": "xhigh"}),
    line("2026-10-28T10:00:01.000Z", "event_msg", {"type": "task_started", "turn_id": "u1"}),
    line("2026-10-28T10:00:02.000Z", "response_item", {"type": "message", "id": "m1", "role": "user", "content": []}),
    record("2026-10-28T10:00:12.000Z", "r1", 100),                      # 10 s after the user message
    line("2026-10-28T10:00:14.000Z", "response_item", {"type": "function_call_output", "call_id": "c1", "output": "x" * 50}),
    record("2026-10-28T10:00:20.000Z", "r2", 60),                       # 6 s after the tool output
    record("2026-10-28T10:00:25.000Z", "r3", 40),                       # no input since r2
    settings("2026-10-28T10:00:26.000Z", "priority", thread="OTHER"),   # another thread's settings: ignored
    settings("2026-10-28T10:00:27.000Z", "priority"),
    line("2026-10-28T10:00:28.000Z", "response_item", {"type": "custom_tool_call_output", "call_id": "c2", "output": "y"}),
    record("2026-10-28T10:00:31.000Z", "r4", 30),                       # 3 s, now Fast
    line("2026-10-28T10:00:32.000Z", "event_msg", {"type": "task_complete", "turn_id": "u1", "started_at": 1, "completed_at": 32,
                                                   "duration_ms": 31000, "time_to_first_token_ms": 4000}),
]


class CodexTierAndTime(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "rollout-test.jsonl")
        with open(self.path, "w") as fh:
            fh.write("\n".join(ROLLOUT) + "\n")
        L.ISSUES.clear()

    def tearDown(self):
        self.dir.cleanup()

    def test_tier_follows_the_thread_and_switches_mid_thread(self):
        turns = []
        meta, calls, _, _ = L.codex_rollout(self.path, {}, turns)
        self.assertEqual([c["response_id"] for c in calls], ["r1", "r2", "r3", "r4"])
        self.assertEqual([c["tier"] for c in calls], ["default", "default", "default", "priority"])
        self.assertEqual([c["latency_s"] for c in calls], [10.0, 6.0, None, 3.0])
        self.assertEqual([c["latency_basis"] for c in calls],
                         ["since_preceding_input", "since_preceding_input", "no_preceding_input", "since_preceding_input"])
        self.assertEqual(len(turns), 1)
        t = turns[0]
        self.assertEqual((t["turn_id"], t["duration_ms"], t["time_to_first_token_ms"], t["service_tier"]), ("u1", 31000, 4000, "priority"))
        self.assertEqual((t["model"], t["effort"]), ("gpt-6.1-sol", "xhigh"))

    def test_f1_call_fields_are_unchanged(self):
        _, calls, _, _ = L.codex_rollout(self.path, {})
        f1_keys = {"ts", "response_id", "usage", "model", "effort", "line", "dedupe_basis"}
        self.assertTrue(all(f1_keys <= set(c) for c in calls))
        self.assertEqual(calls[0]["usage"], usage(100))
        self.assertEqual((calls[0]["model"], calls[0]["effort"], calls[0]["dedupe_basis"]), ("gpt-6.1-sol", "xhigh", "response_id"))

    def test_quota_multiplier_and_summary(self):
        self.assertEqual(L.QUOTA_MULTIPLIER.get("priority"), 2.5)
        self.assertEqual(L.QUOTA_MULTIPLIER.get("default"), 1.0)
        self.assertIsNone(L.QUOTA_MULTIPLIER.get(None))
        rows = [dict(provider="codex", model="gpt-6.1-sol", effort="xhigh", service_tier="priority", quota_multiplier=2.5, usd=1.0, latency_s=3.0),
                dict(provider="codex", model="gpt-6.1-sol", effort="xhigh", service_tier=None, quota_multiplier=None, usd=1.0, latency_s=None)]
        s = L.f11_summary(rows, [dict(duration_ms=31000)])
        fast = s["by_cell"]["codex|gpt-6.1-sol|xhigh|priority"]
        self.assertEqual((fast["quota_weighted_usd"], fast["latency_n"], fast["latency_median_s"]), (2.5, 1, 3.0))
        unknown = s["by_cell"]["codex|gpt-6.1-sol|xhigh|None"]
        self.assertEqual((unknown["quota_weighted_usd"], unknown["quota_unweighted_calls"]), (0.0, 1))
        self.assertEqual((s["turns"], s["turn_duration_median_ms"]), (1, 31000))


class ClaudeTierAndTime(unittest.TestCase):
    def test_latency_spans_the_message_records_and_tier_is_read(self):
        with tempfile.TemporaryDirectory() as d:
            proj = os.path.join(d, ".claude", "projects", "p")
            os.makedirs(proj)
            path = os.path.join(proj, "s1.jsonl")

            def assistant(ts, mid, out, speed="fast"):
                return json.dumps({"parentUuid": "a", "isSidechain": False,
                                   "message": {"id": mid, "model": "claude-opus-5-5",
                                               "usage": {"input_tokens": 10, "cache_creation_input_tokens": 0,
                                                         "cache_read_input_tokens": 0, "output_tokens": out,
                                                         "service_tier": "standard", "speed": speed}},
                                   "type": "assistant", "timestamp": ts, "sessionId": "s1"}, separators=(",", ":"))
            user = json.dumps({"parentUuid": None, "isSidechain": False, "promptId": "p1", "type": "user",
                               "message": {"role": "user", "content": "go"}, "timestamp": "2026-10-28T10:00:00.000Z"},
                              separators=(",", ":"))
            with open(path, "w") as fh:
                fh.write("\n".join([user, assistant("2026-10-28T10:00:02.000Z", "m1", 5),
                                    assistant("2026-10-28T10:00:05.000Z", "m1", 9),
                                    assistant("2026-10-28T10:00:07.000Z", "m2", 3, speed="standard")]) + "\n")
            msgs = {}
            L.CLAUDE_LATENCY_START.clear()
            L.claude_transcript(path, msgs, {})
            self.assertEqual((msgs["m1"]["latency_s"], msgs["m1"]["service_tier"], msgs["m1"]["speed"]), (5.0, "standard", "fast"))
            self.assertEqual(msgs["m1"]["usage"]["output_tokens"], 9)
            self.assertEqual((msgs["m2"]["latency_s"], msgs["m2"]["latency_basis"]), (None, "no_preceding_input"))
            self.assertIsNone(msgs["m1"]["quota_multiplier"])


if __name__ == "__main__":
    unittest.main()
