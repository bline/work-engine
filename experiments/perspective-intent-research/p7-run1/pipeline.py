#!/usr/bin/env python3
"""P7 run 1 offline pipeline, revision 3. No model calls; the executor (RUNBOOK.md)
launches agents and feeds their outputs and native run directories to these commands.

Commands (run from the repository root):
  gate                                           exit 0 if no stop condition is recorded, else 22
  receipt AGENT RUN_DIR [TASK_FILE OUTPUT_FILE]  inspect one launch receipt (calibration mode; records nothing)
  record-smoke OUTPUT_FILE RUN_DIR               record the smoke launch (not counted)
  record-calibration TASK_ID OUTPUT_FILE RUN_DIR record one calibration launch (disclosed, not counted)
  record-attempt RUN_ID OUTPUT_FILE RUN_DIR      record an arm attempt with its receipt
  adjudicate-leakage RECORD coincidental|exposure REASON
                                                 RECORD is a record file under runs/, e.g. attempts/T1.M1.r1/1/attempt.json
  fallback-tasks RUN_ID                          write the fixed M3 fallback tasks for the final attempt
  record-fallback RUN_ID INDEX OUTPUT_FILE RUN_DIR
  compare RUN_ID                                 comparator pre (and post for M3) on the final valid typed attempt
  score-tasks [--new-batch]                      write a versioned batch of scorer tasks
  record-score SCORE_ID OUTPUT_FILE RUN_DIR      record and validate a scorer reply
  human-sample                                   write the human-check list for the latest batch
  declare-stop REASON                            record a stop (for example a pipeline defect)
  report                                         join every record into results

Exit codes: 0 ok; 2 usage; 21 refused or error (nothing written); 22 a stop condition holds.
Every record is written once and never overwritten.
"""

import hashlib
import json
import os
import random
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import comparator  # noqa: E402

RUNS = os.path.join(HERE, "runs")
TASKS = os.path.join(HERE, "tasks")
SEED = 20261124
SMOKE_TASK = "Reply with the single word READY."
NO_SPAN = "NO SUPPORTING SPAN"
STOP_EXIT = 22


class PipelineError(RuntimeError):
    pass


# ---------------------------------------------------------------- io

def read_bytes(path):
    with open(path, "rb") as f:
        return f.read()


def read_text(path):
    return read_bytes(path).decode()


def sha256(data):
    return hashlib.sha256(data if isinstance(data, bytes) else data.encode()).hexdigest()


def load_json(path):
    with open(path) as f:
        return json.load(f)


def dumps(obj):
    return json.dumps(obj, indent=2, ensure_ascii=False, sort_keys=True) + "\n"


def write_once(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "xb") as f:  # raises FileExistsError: nothing is overwritten
        f.write(data if isinstance(data, bytes) else data.encode())


def write_json(path, obj):
    if os.path.exists(path):
        raise PipelineError(f"refusing to overwrite {path}")
    write_once(path, dumps(obj))


def plan():
    return load_json(os.path.join(RUNS, "plan.json"))


def run_row(run_id):
    for r in plan()["runs"]:
        if r["run_id"] == run_id:
            return r
    raise PipelineError(f"unknown run {run_id}")


def expectations():
    return load_json(os.path.join(HERE, "receipt_expectations.json"))


def task_text(row):
    raw = read_bytes(os.path.join(TASKS, row["task_id"] + ".md"))
    if sha256(raw) != row["task_sha256"]:
        raise PipelineError(f"task file {row['task_id']} does not match its planned digest")
    return raw.decode()


def qn(text):
    return comparator.qnorm(text).lower()


# ---------------------------------------------------------------- receipts

def _content_text(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "".join(p.get("text", "") for p in content if isinstance(p, dict) and p.get("type") == "text")
    return ""


def extract_receipt(run_dir, agent, task=None, output_text=None, calibration=False):
    """Read the launch receipt from a native async run directory, outside the model.

    Findings are sorted into violations (the launch contract was proven broken),
    provider_failure (the provider did not complete) and unestablished (the
    evidence needed to establish the contract is missing). Missing is never zero."""
    exp = expectations()
    ag = exp["agents"][agent]
    rc = {"agent": agent, "run_dir": os.path.abspath(run_dir), "violations": [], "provider_failure": [],
          "unestablished": [], "observations": {}}
    v, pf, un = rc["violations"], rc["provider_failure"], rc["unestablished"]
    try:
        status_raw = read_bytes(os.path.join(run_dir, "status.json"))
        events_raw = read_bytes(os.path.join(run_dir, "events.jsonl"))
        status = json.loads(status_raw)
    except (OSError, ValueError) as exc:
        un.append(f"run directory unreadable: {type(exc).__name__}")
        return rc
    rc["status_sha256"], rc["events_sha256"] = sha256(status_raw), sha256(events_raw)
    steps = status.get("steps") or []
    step = steps[0] if steps else {}
    rc.update(run_id=status.get("runId"), session_id=status.get("sessionId"), session_file=step.get("sessionFile"),
              state=status.get("state"), exit_code=step.get("exitCode"), step_model=step.get("model"),
              step_thinking=step.get("thinking"), launch_contract_digest=status.get("launchContractDigest"),
              tokens=step.get("tokens"), effective_extensions=(step.get("launchResolvedExtensions") or {}).get("effective"))
    systems, users, assistants, tool_execs, malformed = [], [], [], 0, 0
    for line in events_raw.decode(errors="replace").splitlines():
        if not line.strip():
            continue
        try:
            e = json.loads(line)
        except ValueError:
            malformed += 1
            continue
        if e.get("type") == "tool_execution_start":
            tool_execs += 1
        m = e.get("message") or {}
        if e.get("type") == "message_end":
            {"system": systems, "user": users, "assistant": assistants}.get(m.get("role"), []).append(m)
    # identity and launch facts
    if not rc["run_id"]:
        un.append("no native run id")
    if len(steps) != 1:
        (v if len(steps) > 1 else un).append(f"{len(steps)} steps")
    if step.get("agent") != agent:
        v.append(f"agent {step.get('agent')!r} != {agent!r}")
    if step.get("model") != ag["step_model"] or step.get("thinking") != ag["thinking"]:
        v.append(f"model/thinking {step.get('model')}/{step.get('thinking')}")
    for key, want in (("context", "fresh"), ("skills", [])):
        if key not in step:
            un.append(f"step {key} not recorded")
        elif step[key] != want:
            v.append(f"step {key} {step[key]!r}")
    if malformed:
        un.append(f"{malformed} malformed event lines")
    # availability: tool declarations at every system message
    if not systems:
        un.append("no system message recorded")
    for i, s in enumerate(systems):
        if "toolsAdded" not in s:
            un.append(f"system message {i} has no toolsAdded record")
        elif s["toolsAdded"]:
            v.append(f"system message {i} declares tools")
        if s.get("toolsRemoved"):
            rc["observations"].setdefault("toolsRemoved", []).append(len(s["toolsRemoved"]))
        sections = s.get("sections")
        if not isinstance(sections, dict) or not sections:
            un.append(f"system message {i} has no sections")
        elif set(sections) != set(ag["system_sections"]):
            v.append(f"system message {i} sections {sorted(sections)}")
    if systems:
        content_sha = sha256(json.dumps([s.get("content") for s in systems], ensure_ascii=False))
        rc["system_content_sha256"] = content_sha
        pinned = ag.get("system_content_sha256")
        if pinned is None:
            if calibration:
                rc["observations"]["system_content_unpinned"] = True
            else:
                un.append("expected system content not pinned")
        elif content_sha != pinned:
            v.append("system content differs from the pinned expectation")
    if tool_execs:
        v.append(f"{tool_execs} tool executions")
    tc = step.get("toolCount", status.get("toolCount"))
    if isinstance(tc, int) and tc:
        v.append(f"toolCount {tc}")
    # task custody: one user turn carrying the planned task
    if len(users) != 1:
        (v if len(users) > 1 else un).append(f"{len(users)} user messages")
    if users and task is not None:
        utext = _content_text(users[0].get("content"))
        if not task or task not in utext:
            v.append("user message does not carry the planned task text")
        else:
            wrapper = sha256(utext.replace(task, "{{TASK}}", 1))
            rc["user_wrapper_sha256"] = wrapper
            pinned = exp.get("user_wrapper_sha256")
            if pinned is None and not calibration:
                un.append("expected user wrapper not pinned")
            elif pinned is not None and wrapper != pinned:
                v.append("user message wrapper differs from the pinned expectation")
    # actual provider, model and effort on every assistant call
    for i, m in enumerate(assistants):
        if (m.get("provider"), m.get("model"), m.get("thinkingLevel")) != (ag["provider"], ag["assistant_model"], ag["thinking"]):
            v.append(f"assistant call {i} ran {m.get('provider')}/{m.get('model')}/{m.get('thinkingLevel')}")
    rc["usage"] = [{"usage": m.get("usage"), "durationMs": m.get("durationMs")} for m in assistants]
    # terminal result
    final = assistants[-1] if assistants else {}
    if status.get("state") != "complete" or step.get("exitCode") not in (0,):
        pf.append(f"state {status.get('state')!r}, exit {step.get('exitCode')!r}")
    if not assistants:
        pf.append("no assistant message")
    elif final.get("stopReason") in ("error", "aborted") or final.get("errorMessage"):
        pf.append(f"final stop {final.get('stopReason')!r}")
    final_text = _content_text(final.get("content"))
    rc["final_text_sha256"] = sha256(final_text.strip())
    if output_text is not None and assistants:
        if output_text.strip() != final_text.strip():
            v.append("output file does not match the final assistant text")
        elif not final_text.strip():
            pf.append("empty final text")
    return rc


def receipt_status(rc):
    if rc["violations"]:
        return "invalid_receipt"
    if rc["provider_failure"]:
        return "provider_failure"
    if rc["unestablished"]:
        return "receipt_unestablished"
    return None


def _all_records():
    """Every recorded launch, across stages: (stage, path, record)."""
    out = []
    for root, _, files in os.walk(RUNS):
        for name in files:
            if not name.endswith(".json") or name.endswith(".adjudication.json"):
                continue
            path = os.path.join(root, name)
            rel = os.path.relpath(path, RUNS)
            stage = rel.split(os.sep)[0]
            if stage not in ("attempts", "fallbacks", "scores", "smoke", "calibration"):
                continue
            rec = load_json(path)
            if isinstance(rec, dict) and "receipt" in rec:
                out.append((stage, rel, rec))
    return sorted(out, key=lambda x: x[1])


def _require_unique_run(rc):
    if rc.get("run_id") and any(r["receipt"].get("run_id") == rc["run_id"] for _, _, r in _all_records()):
        raise PipelineError(f"native run {rc['run_id']} is already bound to a record")


# ---------------------------------------------------------------- statuses, adjudication, stops

def _adjudication_path(record_path):
    return record_path[:-5] + ".adjudication.json"


def effective(record_path, rec=None):
    rec = rec or load_json(record_path)
    if rec["status"] != "suspected_leakage":
        return rec["status"]
    adj = _adjudication_path(record_path)
    if not os.path.exists(adj):
        return "suspected_leakage"
    decision = load_json(adj)["decision"]
    return rec["assessed_status"] if decision == "coincidental" else "void_leakage"


def adjudicate_leakage(record_rel, decision, reason):
    if decision not in ("coincidental", "exposure") or not reason.strip():
        raise PipelineError("decision must be coincidental or exposure, with a reason")
    path = os.path.join(RUNS, record_rel)
    rec = load_json(path)
    if rec.get("status") != "suspected_leakage":
        raise PipelineError("record is not suspected_leakage")
    write_json(_adjudication_path(path), {"record": record_rel, "decision": decision, "reason": reason,
                                         "record_sha256": sha256(read_bytes(path))})
    return {"record": record_rel, "effective": effective(path)}


def _attempt_paths(base):
    if not os.path.isdir(base):
        return []
    return [os.path.join(base, n, "attempt.json") for n in sorted((d for d in os.listdir(base) if d.isdigit()), key=int)]


def _series(paths):
    return [(p, load_json(p)) for p in paths if os.path.exists(p)]


STOP_RECEIPTS = ("invalid_receipt", "receipt_unestablished")


def stop_reasons():
    reasons = []
    stop_file = os.path.join(RUNS, "STOP.json")
    if os.path.exists(stop_file):
        reasons.append(f"declared: {load_json(stop_file)['reason']}")
    for stage, rel, rec in _all_records():
        if stage in ("smoke", "calibration"):
            continue
        st = effective(os.path.join(RUNS, rel), rec)
        if st in STOP_RECEIPTS:
            reasons.append(f"{rel}: {st}")
        if rec.get("replacement") and st in ("provider_failure", "void_leakage") and stage == "attempts":
            reasons.append(f"{rel}: {st} on the replacement attempt")
    return reasons


def require_no_stop():
    reasons = stop_reasons()
    if reasons:
        raise PipelineError("stopped: " + "; ".join(reasons[:5]))


def declare_stop(reason):
    write_json(os.path.join(RUNS, "STOP.json"), {"reason": reason})
    return {"stopped": reason}


def _next_in_series(paths, what):
    """One authorized replacement: a second record only after a provider failure or a voided first record."""
    series = _series(paths)
    if not series:
        return 1, False
    if len(series) >= 2:
        raise PipelineError(f"{what}: no further attempts are authorized")
    st = effective(*series[0])
    if st not in ("provider_failure", "void_leakage", "invalid_reply"):
        raise PipelineError(f"{what}: attempt 1 is {st}; a replacement is not authorized")
    return 2, True


# ---------------------------------------------------------------- output assessment

def leakage_hits(text):
    terms = [t.strip() for t in read_text(os.path.join(HERE, "leakage_terms.txt")).splitlines()
             if t.strip() and not t.startswith("#")]
    low = text.lower()
    return [t for t in terms if t.lower() in low]


def parse_typed(text):
    """Whole output must be one JSON object, optionally inside a single fenced block. No repair."""
    s = text.strip()
    m = re.fullmatch(r"```(?:json)?\s*\n(.*)\n```", s, flags=re.S)
    if m:
        s = m.group(1)
    return json.loads(s)


_FLAGS = re.compile(r"(?m)^[ \t>#*_-]*FLAGS\b[*_ \t]*:?[ \t]*(.*)$")


def prose_complete(text, method, kind, derived_ids):
    """M1: a real FLAGS section; for review tasks it names every derived document.
    A1: a numbered list of obligations. Returns an error string or None."""
    if method == "A1":
        return None if re.search(r"(?m)^\s*\d+[.)]\s+\S", text) else "no numbered list of obligations"
    heads = list(_FLAGS.finditer(text))
    if not heads:
        return "no FLAGS section"
    head = heads[-1]
    body = (head.group(1) + "\n" + text[head.end():]).strip()
    if not body:
        return "empty FLAGS section"
    if kind == "review":
        missing = [d for d in derived_ids if d not in body]
        if missing:
            return f"FLAGS section does not cover {missing}"
    return None


def task_documents(row):
    tm = plan()["task_documents"][row["task_id"]]
    return tm["documents"], set(tm["parents"]), set(tm["derived"]), tm["kind"], tm


def assess_output(row, text):
    """Validity, independent of receipt and leakage. Returns (status, error, quote_failures)."""
    if not text.strip():
        return "incomplete", "empty output", []
    docs, parents, derived, kind, tm = task_documents(row)
    if row["method"] in ("M2", "M3"):
        try:
            extraction = parse_typed(text)
        except ValueError as exc:
            return "invalid_extraction", f"JSON: {exc}", []
        try:
            ids, failures = comparator.validate(extraction, docs, parents, derived)
        except comparator.ExtractionError as exc:
            return "invalid_extraction", str(exc), []
        if failures and comparator.QUOTE_POLICY == "strict":
            return "invalid_extraction", f"{len(failures)} quote failure(s)", failures
        return "valid", None, failures
    err = prose_complete(text, row["method"], kind, [d for d in tm["order"] if d in derived])
    return ("incomplete", err, []) if err else ("valid", None, [])


def _status(rc, assessed, hits):
    return receipt_status(rc) or ("suspected_leakage" if hits else assessed)


# ---------------------------------------------------------------- smoke and calibration (not counted)

def record_smoke(output_file, run_dir):
    text = read_text(output_file)
    rc = extract_receipt(run_dir, "p7-arm", SMOKE_TASK, text, calibration=True)
    _require_unique_run(rc)
    ok = receipt_status(rc) is None and text.strip() == "READY"
    rec = {"stage": "smoke", "receipt": rc, "output_sha256": sha256(text), "status": "ok" if ok else "failed"}
    n = len(os.listdir(os.path.join(RUNS, "smoke"))) + 1 if os.path.isdir(os.path.join(RUNS, "smoke")) else 1
    write_json(os.path.join(RUNS, "smoke", f"{n}.json"), rec)
    return rec


def record_calibration(task_id, output_file, run_dir):
    rows = [r for r in plan()["runs"] if r["task_id"] == task_id]
    if not rows:
        raise PipelineError(f"unknown task {task_id}")
    row = dict(rows[0], method="M2" if task_id.endswith(".M2") else rows[0]["method"])
    raw = read_bytes(output_file)
    text = raw.decode(errors="replace")
    rc = extract_receipt(run_dir, "p7-arm", task_text(row), text, calibration=True)
    _require_unique_run(rc)
    assessed, error, failures = assess_output(row, text)
    base = os.path.join(RUNS, "calibration", task_id)
    n = len([d for d in os.listdir(base) if d.isdigit()]) + 1 if os.path.isdir(base) else 1
    rec = {"stage": "calibration", "task_id": task_id, "receipt": rc, "receipt_status": receipt_status(rc),
           "output_sha256": sha256(raw), "assessed_status": assessed, "error": error,
           "quote_failures": failures, "leakage_hits": leakage_hits(text), "output_chars": len(text),
           "status": "calibration"}
    write_once(os.path.join(base, str(n), "output.txt"), raw)
    write_json(os.path.join(base, str(n), "record.json"), rec)
    return rec


# ---------------------------------------------------------------- arm attempts

def record_attempt(run_id, output_file, run_dir):
    require_no_stop()
    row = run_row(run_id)
    base = os.path.join(RUNS, "attempts", run_id)
    n, replacement = _next_in_series(_attempt_paths(base), run_id)
    raw = read_bytes(output_file)
    text = raw.decode(errors="replace")
    rc = extract_receipt(run_dir, "p7-arm", task_text(row), text)
    _require_unique_run(rc)
    assessed, error, failures = assess_output(row, text)
    hits = leakage_hits(text)
    rec = {"run_id": run_id, "attempt": n, "replacement": replacement, "output_sha256": sha256(raw),
           "task_sha256": row["task_sha256"], "receipt": rc, "leakage_hits": hits,
           "assessed_status": assessed, "error": error, "quote_failures": failures,
           "status": _status(rc, assessed, hits)}
    d = os.path.join(base, str(n))
    write_once(os.path.join(d, "output.txt"), raw)
    write_json(os.path.join(d, "attempt.json"), rec)
    return rec


def final_attempt(run_id):
    """The run's only eligible final attempt: the last one recorded (a second exists only as an
    authorized replacement). Returns the record with its effective status, or None."""
    series = _series(_attempt_paths(os.path.join(RUNS, "attempts", run_id)))
    if not series:
        return None
    path, rec = series[-1]
    return dict(rec, effective=effective(path, rec), path=os.path.relpath(path, RUNS))


def _valid_final(run_id):
    a = final_attempt(run_id)
    if not a or a["effective"] != "valid":
        raise PipelineError(f"{run_id}: no valid final attempt")
    return a


def _attempt_text(run_id, attempt):
    return read_bytes(os.path.join(RUNS, "attempts", run_id, str(attempt), "output.txt")).decode(errors="replace")


# ---------------------------------------------------------------- fallback (M3)

def _fallback_dir(run_id, attempt):
    return os.path.join(RUNS, "fallbacks", run_id, str(attempt))


def fallback_tasks(run_id):
    require_no_stop()
    row = run_row(run_id)
    if row["method"] != "M3":
        raise PipelineError("fallback only for M3")
    a = _valid_final(run_id)
    d = _fallback_dir(run_id, a["attempt"])
    if os.path.exists(os.path.join(d, "targets.json")):
        raise PipelineError("fallback targets already fixed for this attempt")
    extraction = parse_typed(_attempt_text(run_id, a["attempt"]))
    targets = comparator.fallback_targets(extraction)
    by_id = {e["id"]: e for e in extraction["entries"]}
    template = read_text(os.path.join(HERE, "prompts", "M3-fallback.md"))
    out = []
    for i, t in enumerate(targets):
        entry = by_id[t["entry"]]
        cands = entry.get("support_candidates", {}).get(t["property"], []) if entry["role"] == "derived" else []
        spans = "\n".join(f"- {by_id[c]['span']}" for c in cands) or "(none)"
        body = (template.replace("{{ENTRY_ID}}", t["entry"]).replace("{{PROPERTY}}", t["property"])
                .replace("{{MEANING}}", comparator.PROPERTIES[t["property"]]["meaning"])
                .replace("{{READINGS}}", json.dumps(t["readings"])).replace("{{SPAN}}", t["span"])
                .replace("{{PARENT_SPANS}}", spans))
        out.append({"index": i, "target": t, "task_sha256": sha256(body), "body": body})
    for o in out:
        write_once(os.path.join(d, f"task-{o['index']}.md"), o["body"])
    write_json(os.path.join(d, "targets.json"), {"run_id": run_id, "attempt": a["attempt"],
                                                 "attempt_output_sha256": a["output_sha256"],
                                                 "targets": [{k: v for k, v in o.items() if k != "body"} for o in out]})
    return out


def _response_paths(d, index):
    return [os.path.join(d, f"response-{index}.{n}.json") for n in (1, 2)]


def record_fallback(run_id, index, output_file, run_dir):
    require_no_stop()
    a = _valid_final(run_id)
    d = _fallback_dir(run_id, a["attempt"])
    meta = load_json(os.path.join(d, "targets.json"))
    if meta["attempt_output_sha256"] != a["output_sha256"]:
        raise PipelineError("fallback targets are bound to a different attempt")
    targets = meta["targets"]
    if not 0 <= index < len(targets):
        raise PipelineError("bad target index")
    n, replacement = _next_in_series(_response_paths(d, index), f"{run_id} fallback {index}")
    task = read_text(os.path.join(d, f"task-{index}.md"))
    if sha256(task) != targets[index]["task_sha256"]:
        raise PipelineError("fallback task file changed")
    raw = read_bytes(output_file)
    text = raw.decode(errors="replace")
    rc = extract_receipt(run_dir, "p7-arm", task, text)
    _require_unique_run(rc)
    rec = {"run_id": run_id, "attempt": a["attempt"], "index": index, "try": n, "replacement": replacement,
           "target": targets[index]["target"], "output_sha256": sha256(raw), "receipt": rc,
           "leakage_hits": leakage_hits(text), "error": None}
    try:
        rec["response"] = parse_typed(text)
        err = comparator.check_fallback_response(targets[index]["target"], rec["response"])
        assessed = "invalid_response" if err else "valid"
        rec["error"] = err
    except ValueError as exc:
        assessed, rec["error"] = "invalid_response", f"JSON: {exc}"
    rec["assessed_status"] = assessed
    rec["status"] = _status(rc, assessed, rec["leakage_hits"])
    write_json(_response_paths(d, index)[n - 1], rec)
    return rec


# ---------------------------------------------------------------- comparator

def comparator_digest():
    return sha256(read_bytes(os.path.join(HERE, "comparator.py")) + read_bytes(comparator.SCHEMA_PATH))[:16]


def comparator_path(run_id, attempt):
    return os.path.join(RUNS, "attempts", run_id, str(attempt), f"comparator-{comparator_digest()}.json")


def _fallback_outcomes(run_id, a):
    d = _fallback_dir(run_id, a["attempt"])
    meta_path = os.path.join(d, "targets.json")
    if not os.path.exists(meta_path):
        raise PipelineError("M3 needs fixed fallback targets first")
    meta = load_json(meta_path)
    targets, outcomes = [t["target"] for t in meta["targets"]], []
    for i in range(len(targets)):
        series = _series(_response_paths(d, i))
        if not series:
            raise PipelineError(f"incomplete fallback batch: target {i} has no recorded reply")
        path, rec = series[-1]
        st = effective(path, rec)
        if st == "suspected_leakage":
            raise PipelineError(f"fallback reply {i} awaits leakage adjudication")
        outcomes.append({"status": st, "response": rec.get("response"), "error": rec.get("error") or st,
                         "record_sha256": sha256(read_bytes(path))})
    return targets, outcomes


def compare(run_id):
    require_no_stop()
    row = run_row(run_id)
    if row["method"] not in ("M2", "M3"):
        raise PipelineError("compare is for typed runs")
    a = _valid_final(run_id)
    extraction = parse_typed(_attempt_text(run_id, a["attempt"]))
    docs, parents, derived, kind, _ = task_documents(row)
    fb = _fallback_outcomes(run_id, a) if row["method"] == "M3" else None
    rep = comparator.report(extraction, docs, parents, derived, kind, fb)
    rep["bound_to"] = {"run_id": run_id, "attempt": a["attempt"], "output_sha256": a["output_sha256"],
                       "comparator_digest": comparator_digest()}
    path = comparator_path(run_id, a["attempt"])
    data = dumps(rep)
    if os.path.exists(path):
        if read_text(path) != data:
            raise PipelineError(f"comparator defect: recomputation differs from {path}")
        return rep
    write_once(path, data)
    return rep


# ---------------------------------------------------------------- scoring

PHASES = {"A1": ["prose"], "M1": ["prose"], "M2": ["pre"], "M3": ["pre", "post"]}
ANSWERS = {"expect": {"hit", "miss", "not_assessed", "uncertain"}, "positive": {"hit", "miss", "not_assessed", "uncertain"},
           "observe": {"yes", "no", "uncertain"}, "must_not": {"violated", "not_violated", "uncertain"}}
ABSENCE = {"miss", "not_violated", "no", "uncertain"}


def _translate(text, mapping):
    for orig, neutral in sorted(mapping.items(), key=lambda kv: -len(kv[0])):
        text = text.replace(orig, neutral)
    return text


def rubric_items(input_set):
    spec = load_json(os.path.join(HERE, "prompts", "rubric.json"))["inputs"][input_set]
    items = {i["id"]: i["kind"] for i in spec.get("items", [])}
    items.update({i["id"]: "must_not" for i in spec.get("must_not", [])})
    return spec, items


def phase_material(row, a):
    """Per-phase text a scorer may quote from."""
    text = _attempt_text(row["run_id"], a["attempt"])
    if row["method"] in ("A1", "M1"):
        return {"prose": text.strip()}
    rep = load_json(comparator_path(row["run_id"], a["attempt"]))
    ext = "EXTRACTION:\n" + text.strip()
    out = {"pre": ext + "\n\nCOMPARATOR REPORT (pre):\n" + json.dumps(rep["pre"], indent=1, ensure_ascii=False)}
    if row["method"] == "M3":
        out["post"] = (ext + "\n\nCOMPARATOR REPORT (post):\n" + json.dumps(rep["post"], indent=1, ensure_ascii=False)
                       + "\n\nFALLBACK LOG:\n" + json.dumps(rep["fallback_log"], indent=1, ensure_ascii=False))
    return out


def _batches():
    d = os.path.join(RUNS, "scores")
    if not os.path.isdir(d):
        return []
    return sorted((int(x.split("-")[1]) for x in os.listdir(d) if re.fullmatch(r"batch-\d+", x)))


def _batch_dir(k):
    return os.path.join(RUNS, "scores", f"batch-{k}")


def _binding(entry):
    return (entry["run_id"], entry["attempt"], entry["output_sha256"], entry.get("comparator_sha256"))


def score_tasks(new_batch=False):
    require_no_stop()
    p = plan()
    template = read_text(os.path.join(HERE, "prompts", "scorer.md"))
    rows = []
    for r in p["runs"]:
        a = final_attempt(r["run_id"])
        if not a or a["effective"] != "valid":
            continue
        if r["method"] in ("M2", "M3") and not os.path.exists(comparator_path(r["run_id"], a["attempt"])):
            raise PipelineError(f"{r['run_id']}: run compare before scoring")
        rows.append((r, a))
    rng = random.Random(SEED)
    rng.shuffle(rows)
    candidates = []
    for r, a in rows:
        cpath = comparator_path(r["run_id"], a["attempt"])
        candidates.append({"run_id": r["run_id"], "attempt": a["attempt"], "output_sha256": a["output_sha256"],
                           "comparator_sha256": sha256(read_bytes(cpath)) if os.path.exists(cpath) else None})
    existing = _batches()
    if existing:
        key = load_json(os.path.join(_batch_dir(existing[-1]), "key.json"))
        if sorted(map(_binding, key["key"])) == sorted(map(_binding, candidates)):
            return key
        if not new_batch:
            raise PipelineError("eligible attempts or comparator outputs changed since the latest batch; pass --new-batch")
    k = (existing[-1] + 1) if existing else 1
    bdir = _batch_dir(k)
    entries, files = [], {}
    for i, ((r, a), cand) in enumerate(zip(rows, candidates)):
        tm = p["task_documents"][r["task_id"]]
        spec, items = rubric_items(r["input_set"])
        phases = PHASES[r["method"]]
        material = phase_material(r, a)
        docs = "\n\n".join(f"=== DOCUMENT {nid} ===\n{tm['documents'][nid]}\n=== END {nid} ===" for nid in tm["order"])
        output = "\n\n".join(f"=== PHASE {ph} MATERIAL ===\n{material[ph]}\n=== END PHASE {ph} ===" for ph in phases)
        score_id = f"b{k}-S{i + 1:03d}"
        body = (template.replace("{{INPUT_ID}}", score_id).replace("{{PHASES}}", json.dumps(phases))
                .replace("{{RUBRIC}}", _translate(json.dumps(spec, indent=1), tm["id_map"]))
                .replace("{{DOCUMENTS}}", docs).replace("{{OUTPUT}}", output))
        files[f"{score_id}/task.md"] = body
        files[f"{score_id}/material.json"] = dumps(material)
        entries.append(dict(cand, score_id=score_id, input_set=r["input_set"], method=r["method"], phases=phases,
                            items=items, task_sha256=sha256(body), material_sha256=sha256(files[f"{score_id}/material.json"])))
    os.makedirs(bdir)  # fresh directory; raises if it exists
    for rel, data in files.items():
        write_once(os.path.join(bdir, rel), data)
    key = {"batch": k, "seed": SEED, "comparator_digest": comparator_digest(), "key": entries}
    write_json(os.path.join(bdir, "key.json"), key)
    return key


def _key_entry(score_id):
    m = re.fullmatch(r"b(\d+)-S\d{3}", score_id)
    if not m:
        raise PipelineError(f"bad score id {score_id}")
    k = int(m.group(1))
    key = load_json(os.path.join(_batch_dir(k), "key.json"))
    for e in key["key"]:
        if e["score_id"] == score_id:
            return k, e
    raise PipelineError(f"unknown score id {score_id}")


def validate_score_reply(reply, entry, material, documents):
    """Returns (errors, quote_failures). Errors make the reply invalid; quote failures send the
    affected answers to human review."""
    errors, qfail = [], []
    if not isinstance(reply, dict) or reply.get("input") != entry["score_id"]:
        return ["reply missing or for another input"], []
    phases = reply.get("phases")
    if not isinstance(phases, list) or sorted(ph.get("phase") for ph in phases if isinstance(ph, dict)) != sorted(entry["phases"]) \
            or len(phases) != len(entry["phases"]):
        return [f"phases must be exactly {entry['phases']}"], []
    sources = qn("\n".join(documents.values()))
    targets = {i for i, k in entry["items"].items() if k in ("expect", "positive")}
    for ph in phases:
        name, mat = ph["phase"], qn(material[ph["phase"]])

        def quoted(q):
            return isinstance(q, str) and q.strip() and qn(q) in mat
        items = ph.get("items")
        if not isinstance(items, list) or sorted(str(i.get("id")) for i in items if isinstance(i, dict)) != sorted(entry["items"]) \
                or len(items) != len(entry["items"]):
            errors.append(f"{name}: items must be exactly {sorted(entry['items'])}")
            continue
        for it in items:
            kind = entry["items"][it["id"]]
            ans, ev = it.get("answer"), it.get("evidence")
            if ans not in ANSWERS[kind]:
                errors.append(f"{name}.{it['id']}: illegal answer {ans!r}")
            elif not isinstance(ev, str):
                errors.append(f"{name}.{it['id']}: evidence must be a string")
            elif ans == "not_assessed" or (ans in ABSENCE and ev.strip() == NO_SPAN):
                pass
            elif not quoted(ev):
                qfail.append({"phase": name, "id": it["id"]})
        census = ph.get("flag_census")
        if not isinstance(census, list):
            errors.append(f"{name}: flag_census must be a list")
            census = []
        for fl in census:
            cls = fl.get("class") if isinstance(fl, dict) else None
            legal = cls in ("legitimate_other", "false_flag", "context_only") or (
                isinstance(cls, str) and cls.startswith("target:") and cls[7:] in targets)
            if not legal:
                errors.append(f"{name}: bad census class {cls!r}")
                continue
            if not quoted(fl.get("flag")):
                qfail.append({"phase": name, "census": str(fl.get("flag"))[:80]})
            if cls == "legitimate_other" and not (isinstance(fl.get("source_support"), str)
                                                  and fl["source_support"].strip() and qn(fl["source_support"]) in sources):
                qfail.append({"phase": name, "census_support": str(fl.get("flag"))[:80]})
        if name in ("pre", "post"):
            typed = material[name].split(f"COMPARATOR REPORT ({name}):\n", 1)[1].split("\n\nFALLBACK LOG:", 1)[0]
            rep = json.loads(typed)
            n_typed = len(rep.get("derived_flags", [])) + len(rep.get("plan_flags", []))
            if len(census) < n_typed:
                errors.append(f"{name}: census lists {len(census)} flags, the report has {n_typed}")
        w = ph.get("weakening")
        if not isinstance(w, dict) or w.get("answer") not in ("yes", "no") or not isinstance(w.get("evidence"), str):
            errors.append(f"{name}: bad weakening answer")
        elif w["answer"] == "yes" and not quoted(w["evidence"]):
            qfail.append({"phase": name, "weakening": True})
    return errors, qfail


def record_score(score_id, output_file, run_dir):
    require_no_stop()
    k, entry = _key_entry(score_id)
    d = os.path.join(_batch_dir(k), score_id)
    paths = [os.path.join(d, f"reply-{n}.json") for n in (1, 2)]
    n, replacement = _next_in_series(paths, score_id)
    task = read_text(os.path.join(d, "task.md"))
    if sha256(task) != entry["task_sha256"]:
        raise PipelineError("scorer task changed")
    material = load_json(os.path.join(d, "material.json"))
    raw = read_bytes(output_file)
    text = raw.decode(errors="replace")
    rc = extract_receipt(run_dir, "p7-scorer", task, text)
    _require_unique_run(rc)
    rec = {"score_id": score_id, "try": n, "replacement": replacement, "output_sha256": sha256(raw), "receipt": rc,
           "errors": [], "quote_failures": []}
    docs = plan()["task_documents"][run_row(entry["run_id"])["task_id"]]["documents"]
    try:
        rec["reply"] = parse_typed(text)
        rec["errors"], rec["quote_failures"] = validate_score_reply(rec["reply"], entry, material, docs)
    except (ValueError, KeyError, TypeError, AttributeError, IndexError) as exc:
        rec["errors"] = [f"{type(exc).__name__}: {exc}"]
    rec["assessed_status"] = "invalid_reply" if rec["errors"] else "valid"
    rec["status"] = receipt_status(rc) or rec["assessed_status"]
    write_json(paths[n - 1], rec)
    return rec


def _final_reply(k, score_id):
    d = os.path.join(_batch_dir(k), score_id)
    series = _series([os.path.join(d, f"reply-{n}.json") for n in (1, 2)])
    valid = [r for _, r in series if r["status"] == "valid"]
    return (valid[-1] if valid else None), series


def human_sample():
    k = _batches()[-1] if _batches() else None
    if k is None:
        raise PipelineError("no score batch")
    key_doc = load_json(os.path.join(_batch_dir(k), "key.json"))
    key = key_doc["key"]
    cells = {}
    for e in key:
        cells.setdefault((e["input_set"], e["method"]), []).append(e["score_id"])
    rng = random.Random(SEED + 1)
    sample = {sid: ["seeded cell sample"] for sid in (rng.choice(sorted(v)) for _, v in sorted(cells.items()))}
    for e in key:
        reply, series = _final_reply(k, e["score_id"])
        why = []
        if reply is None:
            why.append("no valid reply")
        else:
            if reply["quote_failures"]:
                why.append("quote failures")
            phases = {ph["phase"]: {i["id"]: i["answer"] for i in ph["items"]} for ph in reply["reply"]["phases"]}
            if any(a == "uncertain" for ans in phases.values() for a in ans.values()):
                why.append("uncertain answer")
            if "post" in phases and phases["pre"] != phases["post"]:
                why.append("M3 pre/post transition")
        if e["method"] == "M3":
            rep = load_json(os.path.join(RUNS, "attempts", e["run_id"], str(e["attempt"]),
                                         f"comparator-{key_doc['comparator_digest']}.json"))
            if any(r["outcome"] == "resolved_within_list" for r in rep["fallback_log"]):
                why.append("fallback resolution: check textual warrant")
        if why:
            sample.setdefault(e["score_id"], []).extend(why)
    out = {"batch": k, "seed": SEED + 1, "scores": {s: sample[s] for s in sorted(sample)}}
    write_json(os.path.join(_batch_dir(k), "human-sample.json"), out)
    return out


# ---------------------------------------------------------------- report

def _adjudications(k):
    path = os.path.join(RUNS, "adjudication.json")
    if not os.path.exists(path):
        return {}
    out = {}
    for a in load_json(path):
        if not a.get("score_id", "").startswith(f"b{k}-"):
            continue
        for field in ("score_id", "phase", "item_id", "operator_answer", "reason"):
            if not a.get(field):
                raise PipelineError(f"adjudication entry missing {field}: {a}")
        out[(a["score_id"], a["phase"], a["item_id"])] = a
    return out


def _usage(acc, stage, rc):
    s = acc.setdefault(stage, {"calls": 0, "assistant_messages": 0, "tokens": {}, "durationMs": 0, "unknown_usage": 0})
    s["calls"] += 1
    for u in rc.get("usage") or []:
        s["assistant_messages"] += 1
        if not isinstance(u.get("usage"), dict):
            s["unknown_usage"] += 1
        for key, val in (u.get("usage") or {}).items():
            if isinstance(val, (int, float)):
                s["tokens"][key] = s["tokens"].get(key, 0) + val
        if isinstance(u.get("durationMs"), (int, float)):
            s["durationMs"] += u["durationMs"]
    if not rc.get("usage"):
        s["unknown_usage"] += 1


def report():
    p = plan()
    out = {"runs": len(p["runs"]), "ledger": {}, "cells": {}, "usage_by_stage": {}, "items": {}, "census": {},
           "weakening": {}, "m3": {}, "stops": stop_reasons()}
    for stage, rel, rec in _all_records():
        _usage(out["usage_by_stage"], stage + ("-replacement" if rec.get("replacement") else ""), rec["receipt"])
    finals = {}
    for r in p["runs"]:
        series = _series(_attempt_paths(os.path.join(RUNS, "attempts", r["run_id"])))
        out["ledger"][r["run_id"]] = [{"attempt": rec["attempt"], "status": rec["status"], "effective": effective(path, rec),
                                       "assessed_status": rec.get("assessed_status"), "error": rec.get("error")}
                                      for path, rec in series]
        fa = final_attempt(r["run_id"])
        finals[r["run_id"]] = fa
        cell = out["cells"].setdefault(f"{r['input_set']}.{r['method']}", {"planned": 0, "attempted": 0, "valid": 0, "final_status": {}})
        cell["planned"] += 1
        st = fa["effective"] if fa else "not_run"
        cell["attempted"] += 1 if fa else 0
        cell["valid"] += 1 if st == "valid" else 0
        cell["final_status"][st] = cell["final_status"].get(st, 0) + 1
    batches = _batches()
    if batches:
        k = batches[-1]
        key = load_json(os.path.join(_batch_dir(k), "key.json"))
        out["score_batch"] = k
        adj = _adjudications(k)
        by_run = {e["run_id"]: e for e in key["key"]}
        for r in p["runs"]:
            cell_id = f"{r['input_set']}.{r['method']}"
            valid_runs = out["cells"][cell_id]["valid"]
            attempted = out["cells"][cell_id]["attempted"]
            e = by_run.get(r["run_id"])
            if not e:
                continue
            reply, _ = _final_reply(k, e["score_id"])
            answers = {}
            for ph in e["phases"]:
                rph = next((x for x in reply["reply"]["phases"] if x["phase"] == ph), None) if reply else None
                failed = {q["id"] for q in (reply or {}).get("quote_failures", []) if q.get("phase") == ph and "id" in q}
                for item_id in e["items"]:
                    a = adj.get((e["score_id"], ph, item_id))
                    if a:
                        ans = a["operator_answer"]
                    elif rph is None or item_id in failed:
                        ans = "unresolved"
                    else:
                        ans = next(i["answer"] for i in rph["items"] if i["id"] == item_id)
                    answers[(ph, item_id)] = ans
                    slot = out["items"].setdefault(f"{cell_id}.{ph}.{item_id}", {"answers": {}, "valid_runs": valid_runs,
                                                                                  "attempted_runs": attempted})
                    slot["answers"][ans] = slot["answers"].get(ans, 0) + 1
                cen = out["census"].setdefault(f"{cell_id}.{ph}", {})
                wk = out["weakening"].setdefault(f"{cell_id}.{ph}", {"yes": 0, "no": 0, "unresolved": 0})
                if rph is None:
                    wk["unresolved"] += 1
                    continue
                for fl in rph["flag_census"]:
                    cls = "target" if fl["class"].startswith("target:") else fl["class"]
                    cen[cls] = cen.get(cls, 0) + 1
                wk[rph["weakening"]["answer"]] += 1
            if r["method"] == "M3":
                rep = load_json(os.path.join(RUNS, "attempts", r["run_id"], str(e["attempt"]), f"comparator-{key['comparator_digest']}.json"))
                trans = {i: [answers[("pre", i)], answers[("post", i)]] for i in e["items"]}
                pre_f = {json.dumps(f, sort_keys=True) for f in rep["pre"]["derived_flags"] + rep["pre"].get("plan_flags", [])}
                post_f = {json.dumps(f, sort_keys=True) for f in rep["post"]["derived_flags"] + rep["post"].get("plan_flags", [])}
                outcomes = {}
                for row in rep["fallback_log"]:
                    outcomes[row["outcome"]] = outcomes.get(row["outcome"], 0) + 1
                out["m3"][r["run_id"]] = {
                    "item_transitions": trans,
                    "hits_gained": sorted(i for i, (a, b) in trans.items() if a != "hit" and b == "hit"),
                    "hits_lost": sorted(i for i, (a, b) in trans.items() if a == "hit" and b != "hit"),
                    "flags_removed": len(pre_f - post_f), "flags_added": len(post_f - pre_f),
                    "fallback_outcomes": outcomes, "targets": len(rep["fallback_log"])}
        for slot in out["items"].values():
            v = slot["valid_runs"]
            slot["hit_rate_valid"] = (slot["answers"].get("hit", 0) / v) if v else None
            slot["hit_rate_attempted"] = (slot["answers"].get("hit", 0) / slot["attempted_runs"]) if slot["attempted_runs"] else None
    path = os.path.join(RUNS, "report.json")
    if os.path.exists(path):
        os.replace(path, os.path.join(RUNS, "report.prev.json"))
    with open(path, "w") as f:
        f.write(dumps(out))
    return out


# ---------------------------------------------------------------- cli

def main(argv):
    cmd = argv[1] if len(argv) > 1 else ""
    args = argv[2:]
    try:
        if cmd == "gate":
            reasons = stop_reasons()
            print(json.dumps({"stopped": bool(reasons), "reasons": reasons}, indent=2))
            return STOP_EXIT if reasons else 0
        if cmd == "receipt" and len(args) in (2, 4):
            task = read_text(args[2]) if len(args) == 4 else None
            out = read_text(args[3]) if len(args) == 4 else None
            res = extract_receipt(args[1], args[0], task, out, calibration=True)
            print(json.dumps(res, indent=2)[:6000])
            return 0 if receipt_status(res) is None else STOP_EXIT
        handlers = {
            "record-smoke": (2, lambda: record_smoke(*args)),
            "record-calibration": (3, lambda: record_calibration(*args)),
            "record-attempt": (3, lambda: record_attempt(*args)),
            "adjudicate-leakage": (3, lambda: adjudicate_leakage(*args)),
            "fallback-tasks": (1, lambda: [{k: v for k, v in o.items() if k != "body"} for o in fallback_tasks(args[0])]),
            "record-fallback": (4, lambda: record_fallback(args[0], int(args[1]), args[2], args[3])),
            "compare": (1, lambda: compare(args[0])),
            "score-tasks": (None, lambda: score_tasks("--new-batch" in args)),
            "record-score": (3, lambda: record_score(*args)),
            "human-sample": (0, human_sample),
            "declare-stop": (1, lambda: declare_stop(args[0])),
            "report": (0, report),
        }
        if cmd not in handlers or (handlers[cmd][0] is not None and len(args) != handlers[cmd][0]):
            sys.stderr.write(__doc__)
            return 2
        res = handlers[cmd][1]()
    except (PipelineError, comparator.ExtractionError, OSError, ValueError) as exc:
        print(json.dumps({"error": f"{type(exc).__name__}: {exc}"}))
        return 21
    print(json.dumps(res, indent=2, ensure_ascii=False)[:6000])
    if cmd == "record-smoke" and res.get("status") != "ok":
        return STOP_EXIT
    return STOP_EXIT if stop_reasons() else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
