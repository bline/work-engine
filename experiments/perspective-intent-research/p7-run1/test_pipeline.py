#!/usr/bin/env python3
"""Offline tests for the P7 run 1 pipeline and build check, on canned outputs and receipts.

Run: python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
"""

import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_tasks  # noqa: E402
import pipeline  # noqa: E402

DOCS = {"DOC-1": "The worker must delete its file. Someone must confirm the file is gone."}
SCHEMA = "obligation-extract/v2"


def make_run_dir(base, name, agent="p7-arm", tool_count=0, sections=None, model="claude-sdk/claude-opus-5-5",
                 provider="claude-sdk", skills=None, tool_execs=0):
    d = os.path.join(base, name)
    os.makedirs(d)
    step = {"agent": agent, "model": model, "thinking": "high", "toolCount": tool_count, "exitCode": 0,
            "context": "fresh", "skills": skills or [], "launchContractDigest": "x",
            "launchResolvedExtensions": {"effective": ["sha256:provider"]}, "tokens": {"input": 10, "output": 5}}
    with open(os.path.join(d, "status.json"), "w") as f:
        json.dump({"steps": [step], "toolCount": tool_count}, f)
    sections = sections if sections is not None else {"preamble": "You answer only from the documents.", "cwd": "/scratch"}
    events = [{"type": "message_end", "message": {"role": "system", "content": "", "sections": sections}}]
    events += [{"type": "tool_execution_start", "toolName": "read"}] * tool_execs
    events.append({"type": "message_end", "message": {"role": "assistant", "provider": provider, "model": "claude-opus-5-5",
                                                      "thinkingLevel": "high", "usage": {"input": 10, "output": 5}, "durationMs": 100}})
    with open(os.path.join(d, "events.jsonl"), "w") as f:
        for e in events:
            f.write(json.dumps(e) + "\n")
    return d


def write(base, name, text):
    p = os.path.join(base, name)
    with open(p, "w") as f:
        f.write(text)
    return p


def read_text(path):
    with open(path) as f:
        return f.read()


class PipelineTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = self.tmp.name
        pipeline.RUNS = os.path.join(self.base, "runs")
        os.makedirs(pipeline.RUNS)
        meta = {"kind": "plan", "order": ["DOC-1"], "parents": ["DOC-1"], "derived": [], "documents": DOCS,
                "id_map": {"X-DOC": "DOC-1"}}
        runs = [{"run_id": f"NC-A.{m}.r1", "input_set": "NC-A", "method": m, "repeat": 1,
                 "task_id": f"NC-A.{'M2' if m == 'M3' else m}", "task_sha256": "t"} for m in ("M1", "M2", "M3")]
        with open(os.path.join(pipeline.RUNS, "plan.json"), "w") as f:
            json.dump({"runs": runs, "task_documents": {"NC-A.M1": meta, "NC-A.M2": meta}}, f)

    def tearDown(self):
        self.tmp.cleanup()

    def extraction(self, ambiguous=False):
        owner = ({"status": "ambiguous", "value": [], "readings": [["acting_component"], ["any"]]} if ambiguous
                 else {"status": "binding", "value": ["acting_component"]})
        return {"schema": SCHEMA, "entries": [{"id": "e1", "document": "DOC-1", "role": "parent",
                                               "span": "The worker must delete its file.",
                                               "fields": {"outcome_owner": owner,
                                                          "acceptance_evidence": {"status": "binding", "value": ["outcome_observed"]}}}]}

    # receipts
    def test_receipt_valid(self):
        r = pipeline.extract_receipt(make_run_dir(self.base, "ok"), "p7-arm")
        self.assertEqual(r["problems"], [])

    def test_receipt_rejections(self):
        cases = {
            "tools": dict(tool_count=3),
            "execs": dict(tool_execs=1),
            "bridge": dict(sections={"preamble": "Use contact_supervisor when blocked."}),
            "project": dict(sections={"preamble": "x", "project_context": "AGENTS.md"}),
            "model": dict(model="claude-sdk/claude-sonnet-5-5"),
            "provider": dict(provider="anthropic"),
            "skills": dict(skills=["manual-orchestration"]),
        }
        for name, kw in cases.items():
            r = pipeline.extract_receipt(make_run_dir(self.base, name, **kw), "p7-arm")
            self.assertTrue(r["problems"], name)

    # attempts
    def test_m1_valid_and_incomplete_and_no_overwrite(self):
        rd = make_run_dir(self.base, "rd")
        a1 = pipeline.record_attempt("NC-A.M1.r1", write(self.base, "o1.txt", "Analysis.\nFLAGS: none"), rd)
        self.assertEqual(a1["status"], "valid")
        a2 = pipeline.record_attempt("NC-A.M1.r1", write(self.base, "o2.txt", "Analysis without the section"), rd)
        self.assertEqual((a2["attempt"], a2["status"]), (2, "incomplete"))
        self.assertEqual(pipeline.final_attempt("NC-A.M1.r1")["attempt"], 1)

    def test_m2_valid_invalid_and_leakage(self):
        rd = make_run_dir(self.base, "rd")
        ok = pipeline.record_attempt("NC-A.M2.r1", write(self.base, "e.json", "```json\n" + json.dumps(self.extraction()) + "\n```"), rd)
        self.assertEqual(ok["status"], "valid")
        bad = pipeline.record_attempt("NC-A.M2.r1", write(self.base, "b.json", "{not json"), rd)
        self.assertEqual(bad["status"], "invalid_extraction")
        wrong_span = self.extraction()
        wrong_span["entries"][0]["span"] = "invented text"
        bad2 = pipeline.record_attempt("NC-A.M2.r1", write(self.base, "w.json", json.dumps(wrong_span)), rd)
        self.assertEqual(bad2["status"], "invalid_extraction")
        leak = pipeline.record_attempt("NC-A.M1.r1", write(self.base, "l.txt", "the verifier claims it is gone\nFLAGS: none"), rd)
        self.assertEqual(leak["status"], "suspected_leakage")

    def test_invalid_receipt_is_recorded_not_scored(self):
        rd = make_run_dir(self.base, "rd", tool_count=2)
        a = pipeline.record_attempt("NC-A.M1.r1", write(self.base, "o.txt", "FLAGS: none"), rd)
        self.assertEqual(a["status"], "invalid_receipt")

    # fallback and comparison
    def test_m3_fallback_paired_comparison(self):
        rd = make_run_dir(self.base, "rd")
        pipeline.record_attempt("NC-A.M3.r1", write(self.base, "e.json", json.dumps(self.extraction(ambiguous=True))), rd)
        tasks = pipeline.fallback_tasks("NC-A.M3.r1")
        self.assertEqual(len(tasks), 1)
        self.assertIn('[["acting_component"], ["any"]]', tasks[0]["body"])
        resp = {"entry": "e1", "property": "outcome_owner", "status": "binding", "value": ["acting_component"], "reason": "named"}
        self.assertEqual(pipeline.record_fallback("NC-A.M3.r1", 0, write(self.base, "f.json", json.dumps(resp)), rd)["status"], "valid")
        rep = pipeline.compare("NC-A.M3.r1")
        self.assertIn("owner_ambiguous", {f["flag"] for f in rep["pre"]["plan_flags"]})
        self.assertNotIn("owner_ambiguous", {f["flag"] for f in rep["post"]["plan_flags"]})
        out = pipeline.report()
        self.assertEqual(out["m3_transitions"]["NC-A.M3.r1"]["removed"], 1)

    # scoring
    def test_score_tasks_blind_and_quote_check(self):
        rd = make_run_dir(self.base, "rd")
        pipeline.record_attempt("NC-A.M1.r1", write(self.base, "o.txt", "The worker owns deletion.\nFLAGS: none"), rd)
        key = pipeline.score_tasks()
        self.assertEqual(len(key), 1)
        body = read_text(os.path.join(pipeline.RUNS, "scores", key[0]["score_id"], "task.md"))
        self.assertNotIn("NC-A.M1", body)
        self.assertNotIn("X-DOC", body)
        reply = {"input": key[0]["score_id"], "phases": [{"phase": "prose",
                 "items": [{"id": "P-NCA", "answer": "hit", "evidence": "The worker owns deletion."},
                           {"id": "X6", "answer": "not_violated", "evidence": "a quote that is not in the output"}],
                 "flag_census": [], "weakening": {"answer": "no", "evidence": ""}}]}
        srd = make_run_dir(self.base, "srd", agent="p7-scorer")
        rec = pipeline.record_score(key[0]["score_id"], write(self.base, "s.json", json.dumps(reply)), srd)
        self.assertEqual(rec["status"], "valid")
        self.assertEqual(rec["quote_failures"], [{"phase": "prose", "id": "X6"}])
        self.assertIn(key[0]["score_id"], pipeline.human_sample())


class BuildCheck(unittest.TestCase):
    def test_current_artifacts_verify(self):
        tasks, plan = build_tasks.expected_artifacts()
        self.assertEqual(build_tasks.verify(tasks, plan, *build_tasks.read_disk()), [])

    def test_stale_artifacts_rejected(self):
        tasks, plan = build_tasks.expected_artifacts()
        disk_tasks, disk_plan = build_tasks.read_disk()
        altered = dict(disk_tasks)
        first = sorted(altered)[0]
        altered[first] = altered[first] + "x"
        self.assertTrue(build_tasks.verify(tasks, plan, altered, disk_plan))
        missing = {k: v for k, v in disk_tasks.items() if k != first}
        self.assertTrue(build_tasks.verify(tasks, plan, missing, disk_plan))
        extra = dict(disk_tasks, **{"EXTRA.M1": "x"})
        self.assertTrue(build_tasks.verify(tasks, plan, extra, disk_plan))
        bad_plan = dict(disk_plan, asset_digests=dict(disk_plan["asset_digests"], **{"comparator.py": "0"}))
        self.assertTrue(build_tasks.verify(tasks, plan, disk_tasks, bad_plan))


if __name__ == "__main__":
    unittest.main()
