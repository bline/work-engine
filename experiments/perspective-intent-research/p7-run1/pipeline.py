#!/usr/bin/env python3
"""P7 run 1 offline pipeline. No model calls; the operator runbook (RUNBOOK.md)
launches agents and feeds their outputs and run directories to these commands.

Commands (run from the repository root):
  receipt AGENT RUN_DIR                          check one launch receipt (used for the smoke test)
  record-attempt RUN_ID OUTPUT_FILE RUN_DIR      record an arm attempt with its launch receipt
  fallback-tasks RUN_ID                          write M3 fallback task texts for a recorded attempt
  record-fallback RUN_ID TARGET_INDEX OUTPUT_FILE RUN_DIR
  compare RUN_ID                                 comparator pre (and post for M3) on a valid typed attempt
  score-tasks                                    write blinded scorer tasks for every final attempt
  record-score SCORE_ID OUTPUT_FILE RUN_DIR      record a scorer reply with its receipt and quote checks
  human-sample                                   write the preselected human-check list
  report                                         aggregate results

State lives under runs/ (attempts, fallbacks, scores). Nothing is overwritten:
every attempt is kept, numbered, with its digests.
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
SEED = 20261124
EXPECTED = {
    "p7-arm": {"model": "claude-sdk/claude-opus-5-5", "thinking": "high", "provider": "claude-sdk"},
    "p7-scorer": {"model": "claude-sdk/claude-opus-5-5", "thinking": "high", "provider": "claude-sdk"},
}
ALLOWED_SYSTEM_SECTIONS = {"preamble", "cwd"}
BRIDGE_MARKERS = ("contact_supervisor", "intercom", "orchestrator")


class PipelineError(RuntimeError):
    pass


def read_text(path):
    with open(path) as f:
        return f.read()


def sha256(data):
    return hashlib.sha256(data if isinstance(data, bytes) else data.encode()).hexdigest()


def load_json(path):
    with open(path) as f:
        return json.load(f)


def write_json(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        raise PipelineError(f"refusing to overwrite {path}")
    with open(path, "w") as f:
        json.dump(obj, f, indent=2, ensure_ascii=False)
        f.write("\n")


def plan():
    return load_json(os.path.join(RUNS, "plan.json"))


def run_row(run_id):
    for r in plan()["runs"]:
        if r["run_id"] == run_id:
            return r
    raise PipelineError(f"unknown run {run_id}")


def norm(text):
    return re.sub(r"\s+", " ", text).strip().lower()


# ---------------------------------------------------------------- receipts

def extract_receipt(run_dir, agent):
    """Read the launch receipt from a pi-subagents async run directory, outside the model."""
    status = load_json(os.path.join(run_dir, "status.json"))
    step = (status.get("steps") or [{}])[0]
    system_sections, models, usage, tool_execs = None, set(), [], 0
    with open(os.path.join(run_dir, "events.jsonl")) as f:
        for line in f:
            try:
                e = json.loads(line)
            except ValueError:
                continue
            t = e.get("type")
            if t == "tool_execution_start":
                tool_execs += 1
            m = e.get("message") or {}
            if t == "message_end" and m.get("role") == "system" and system_sections is None:
                system_sections = m.get("sections") or {}
            if t == "message_end" and m.get("role") == "assistant":
                models.add((m.get("provider"), m.get("model"), m.get("thinkingLevel")))
                usage.append({"usage": m.get("usage"), "durationMs": m.get("durationMs")})
    exp = EXPECTED[agent]
    problems = []
    if step.get("agent") != agent:
        problems.append(f"agent {step.get('agent')!r} != {agent!r}")
    if step.get("toolCount", status.get("toolCount")) != 0:
        problems.append(f"toolCount {step.get('toolCount', status.get('toolCount'))!r} != 0")
    if tool_execs:
        problems.append(f"{tool_execs} tool executions")
    if step.get("model") != exp["model"] or step.get("thinking") != exp["thinking"]:
        problems.append(f"model/thinking {step.get('model')}/{step.get('thinking')} != {exp['model']}/{exp['thinking']}")
    if any(p != exp["provider"] for p, _, _ in models):
        problems.append(f"provider(s) {sorted({p for p, _, _ in models})}")
    if step.get("skills"):
        problems.append("skills injected")
    if step.get("context") not in (None, "fresh"):
        problems.append(f"context {step.get('context')!r}")
    if system_sections is None:
        problems.append("no system message recorded")
    else:
        extra = set(system_sections) - ALLOWED_SYSTEM_SECTIONS
        if extra:
            problems.append(f"unexpected system sections {sorted(extra)}")
        preamble = str(system_sections.get("preamble", "")).lower()
        if any(mark in preamble for mark in BRIDGE_MARKERS):
            problems.append("bridge instructions in system prompt")
    return {"agent": agent, "step_model": step.get("model"), "step_thinking": step.get("thinking"),
            "toolCount": step.get("toolCount", status.get("toolCount")), "tool_executions": tool_execs,
            "launchContractDigest": step.get("launchContractDigest") or status.get("launchContractDigest"),
            "effective_extensions": ((step.get("launchResolvedExtensions") or status.get("launchResolvedExtensions") or {}).get("effective")),
            "system_sections": sorted((system_sections or {}).keys()),
            "system_digest": sha256(json.dumps(system_sections, sort_keys=True)) if system_sections is not None else None,
            "assistant_models": sorted([list(m) for m in models]), "usage": usage,
            "exitCode": step.get("exitCode"), "tokens": step.get("tokens"), "problems": problems}


def leakage_hits(text):
    terms = [t.strip() for t in read_text(os.path.join(HERE, "leakage_terms.txt")).splitlines()
             if t.strip() and not t.startswith("#")]
    low = text.lower()
    return [t for t in terms if t.lower() in low]


# ---------------------------------------------------------------- arm attempts

def _attempt_dir(run_id):
    base = os.path.join(RUNS, "attempts", run_id)
    os.makedirs(base, exist_ok=True)
    n = len([d for d in os.listdir(base) if d.isdigit()]) + 1
    return os.path.join(base, str(n)), n


def parse_typed(text):
    """Whole output must be one JSON object, optionally inside a single fenced block. No repair."""
    s = text.strip()
    m = re.fullmatch(r"```(?:json)?\s*\n(.*)\n```", s, flags=re.S)
    if m:
        s = m.group(1)
    return json.loads(s)


def task_documents(row):
    p = plan()
    tm = p["task_documents"][row["task_id"]]
    return tm["documents"], set(tm["parents"]), set(tm["derived"]), tm["kind"]


def record_attempt(run_id, output_file, run_dir):
    row = run_row(run_id)
    with open(output_file, "rb") as f:
        raw = f.read()
    text = raw.decode(errors="replace")
    receipt = extract_receipt(run_dir, "p7-arm")
    d, n = _attempt_dir(run_id)
    os.makedirs(d)
    with open(os.path.join(d, "output.txt"), "wb") as f:
        f.write(raw)
    rec = {"run_id": run_id, "attempt": n, "output_sha256": sha256(raw), "task_sha256": row["task_sha256"],
           "receipt": receipt, "leakage_hits": leakage_hits(text)}
    if receipt["problems"]:
        rec["status"] = "invalid_receipt"
    elif receipt["exitCode"] not in (0, None) or not text.strip():
        rec["status"] = "provider_or_empty"
    elif rec["leakage_hits"]:
        rec["status"] = "suspected_leakage"
    elif row["method"] in ("M2", "M3"):
        try:
            extraction = parse_typed(text)
            docs, parents, derived, kind = task_documents(row)
            comparator.validate(extraction, docs, parents, derived)
            rec["status"] = "valid"
        except (ValueError, comparator.ExtractionError) as exc:
            rec["status"] = "invalid_extraction"
            rec["error"] = f"{type(exc).__name__}: {exc}"
    elif row["method"] == "M1" and "flags" not in text.lower():
        rec["status"] = "incomplete"
    else:
        rec["status"] = "valid"
    write_json(os.path.join(d, "attempt.json"), rec)
    return rec


def final_attempt(run_id):
    """The first attempt with status valid; else the last attempt. Leakage-suspected attempts
    are adjudicated before scores are unblinded (RUNBOOK)."""
    base = os.path.join(RUNS, "attempts", run_id)
    if not os.path.isdir(base):
        return None
    attempts = [load_json(os.path.join(base, n, "attempt.json")) for n in sorted(os.listdir(base), key=int)]
    for a in attempts:
        if a["status"] == "valid":
            return a
    return attempts[-1] if attempts else None


def _attempt_text(run_id, attempt):
    with open(os.path.join(RUNS, "attempts", run_id, str(attempt), "output.txt"), "rb") as f:
        return f.read().decode(errors="replace")


# ---------------------------------------------------------------- fallback (M3)

def fallback_tasks(run_id):
    row = run_row(run_id)
    if row["method"] != "M3":
        raise PipelineError("fallback only for M3")
    a = final_attempt(run_id)
    if not a or a["status"] != "valid":
        raise PipelineError("no valid attempt")
    extraction = parse_typed(_attempt_text(run_id, a["attempt"]))
    targets = comparator.fallback_targets(extraction)
    by_id = {e["id"]: e for e in extraction["entries"]}
    template = read_text(os.path.join(HERE, "prompts", "M3-fallback.md"))
    meanings = comparator.PROPERTIES
    out = []
    for i, t in enumerate(targets):
        entry = by_id[t["entry"]]
        cands = (entry.get("support_candidates") or {}).get(t["property"], []) if entry["role"] == "derived" else []
        spans = "\n".join(f"- {by_id[c]['span']}" for c in cands) or "(none)"
        body = (template.replace("{{ENTRY_ID}}", t["entry"]).replace("{{PROPERTY}}", t["property"])
                .replace("{{MEANING}}", meanings[t["property"]]["meaning"])
                .replace("{{READINGS}}", json.dumps(t["readings"])).replace("{{SPAN}}", t["span"])
                .replace("{{PARENT_SPANS}}", spans))
        out.append({"index": i, "target": t, "task_sha256": sha256(body), "body": body})
    d = os.path.join(RUNS, "fallbacks", run_id, str(a["attempt"]))
    write_json(os.path.join(d, "targets.json"), {"attempt": a["attempt"], "targets": [{k: v for k, v in o.items() if k != "body"} for o in out]})
    for o in out:
        with open(os.path.join(d, f"task-{o['index']}.md"), "w") as f:
            f.write(o["body"])
    return out


def record_fallback(run_id, index, output_file, run_dir):
    a = final_attempt(run_id)
    d = os.path.join(RUNS, "fallbacks", run_id, str(a["attempt"]))
    targets = load_json(os.path.join(d, "targets.json"))["targets"]
    if not 0 <= index < len(targets):
        raise PipelineError("bad target index")
    with open(output_file, "rb") as f:
        raw = f.read()
    receipt = extract_receipt(run_dir, "p7-arm")
    rec = {"index": index, "output_sha256": sha256(raw), "receipt": receipt,
           "leakage_hits": leakage_hits(raw.decode(errors="replace"))}
    try:
        rec["response"] = parse_typed(raw.decode(errors="replace"))
        rec["status"] = "invalid_receipt" if receipt["problems"] else "valid"
    except ValueError as exc:
        rec["status"], rec["error"] = "invalid_json", str(exc)
    write_json(os.path.join(d, f"response-{index}.json"), rec)
    return rec


# ---------------------------------------------------------------- comparator

def compare(run_id):
    row = run_row(run_id)
    a = final_attempt(run_id)
    if not a or a["status"] != "valid" or row["method"] not in ("M2", "M3"):
        raise PipelineError("compare needs a valid typed attempt")
    extraction = parse_typed(_attempt_text(run_id, a["attempt"]))
    docs, parents, derived, kind = task_documents(row)
    fb = None
    if row["method"] == "M3":
        d = os.path.join(RUNS, "fallbacks", run_id, str(a["attempt"]))
        targets = [t["target"] for t in load_json(os.path.join(d, "targets.json"))["targets"]]
        responses = []
        for i in range(len(targets)):
            p = os.path.join(d, f"response-{i}.json")
            if os.path.exists(p):
                r = load_json(p)
                if r["status"] == "valid":
                    responses.append(r["response"])
        fb = (targets, responses)
    rep = comparator.report(extraction, docs, parents, derived, kind, fb)
    write_json(os.path.join(RUNS, "attempts", run_id, str(a["attempt"]), "comparator.json"), rep)
    return rep


# ---------------------------------------------------------------- scoring

def _translate(text, mapping):
    for orig, neutral in sorted(mapping.items(), key=lambda kv: -len(kv[0])):
        text = text.replace(orig, neutral)
    return text


def score_tasks():
    p = plan()
    rubric = load_json(os.path.join(HERE, "prompts", "rubric.json"))["inputs"]
    template = read_text(os.path.join(HERE, "prompts", "scorer.md"))
    rows = [r for r in p["runs"] if (final_attempt(r["run_id"]) or {}).get("status") == "valid"]
    rng = random.Random(SEED)
    rng.shuffle(rows)
    key = []
    for i, r in enumerate(rows):
        a = final_attempt(r["run_id"])
        tm = p["task_documents"][r["task_id"]]
        mapping = tm["id_map"]  # original id -> neutral id
        text = _attempt_text(r["run_id"], a["attempt"])
        if r["method"] in ("M2", "M3"):
            cmp_path = os.path.join(RUNS, "attempts", r["run_id"], str(a["attempt"]), "comparator.json")
            text = "EXTRACTION:\n" + text.strip() + "\n\nCOMPARATOR REPORT:\n" + read_text(cmp_path)
        docs = "\n\n".join(f"=== DOCUMENT {nid} ===\n{tm['documents'][nid]}\n=== END {nid} ===" for nid in tm["order"])
        score_id = f"S{i + 1:03d}"
        body = (template.replace("{{INPUT_ID}}", score_id)
                .replace("{{RUBRIC}}", _translate(json.dumps(rubric[r["input_set"]], indent=1), mapping))
                .replace("{{DOCUMENTS}}", docs).replace("{{OUTPUT}}", text))
        d = os.path.join(RUNS, "scores", score_id)
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "task.md"), "w") as f:
            f.write(body)
        with open(os.path.join(d, "scored-output.txt"), "w") as f:
            f.write(text)
        key.append({"score_id": score_id, "run_id": r["run_id"], "attempt": a["attempt"], "task_sha256": sha256(body)})
    write_json(os.path.join(RUNS, "scores", "key.json"), {"seed": SEED, "key": key})
    return key


def record_score(score_id, output_file, run_dir):
    d = os.path.join(RUNS, "scores", score_id)
    scored = norm(read_text(os.path.join(d, "scored-output.txt")))
    with open(output_file, "rb") as f:
        raw = f.read()
    receipt = extract_receipt(run_dir, "p7-scorer")
    rec = {"score_id": score_id, "output_sha256": sha256(raw), "receipt": receipt, "quote_failures": []}
    try:
        reply = parse_typed(raw.decode(errors="replace"))
        for phase in reply["phases"]:
            for item in phase["items"]:
                if item["answer"] != "not_assessed" and norm(item.get("evidence", "")) not in scored:
                    rec["quote_failures"].append({"phase": phase["phase"], "id": item["id"]})
            for flag in phase.get("flag_census", []):
                if norm(flag.get("flag", "")) not in scored:
                    rec["quote_failures"].append({"phase": phase["phase"], "census": flag.get("flag", "")[:80]})
        rec["reply"] = reply
        rec["status"] = "invalid_receipt" if receipt["problems"] else "valid"
    except (ValueError, KeyError, TypeError) as exc:
        rec["status"], rec["error"] = "invalid_reply", f"{type(exc).__name__}: {exc}"
    n = len([x for x in os.listdir(d) if x.startswith("reply-")]) + 1
    write_json(os.path.join(d, f"reply-{n}.json"), rec)
    return rec


def human_sample():
    """One valid output per (input set, method) cell, chosen by seed, plus every score with an
    uncertain answer, a quote failure or an invalid reply."""
    key = load_json(os.path.join(RUNS, "scores", "key.json"))["key"]
    rows = {r["run_id"]: r for r in plan()["runs"]}
    cells = {}
    for k in key:
        r = rows[k["run_id"]]
        cells.setdefault((r["input_set"], r["method"]), []).append(k["score_id"])
    rng = random.Random(SEED + 1)
    sample = {rng.choice(sorted(v)) for v in cells.values()}
    for k in key:
        d = os.path.join(RUNS, "scores", k["score_id"])
        for name in os.listdir(d):
            if not name.startswith("reply-"):
                continue
            rec = load_json(os.path.join(d, name))
            uncertain = any(i["answer"] == "uncertain" for ph in rec.get("reply", {}).get("phases", []) for i in ph["items"])
            if rec["status"] != "valid" or rec["quote_failures"] or uncertain:
                sample.add(k["score_id"])
    out = sorted(sample)
    write_json(os.path.join(RUNS, "human-sample.json"), {"seed": SEED + 1, "score_ids": out})
    return out


# ---------------------------------------------------------------- report

def report():
    p = plan()
    rows = p["runs"]
    out = {"runs": len(rows), "attempt_status": {}, "cells": {}, "m3_transitions": {}, "tokens": {}}
    for r in rows:
        a = final_attempt(r["run_id"])
        status = a["status"] if a else "not_run"
        out["attempt_status"][status] = out["attempt_status"].get(status, 0) + 1
        cell = out["cells"].setdefault(f"{r['input_set']}.{r['method']}", {"attempted": 0, "valid": 0})
        cell["attempted"] += 1 if a else 0
        cell["valid"] += 1 if status == "valid" else 0
        if a:
            for u in a["receipt"]["usage"]:
                for k2, v in (u.get("usage") or {}).items():
                    if isinstance(v, (int, float)):
                        out["tokens"][k2] = out["tokens"].get(k2, 0) + v
        if r["method"] == "M3" and a and status == "valid":
            cpath = os.path.join(RUNS, "attempts", r["run_id"], str(a["attempt"]), "comparator.json")
            if os.path.exists(cpath):
                c = load_json(cpath)
                pre = {json.dumps(f, sort_keys=True) for f in c["pre"].get("derived_flags", []) + c["pre"].get("plan_flags", [])}
                post = {json.dumps(f, sort_keys=True) for f in c.get("post", {}).get("derived_flags", []) + c.get("post", {}).get("plan_flags", [])}
                inventions = sum(1 for l in c.get("fallback_log", []) if l.get("invented"))
                out["m3_transitions"][r["run_id"]] = {"removed": len(pre - post), "added": len(post - pre),
                                                      "targets": len(c.get("fallback_log", [])), "inventions": inventions}
    path = os.path.join(RUNS, "report.json")
    if os.path.exists(path):
        os.replace(path, path + ".prev")
    with open(path, "w") as f:
        json.dump(out, f, indent=2)
    return out


def main(argv):
    cmd = argv[1] if len(argv) > 1 else ""
    try:
        if cmd == "receipt":
            res = extract_receipt(argv[3], argv[2])
            print(json.dumps(res, indent=2)[:4000])
            return 0 if not res["problems"] else 21
        elif cmd == "record-attempt":
            res = record_attempt(argv[2], argv[3], argv[4])
        elif cmd == "fallback-tasks":
            res = [{k: v for k, v in o.items() if k != "body"} for o in fallback_tasks(argv[2])]
        elif cmd == "record-fallback":
            res = record_fallback(argv[2], int(argv[3]), argv[4], argv[5])
        elif cmd == "compare":
            res = compare(argv[2])
        elif cmd == "score-tasks":
            res = score_tasks()
        elif cmd == "record-score":
            res = record_score(argv[2], argv[3], argv[4])
        elif cmd == "human-sample":
            res = human_sample()
        elif cmd == "report":
            res = report()
        else:
            sys.stderr.write(__doc__)
            return 2
    except (PipelineError, comparator.ExtractionError, OSError) as exc:
        print(json.dumps({"error": str(exc)}))
        return 21
    print(json.dumps(res, indent=2)[:4000])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
