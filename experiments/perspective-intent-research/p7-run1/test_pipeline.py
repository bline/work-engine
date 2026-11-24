#!/usr/bin/env python3
"""Offline tests for the P7 run 1 pipeline and build check, on canned outputs and receipts.

Run: python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
"""

import json
import os
import sys
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_tasks  # noqa: E402
import pipeline  # noqa: E402

SCHEMA = "obligation-extract/v3"
DOCS = {"DOC-1": "The worker must delete its file. Someone must confirm the file is gone."}
REVIEW_DOCS = {"DOC-1": "The worker must delete its file.", "DOC-2": "The review requires the worker to delete its file."}
PREAMBLE_CONTENT = "You answer only from the documents."
SECTIONS = {"preamble": PREAMBLE_CONTENT, "cwd": "/scratch"}


def pinned_expectations():
    exp = json.loads(json.dumps(pipeline.load_json(os.path.join(HERE, "receipt_expectations.json"))))
    digest = pipeline.sha256(json.dumps([PREAMBLE_CONTENT], ensure_ascii=False))
    for a in exp["agents"].values():
        a["system_content_sha256"] = digest
    exp["user_wrapper_sha256"] = pipeline.sha256("{{TASK}}")
    exp["pinned"] = True
    return exp


class Fixture:
    def __init__(self, base):
        self.base, self.n = base, 0

    def run_dir(self, task, output, agent="p7-arm", tools_added=(), sections=None, model="claude-sdk/claude-opus-5-5",
                provider="claude-sdk", assistant_model="claude-opus-5-5", skills=None, tool_execs=0, state="complete",
                exit_code=0, users=1, assistant=True, drop=(), native_id=None):
        self.n += 1
        d = os.path.join(self.base, f"rd{self.n}")
        os.makedirs(d)
        step = {"agent": agent, "model": model, "thinking": "high", "exitCode": exit_code, "context": "fresh",
                "skills": skills or [], "sessionFile": f"/s/{self.n}.jsonl", "tokens": {"input": 10, "output": 5}}
        for k in drop:
            step.pop(k, None)
        with open(os.path.join(d, "status.json"), "w") as f:
            json.dump({"runId": native_id or f"run-{self.n}", "state": state, "steps": [step], "launchContractDigest": "x"}, f)
        system = {"role": "system", "content": PREAMBLE_CONTENT, "sections": SECTIONS if sections is None else sections,
                  "toolsAdded": list(tools_added)}
        if "toolsAdded" in drop:
            system.pop("toolsAdded")
        events = [] if "system" in drop else [{"type": "message_end", "message": system}]
        events += [{"type": "message_end", "message": {"role": "user", "content": [{"type": "text", "text": task}]}}] * users
        events += [{"type": "tool_execution_start", "toolName": "read"}] * tool_execs
        if assistant:
            events.append({"type": "message_end", "message": {
                "role": "assistant", "provider": provider, "model": assistant_model, "thinkingLevel": "high", "stopReason": "stop",
                "content": [{"type": "thinking", "thinking": "private"}, {"type": "text", "text": output}],
                "usage": {"input": 10, "output": 5}, "durationMs": 100}})
        with open(os.path.join(d, "events.jsonl"), "w") as f:
            for e in events:
                f.write(json.dumps(e) + "\n")
        return d

    def file(self, text):
        self.n += 1
        p = os.path.join(self.base, f"out{self.n}.txt")
        with open(p, "w") as f:
            f.write(text)
        return p


def extraction(ambiguous=False, span="The worker must delete its file."):
    owner = ({"status": "ambiguous", "value": [], "readings": [["acting_component"], ["any"]]} if ambiguous
             else {"status": "binding", "value": ["acting_component"]})
    return json.dumps({"schema": SCHEMA, "entries": [{"id": "e1", "document": "DOC-1", "role": "parent", "span": span,
                                                       "fields": {"outcome_owner": owner,
                                                                  "acceptance_evidence": {"status": "binding", "value": ["outcome_observed"]}}}]})


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        base = self.tmp.name
        self.fx = Fixture(base)
        pipeline.RUNS = os.path.join(base, "runs")
        pipeline.TASKS = os.path.join(base, "tasks")
        os.makedirs(pipeline.RUNS)
        os.makedirs(pipeline.TASKS)
        self.patch = mock.patch.object(pipeline, "expectations", pinned_expectations)
        self.patch.start()
        plan_meta = {"kind": "plan", "order": ["DOC-1"], "parents": ["DOC-1"], "derived": [], "documents": DOCS, "id_map": {"NC-A": "DOC-1"}}
        review_meta = {"kind": "review", "order": ["DOC-1", "DOC-2"], "parents": ["DOC-1"], "derived": ["DOC-2"],
                       "documents": REVIEW_DOCS, "id_map": {"NC-B-CONTRACT": "DOC-1", "NC-B-REVIEW": "DOC-2"}}
        self.tasks = {"NC-A.M1": "Plan task M1", "NC-A.M2": "Plan task M2", "NC-B.M1": "Review task M1"}
        for tid, body in self.tasks.items():
            with open(os.path.join(pipeline.TASKS, tid + ".md"), "w") as f:
                f.write(body)
        runs = []
        for set_id, methods in (("NC-A", ("M1", "M2", "M3")), ("NC-B", ("M1",))):
            for m in methods:
                for rep in (1, 2):
                    tid = f"{set_id}.{'M2' if m == 'M3' else m}"
                    runs.append({"run_id": f"{set_id}.{m}.r{rep}", "input_set": set_id, "method": m, "repeat": rep,
                                 "task_id": tid, "task_sha256": pipeline.sha256(self.tasks[tid])})
        with open(os.path.join(pipeline.RUNS, "plan.json"), "w") as f:
            json.dump({"runs": runs, "task_documents": {"NC-A.M1": plan_meta, "NC-A.M2": plan_meta, "NC-B.M1": review_meta}}, f)

    def tearDown(self):
        self.patch.stop()
        self.tmp.cleanup()

    def attempt(self, run_id, output, **kw):
        row = pipeline.run_row(run_id)
        return pipeline.record_attempt(run_id, self.fx.file(output), self.fx.run_dir(self.tasks[row["task_id"]], output, **kw))


class Receipts(Base):
    def rc(self, output="ok", task="Plan task M1", calibration=False, **kw):
        return pipeline.extract_receipt(self.fx.run_dir(task, output, **kw), kw.get("agent", "p7-arm"), task, output, calibration)

    def test_clean_receipt(self):
        rc = self.rc()
        self.assertEqual((rc["violations"], rc["provider_failure"], rc["unestablished"]), ([], [], []))
        self.assertEqual(rc["run_id"], "run-1")

    def test_violations(self):
        cases = {"declared but unused tool": dict(tools_added=[{"name": "read"}]), "tool execution": dict(tool_execs=1),
                 "extra section": dict(sections=dict(SECTIONS, project_context="AGENTS.md")),
                 "step model": dict(model="claude-sdk/claude-sonnet-5-5"), "provider": dict(provider="anthropic"),
                 "actual model": dict(assistant_model="claude-sonnet-5-5"), "skills": dict(skills=["x"]),
                 "second user turn": dict(users=2)}
        for name, kw in cases.items():
            self.assertTrue(self.rc(**kw)["violations"], name)
            self.assertEqual(pipeline.receipt_status(self.rc(**kw)), "invalid_receipt", name)

    def test_task_and_output_custody(self):
        d = self.fx.run_dir("Plan task M1", "the real answer")
        self.assertIn("user message does not carry the planned task text",
                      pipeline.extract_receipt(d, "p7-arm", "Another task", "the real answer")["violations"])
        self.assertIn("output file does not match the final assistant text",
                      pipeline.extract_receipt(d, "p7-arm", "Plan task M1", "a different answer")["violations"])

    def test_missing_evidence_is_unestablished_not_zero(self):
        for drop in (("toolsAdded",), ("system",), ("context",), ("skills",)):
            rc = self.rc(drop=drop)
            self.assertEqual(pipeline.receipt_status(rc), "receipt_unestablished", drop)

    def test_unpinned_expectations_block_counted_but_not_calibration(self):
        self.patch.stop()
        try:
            self.assertEqual(pipeline.receipt_status(self.rc()), "receipt_unestablished")
            self.assertIsNone(pipeline.receipt_status(self.rc(calibration=True)))
        finally:
            self.patch.start()

    def test_provider_failure_is_separate(self):
        for kw in (dict(state="failed", exit_code=1), dict(assistant=False, state="failed", exit_code=1)):
            self.assertEqual(pipeline.receipt_status(self.rc(**kw)), "provider_failure", kw)


class Attempts(Base):
    def test_valid_then_no_retry(self):
        a = self.attempt("NC-A.M1.r1", "Analysis.\nFLAGS: none")
        self.assertEqual(a["status"], "valid")
        with self.assertRaises(pipeline.PipelineError):
            self.attempt("NC-A.M1.r1", "Another try\nFLAGS: none")
        self.assertEqual(pipeline.final_attempt("NC-A.M1.r1")["attempt"], 1)

    def test_invalid_extraction_is_terminal(self):
        a = self.attempt("NC-A.M2.r1", "{not json")
        self.assertEqual(a["status"], "invalid_extraction")
        with self.assertRaises(pipeline.PipelineError):
            self.attempt("NC-A.M2.r1", extraction())
        bad_quote = self.attempt("NC-A.M2.r2", extraction(span="invented text"))
        self.assertEqual(bad_quote["status"], "invalid_extraction")
        self.assertEqual(bad_quote["quote_failures"][0]["where"], "span")

    def test_prose_completeness(self):
        self.assertEqual(self.attempt("NC-A.M1.r1", "I could not produce the flags")["status"], "incomplete")
        self.assertEqual(self.attempt("NC-B.M1.r1", "## FLAGS\nDOC-1: none")["status"], "incomplete")
        self.assertEqual(self.attempt("NC-B.M1.r2", "## FLAGS\nDOC-2: none")["status"], "valid")

    def test_provider_failure_allows_one_replacement_then_stops(self):
        a = self.attempt("NC-A.M1.r1", "x", state="failed", exit_code=1)
        self.assertEqual(a["status"], "provider_failure")
        self.assertEqual(pipeline.stop_reasons(), [])
        b = self.attempt("NC-A.M1.r1", "y", state="failed", exit_code=1)
        self.assertTrue(b["replacement"])
        self.assertTrue(pipeline.stop_reasons())
        with self.assertRaises(pipeline.PipelineError):
            self.attempt("NC-A.M1.r2", "FLAGS: none")

    def test_invalid_receipt_stops_everything(self):
        a = self.attempt("NC-A.M1.r1", "FLAGS: none", tools_added=[{"name": "read"}])
        self.assertEqual(a["status"], "invalid_receipt")
        with self.assertRaises(pipeline.PipelineError):
            self.attempt("NC-A.M1.r2", "FLAGS: none")
        self.assertEqual(pipeline.main(["pipeline.py", "gate"]), pipeline.STOP_EXIT)

    def test_native_run_reuse_refused(self):
        self.attempt("NC-A.M1.r1", "FLAGS: none", native_id="same")
        with self.assertRaises(pipeline.PipelineError):
            self.attempt("NC-A.M1.r2", "FLAGS: none", native_id="same")
        self.assertIsNone(pipeline.final_attempt("NC-A.M1.r2"))

    def test_leakage_is_adjudicated_against_the_independent_assessment(self):
        a = self.attempt("NC-A.M1.r1", "the verifier claims it is gone, and no section")
        self.assertEqual((a["status"], a["assessed_status"]), ("suspected_leakage", "incomplete"))
        pipeline.adjudicate_leakage("attempts/NC-A.M1.r1/1/attempt.json", "coincidental", "ordinary phrase")
        self.assertEqual(pipeline.final_attempt("NC-A.M1.r1")["effective"], "incomplete")
        b = self.attempt("NC-A.M1.r2", "the verifier claims it is gone\nFLAGS: none")
        pipeline.adjudicate_leakage("attempts/NC-A.M1.r2/1/attempt.json", "exposure", "withheld phrase")
        self.assertEqual(pipeline.final_attempt("NC-A.M1.r2")["effective"], "void_leakage")
        c = self.attempt("NC-A.M1.r2", "FLAGS: none")
        self.assertEqual((c["attempt"], c["status"]), (2, "valid"))
        with self.assertRaises(pipeline.PipelineError):
            pipeline.adjudicate_leakage("attempts/NC-A.M1.r2/1/attempt.json", "coincidental", "again")


class FallbackAndCompare(Base):
    def test_m3_pairing_and_complete_batch(self):
        self.attempt("NC-A.M3.r1", extraction(ambiguous=True))
        tasks = pipeline.fallback_tasks("NC-A.M3.r1")
        self.assertEqual(len(tasks), 1)
        with self.assertRaises(pipeline.PipelineError):
            pipeline.compare("NC-A.M3.r1")  # incomplete batch
        wrong = json.dumps({"entry": "e1", "property": "verifier", "status": "binding", "value": ["any"], "reason": "x"})
        rd = self.fx.run_dir(tasks[0]["body"], wrong)
        rec = pipeline.record_fallback("NC-A.M3.r1", 0, self.fx.file(wrong), rd)
        self.assertEqual((rec["status"], rec["error"]), ("invalid_response", "response names a different target"))
        rep = pipeline.compare("NC-A.M3.r1")
        self.assertEqual(rep["fallback_log"][0]["outcome"], "invalid_response")
        self.assertEqual(pipeline.compare("NC-A.M3.r1"), rep)  # deterministic recomputation, no overwrite

    def test_m3_resolution(self):
        self.attempt("NC-A.M3.r1", extraction(ambiguous=True))
        tasks = pipeline.fallback_tasks("NC-A.M3.r1")
        ok = json.dumps({"entry": "e1", "property": "outcome_owner", "status": "binding", "value": ["acting_component"], "reason": "named"})
        pipeline.record_fallback("NC-A.M3.r1", 0, self.fx.file(ok), self.fx.run_dir(tasks[0]["body"], ok))
        rep = pipeline.compare("NC-A.M3.r1")
        self.assertIn("owner_unresolved", {f["flag"] for f in rep["pre"]["plan_flags"]})
        self.assertNotIn("owner_unresolved", {f["flag"] for f in rep["post"]["plan_flags"]})


class Scoring(Base):
    def reply(self, sid, phases, items, census=None, evidence=None):
        return json.dumps({"input": sid, "phases": [
            {"phase": ph, "items": [{"id": i, "answer": a, "evidence": (evidence or {}).get(i, pipeline.NO_SPAN)} for i, a in items.items()],
             "flag_census": census or [], "weakening": {"answer": "no", "evidence": ""}} for ph in phases]})

    def score(self, entry, text):
        task = pipeline.read_text(os.path.join(pipeline._batch_dir(int(entry["score_id"][1])), entry["score_id"], "task.md"))
        return pipeline.record_score(entry["score_id"], self.fx.file(text), self.fx.run_dir(task, text, agent="p7-scorer"))

    def test_mixed_batch_through_item_outcomes(self):
        self.attempt("NC-A.M1.r1", "The worker owns deletion.\nFLAGS: none")
        self.attempt("NC-A.M1.r2", "nothing useful")  # incomplete: counted in attempted, not valid
        self.attempt("NC-A.M2.r1", extraction())
        pipeline.compare("NC-A.M2.r1")
        key = pipeline.score_tasks()
        self.assertEqual(pipeline.score_tasks(), key)  # unchanged eligible set: same batch, no writes
        entries = {e["run_id"]: e for e in key["key"]}
        self.assertEqual(set(entries), {"NC-A.M1.r1", "NC-A.M2.r1"})
        prose = entries["NC-A.M1.r1"]
        body = pipeline.read_text(os.path.join(pipeline._batch_dir(1), prose["score_id"], "task.md"))
        self.assertNotIn("NC-A.M1", body)
        self.assertNotIn("NC-A\"", body)
        # An empty reply is invalid; one replacement is allowed.
        empty = self.score(prose, json.dumps({"input": prose["score_id"], "phases": []}))
        self.assertEqual(empty["status"], "invalid_reply")
        good = self.score(prose, self.reply(prose["score_id"], ["prose"], {"P-NCA": "hit", "X6": "not_violated"},
                                            evidence={"P-NCA": "The worker owns deletion."}))
        self.assertEqual((good["status"], good["quote_failures"]), ("valid", []))
        typed = entries["NC-A.M2.r1"]
        # Quotes are phase-local, and the census must cover every typed flag.
        material = json.loads(pipeline.read_text(os.path.join(pipeline._batch_dir(1), typed["score_id"], "material.json")))
        n_flags = len(json.loads(material["pre"].split("COMPARATOR REPORT (pre):\n")[1])["plan_flags"])
        self.assertGreater(n_flags, 0)
        short = self.score(typed, self.reply(typed["score_id"], ["pre"], {"P-NCA": "miss", "X6": "not_violated"}))
        self.assertIn("census", short["errors"][0])
        census = [{"flag": "verifier_unassigned", "class": "false_flag", "source_support": ""}] * n_flags
        ok = self.score(typed, self.reply(typed["score_id"], ["pre"], {"P-NCA": "hit", "X6": "not_violated"}, census,
                                          evidence={"P-NCA": "quote not in the material"}))
        self.assertEqual(ok["quote_failures"], [{"phase": "pre", "id": "P-NCA"}])
        sample = pipeline.human_sample()["scores"]
        self.assertIn("quote failures", sample[typed["score_id"]])
        with open(os.path.join(pipeline.RUNS, "adjudication.json"), "w") as f:
            json.dump([{"score_id": typed["score_id"], "phase": "pre", "item_id": "P-NCA", "scorer_answer": "hit",
                        "operator_answer": "miss", "reason": "the flag concerns the verifier"}], f)
        rep = pipeline.report()
        m1 = rep["items"]["NC-A.M1.prose.P-NCA"]
        self.assertEqual((m1["answers"], m1["valid_runs"], m1["attempted_runs"]), ({"hit": 1}, 1, 2))
        self.assertEqual(m1["hit_rate_valid"], 1.0)
        self.assertEqual(m1["hit_rate_attempted"], 0.5)
        self.assertEqual(rep["items"]["NC-A.M2.pre.P-NCA"]["answers"], {"miss": 1})
        self.assertEqual(rep["census"]["NC-A.M2.pre"], {"false_flag": n_flags})
        self.assertEqual(rep["cells"]["NC-A.M1"]["final_status"], {"valid": 1, "incomplete": 1})
        self.assertEqual(rep["usage_by_stage"]["attempts"]["calls"], 3)
        self.assertEqual(rep["usage_by_stage"]["scores"]["calls"], 2)
        self.assertEqual(rep["usage_by_stage"]["scores-replacement"]["calls"], 2)
        # A changed eligible set needs an explicit new batch; the old batch is kept.
        self.attempt("NC-B.M1.r1", "## FLAGS\nDOC-2: none")
        with self.assertRaises(pipeline.PipelineError):
            pipeline.score_tasks()
        key2 = pipeline.score_tasks(new_batch=True)
        self.assertEqual(key2["batch"], 2)
        self.assertTrue(os.path.exists(os.path.join(pipeline._batch_dir(1), prose["score_id"], "reply-2.json")))


class BuildCheck(unittest.TestCase):
    def test_current_artifacts_verify(self):
        tasks, plan = build_tasks.expected_artifacts()
        self.assertEqual(build_tasks.verify(tasks, plan, *build_tasks.read_disk()), [])

    def test_stale_artifacts_rejected(self):
        tasks, plan = build_tasks.expected_artifacts()
        disk_tasks, disk_plan = build_tasks.read_disk()
        first = sorted(disk_tasks)[0]
        altered = dict(disk_tasks, **{first: disk_tasks[first] + b"x"})
        self.assertTrue(build_tasks.verify(tasks, plan, altered, disk_plan))
        crlf = dict(disk_tasks, **{first: disk_tasks[first].replace(b"\n", b"\r\n")})
        self.assertTrue(build_tasks.verify(tasks, plan, crlf, disk_plan))
        missing = {k: v for k, v in disk_tasks.items() if k != first}
        self.assertTrue(build_tasks.verify(tasks, plan, missing, disk_plan))
        self.assertTrue(build_tasks.verify(tasks, plan, dict(disk_tasks, **{"EXTRA.M1": b"x"}), disk_plan))
        self.assertTrue(build_tasks.verify(tasks, plan, disk_tasks, disk_plan.replace(b'"seed"', b'"seed" ', 1)))


if __name__ == "__main__":
    unittest.main()
