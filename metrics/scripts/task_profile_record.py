#!/usr/bin/env python3
"""Shadow-mode task-profile records for Work Engine dispatches
(metrics/dogfood-selection-plan-20261027.md). NOT ACTIVE: nothing in skills/ calls this yet.

Uses the pinned agent-routing-lab release (`taskprofile`) and writes, never overwriting:

  <run_dir>/task-profiles/<task_revision>/
    sidecar-stageB.json         the coordinator's declaration, byte for byte
    profile-stageB.json         taskprofile compute output (profile-failed.json if compute failed)
    validation.json             taskprofile validate --production result
    overhead.json               declaration timing, tool pin, unknown-field count, hashes of the files above
    assignment.json             first assignment; retained continuations add assignment-02.json, ...
    attribution-<assignment>.json  models actually used, resolved from the usage ledger
    SHA256SUMS                  append-only index; a listed file that changes is an integrity failure

Every record after the first also stores `prior_files`: the sha256 of each file that existed
when it was written. The run directory is then copied into the data repository, whose git
history is the tamper-evident copy; these local checks catch accidental edits, not an
adversary who rewrites every file.

Install a pinned tool once per commit (outside this script):
  python3 -m venv ~/.local/state/work-engine/tools/agent-routing-lab/<commit>
  ~/.local/state/work-engine/tools/agent-routing-lab/<commit>/bin/pip install \\
      "git+https://github.com/bline/agent-routing-lab@<commit>"

Usage
  task_profile_record.py check-tool [--tool-dir DIR]
  task_profile_record.py profile   --run-dir R --sidecar S --repo REPO --declare-started-at ISO8601
                                   [--call-id ID ...] [--tool-dir DIR]
  task_profile_record.py assign    --run-dir R --task-revision ID --role builder|reviewer
                                   --requested-model M --requested-effort E --harness H --run NAME
                                   --dispatched-packet PATH --child-session ID [--child-session ID ...]
                                   [--reported-model M] [--reported-effort E] [--retained]
  task_profile_record.py attribute --run-dir R --task-revision ID --ledger DIR [--assignment NAME]
  task_profile_record.py verify    --run-dir R [--repo REPO] [--no-recompute] [--tools-root DIR]

--child-session takes the child's Claude session id or Codex thread id; pass "unknown" only when
the runtime gave none, and attribution will then be unresolvable. A profiling failure never
blocks dispatch: `profile` exits 0 after recording the failure. verify exits 0 only when every
record is fully verified, 2 when hashes pass but a recompute was skipped or unavailable, 1 on
any problem.
"""
import argparse
import collections
import datetime as dt
import glob
import hashlib
import json
import os
import re
import subprocess
import sys

PIN = {"repository": "https://github.com/bline/agent-routing-lab", "tag": "v0.2.0",
       "commit": "534cd79d6c3e489bacfce2ad684187f7d3e8af85"}
TOOLS_ROOT = os.path.expanduser("~/.local/state/work-engine/tools/agent-routing-lab")
REVISION = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}@[0-9a-f]{16}\Z")
SUMS = "SHA256SUMS"


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def sha256(path):
    with open(path, "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def read_json(path):
    with open(path) as fh:
        return json.load(fh)


def read_text(path):
    with open(path, encoding="utf-8", errors="replace") as fh:
        return fh.read()


def write_once(path, data):
    if os.path.exists(path):
        raise SystemExit(f"refusing to overwrite {path}: records are write-once")
    with open(path, "x") as fh:
        if isinstance(data, (dict, list)):
            json.dump(data, fh, indent=1, sort_keys=True)
            fh.write("\n")
        else:
            fh.write(data)


def listed(d):
    path = os.path.join(d, SUMS)
    if not os.path.exists(path):
        return {}
    out = {}
    for line in read_text(path).splitlines():
        digest, name = line.split("  ", 1)
        out[name] = digest
    return out


def record_files(d):
    return sorted(n for n in os.listdir(d) if n != SUMS and not n.startswith(".") and os.path.isfile(os.path.join(d, n)))


def prior_files(d):
    return {n: sha256(os.path.join(d, n)) for n in record_files(d)}


def append_sums(d):
    """Append hashes for files not yet listed. A listed file whose bytes changed is refused."""
    have = listed(d)
    for name, digest in have.items():
        p = os.path.join(d, name)
        if not os.path.exists(p) or sha256(p) != digest:
            raise SystemExit(f"integrity: {name} in {d} no longer matches {SUMS}")
    new = [n for n in record_files(d) if n not in have]
    with open(os.path.join(d, SUMS), "a") as fh:
        fh.writelines(f"{sha256(os.path.join(d, n))}  {n}\n" for n in new)


def tool(tool_dir, commit):
    """Path to the taskprofile CLI in tool_dir after checking it was installed from `commit`."""
    exe = os.path.join(tool_dir, "bin", "taskprofile")
    found = glob.glob(os.path.join(tool_dir, "lib", "python*", "site-packages", "agent_routing_lab-*.dist-info", "direct_url.json"))
    if not os.path.isfile(exe) or not found:
        raise SystemExit(f"pinned tool not installed in {tool_dir} (see the install note in this script)")
    installed = (read_json(found[0]).get("vcs_info") or {}).get("commit_id")
    if installed != commit:
        raise SystemExit(f"tool in {tool_dir} is {installed}, expected {commit}")
    return exe


def record_dir(run_dir, rev):
    if not isinstance(rev, str) or not REVISION.match(rev):
        raise SystemExit(f"not a valid task revision id: {rev!r}")
    return os.path.join(run_dir, "task-profiles", rev)


def cmd_profile(a):
    exe = tool(a.tool_dir, PIN["commit"])
    raw = open(a.sidecar, "rb").read()
    staging = os.path.join(a.run_dir, "task-profiles", ".staging")
    os.makedirs(staging, exist_ok=True)
    out = os.path.join(staging, f"profile-{os.getpid()}.json")
    comp = subprocess.run([exe, "compute", a.sidecar, "--repo", a.repo, "-o", out], capture_output=True, text=True)
    frozen_at = now()
    if comp.returncode != 0:
        task_id = re.sub(r"[^A-Za-z0-9._-]", "-", str(json.loads(raw).get("identity", {}).get("task_id", "unknown")))[:100] or "unknown"
        d = os.path.join(a.run_dir, "task-profiles", f"{task_id}@{hashlib.sha256(raw).hexdigest()[:16]}")
        os.makedirs(d, exist_ok=False)
        write_once(os.path.join(d, "sidecar-stageB.json"), raw.decode())
        write_once(os.path.join(d, "profile-failed.json"), {"stage": "compute", "returncode": comp.returncode,
                                                            "stderr": comp.stderr[-4000:]})
        validation, unknown, rev = {"valid": False, "errors": ["compute failed"]}, None, os.path.basename(d)
    else:
        prof = read_json(out)
        rev = prof["computed"]["task_revision"]["id"]
        d = record_dir(a.run_dir, rev)
        if os.path.exists(d):
            os.remove(out)
            raise SystemExit(f"{d} exists: this task revision already has a profile")
        os.makedirs(d)
        write_once(os.path.join(d, "sidecar-stageB.json"), raw.decode())
        os.replace(out, os.path.join(d, "profile-stageB.json"))
        val = subprocess.run([exe, "validate", os.path.join(d, "profile-stageB.json"), "--production"], capture_output=True, text=True)
        errors = [] if val.returncode == 0 else [ln for ln in val.stdout.splitlines() if ln.strip()]
        validation = {"valid": val.returncode == 0, "errors": errors}
        unknown = sum(1 for c in prof["coverage"] if c["status"] == "unknown" and c["field"].startswith("declared."))
    try:
        os.rmdir(staging)
    except OSError:
        pass  # another profile is staging concurrently
    write_once(os.path.join(d, "validation.json"), validation)
    started = dt.datetime.fromisoformat(a.declare_started_at.replace("Z", "+00:00"))
    write_once(os.path.join(d, "overhead.json"), {
        "declare_started_at": a.declare_started_at, "frozen_at": frozen_at,
        "declare_wall_seconds": round((dt.datetime.fromisoformat(frozen_at) - started).total_seconds(), 1),
        "coordinator_call_ids": a.call_id or [], "tool": {**PIN, "tool_dir": a.tool_dir},
        "validation_errors": len(validation["errors"]), "unknown_declared_fields": unknown,
        "prior_files": prior_files(d)})
    append_sums(d)
    print(json.dumps({"task_revision": rev, "record_dir": d, "valid": validation["valid"]}, indent=1))


def cmd_assign(a):
    d = record_dir(a.run_dir, a.task_revision)
    if not os.path.isdir(d):
        raise SystemExit(f"no profile record for {a.task_revision} in {a.run_dir}")
    existing = sorted(glob.glob(os.path.join(d, "assignment*.json")))
    if existing and not a.retained:
        raise SystemExit(f"{existing[-1]} exists; pass --retained for a retained continuation")
    name = "assignment.json" if not existing else f"assignment-{len(existing) + 1:02d}.json"
    prof = os.path.join(d, "profile-stageB.json")
    prof = prof if os.path.exists(prof) else os.path.join(d, "profile-failed.json")
    assigned_at = now()
    frozen_at = read_json(os.path.join(d, "overhead.json"))["frozen_at"]
    packet = read_text(a.dispatched_packet)
    tokens = [a.task_revision, "task-profile/v0", "sidecar-stageB", "profile-stageB"]
    leaked = sorted({t for t in tokens if t in packet})
    children = [c for c in a.child_session if c != "unknown"]
    write_once(os.path.join(d, name), {
        "task_revision": a.task_revision, "profile_file": os.path.basename(prof), "profile_sha256": sha256(prof),
        "profile_frozen_before_assignment": frozen_at < assigned_at,
        "frozen_at": frozen_at, "assigned_at": assigned_at, "role": a.role, "run": a.run,
        "requested": {"model": a.requested_model, "effort": a.requested_effort, "harness": a.harness},
        "reported": {"model": a.reported_model or "unknown", "effort": a.reported_effort or "unknown", "source": "coordinator"},
        "child_sessions": children,
        "attribution": "pending: run `attribute` against the usage ledger" if children else "unresolvable: no child session id recorded",
        "retained_continuation": bool(existing),
        "dispatched_packet": {"path": os.path.abspath(a.dispatched_packet), "sha256": sha256(a.dispatched_packet)},
        "profile_absent_from_dispatch": not leaked, "leaked_tokens": leaked,
        "prior_files": prior_files(d)})
    append_sums(d)
    print(json.dumps({"assignment": os.path.join(d, name), "profile_absent_from_dispatch": not leaked}, indent=1))


def cmd_attribute(a):
    """Resolve which models the child sessions actually ran, from the usage ledger's calls.jsonl."""
    d = record_dir(a.run_dir, a.task_revision)
    names = sorted(glob.glob(os.path.join(d, "assignment*.json")))
    if not names:
        raise SystemExit("no assignment to attribute")
    path = os.path.join(d, a.assignment) if a.assignment else names[-1]
    asg = read_json(path)
    children = set(asg["child_sessions"])
    calls = collections.defaultdict(lambda: collections.Counter())
    span = {}
    if children:
        with open(os.path.join(a.ledger, "calls.jsonl")) as fh:
            for line in fh:
                r = json.loads(line)
                cid = r.get("thread") if r.get("provider") == "codex" else r.get("session")
                if cid in children:
                    calls[cid][(r.get("model"), r.get("effort"))] += 1
                    lo, hi = span.get(cid, (r["ts"], r["ts"]))
                    span[cid] = (min(lo, r["ts"]), max(hi, r["ts"]))
    req = asg["requested"]
    seen = sorted({m for c in calls.values() for (m, _e) in c})
    efforts = sorted({e for c in calls.values() for (_m, e) in c if e})
    if not children:
        status = "unresolvable: no child session id recorded"
    elif not calls:
        status = "unresolved: no ledger calls for the recorded child sessions"
    else:
        status = "resolved"
    out = os.path.join(d, f"attribution-{os.path.splitext(os.path.basename(path))[0]}.json")
    write_once(out, {
        "assignment": os.path.basename(path), "assignment_sha256": sha256(path), "status": status,
        "ledger": os.path.abspath(a.ledger), "ledger_calls_sha256": sha256(os.path.join(a.ledger, "calls.jsonl")),
        "per_session": {cid: {"calls": [{"model": m, "effort": e, "n": n} for (m, e), n in sorted(c.items(), key=str)],
                              "first_ts": span[cid][0], "last_ts": span[cid][1]} for cid, c in sorted(calls.items())},
        "models_seen": seen, "efforts_seen": efforts,
        "model_matches_request": (seen == [req["model"]]) if calls else None,
        "effort_matches_request": (efforts == [req["effort"]]) if efforts else None,
        "prior_files": prior_files(d)})
    append_sums(d)
    print(json.dumps({"attribution": out, "status": status, "models_seen": seen}, indent=1))


def cmd_verify(a):
    worst = 0  # 0 fully verified, 2 recompute not done, 1 problem
    for d in sorted(glob.glob(os.path.join(a.run_dir, "task-profiles", "*@*"))):
        problems, notes = [], []
        have = listed(d)
        files = record_files(d)
        if not have:
            problems.append(f"{SUMS} missing or empty")
        problems += [f"unlisted file: {n}" for n in files if n not in have]
        problems += [f"listed but missing: {n}" for n in have if n not in files]
        problems += [f"hash mismatch: {n}" for n in files if n in have and sha256(os.path.join(d, n)) != have[n]]
        for n in files:
            if n.endswith(".json"):
                try:
                    rec = read_json(os.path.join(d, n))
                except ValueError:
                    problems.append(f"not valid JSON: {n}")
                    continue
                for pn, pd in (rec.get("prior_files") or {}).items():
                    p = os.path.join(d, pn)
                    if not os.path.exists(p) or sha256(p) != pd:
                        problems.append(f"{n} recorded {pn} with a different hash")
                if "profile_sha256" in rec:
                    pf = os.path.join(d, rec.get("profile_file", "profile-stageB.json"))
                    if not os.path.exists(pf) or sha256(pf) != rec["profile_sha256"]:
                        problems.append(f"{n}: profile hash differs")
        prof = os.path.join(d, "profile-stageB.json")
        if os.path.exists(prof):
            if a.no_recompute or not a.repo:
                notes.append("recompute not done")
            else:
                pin = read_json(os.path.join(d, "overhead.json"))["tool"]["commit"]
                tdir = os.path.join(a.tools_root, pin)
                try:
                    exe = tool(tdir, pin)
                except SystemExit as e:
                    notes.append(f"recompute unavailable: {e}")
                else:
                    tmp = os.path.join(d, ".verify.json")
                    subprocess.run([exe, "compute", os.path.join(d, "sidecar-stageB.json"), "--repo", a.repo, "-o", tmp], check=True)
                    same = open(tmp, "rb").read() == open(prof, "rb").read()
                    os.remove(tmp)
                    if not same:
                        problems.append("recompute differs from profile-stageB.json")
        state = 1 if problems else (2 if notes else 0)
        worst = 1 if 1 in (worst, state) else max(worst, state)
        label = {0: "verified", 2: "hashes ok; " + "; ".join(notes), 1: "; ".join(problems)}[state]
        print(f"{os.path.basename(d)}: {label}")
    sys.exit(worst)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    default_tool = os.path.join(TOOLS_ROOT, PIN["commit"])
    c = sub.add_parser("check-tool")
    c.add_argument("--tool-dir", default=default_tool)
    p = sub.add_parser("profile")
    p.add_argument("--run-dir", required=True)
    p.add_argument("--sidecar", required=True)
    p.add_argument("--repo", required=True)
    p.add_argument("--declare-started-at", required=True)
    p.add_argument("--call-id", action="append")
    p.add_argument("--tool-dir", default=default_tool)
    s = sub.add_parser("assign")
    s.add_argument("--run-dir", required=True)
    s.add_argument("--task-revision", required=True)
    s.add_argument("--role", choices=("builder", "reviewer"), required=True)
    s.add_argument("--requested-model", required=True)
    s.add_argument("--requested-effort", required=True)
    s.add_argument("--harness", required=True)
    s.add_argument("--reported-model")
    s.add_argument("--reported-effort")
    s.add_argument("--run", required=True)
    s.add_argument("--dispatched-packet", required=True)
    s.add_argument("--child-session", action="append", required=True)
    s.add_argument("--retained", action="store_true")
    t = sub.add_parser("attribute")
    t.add_argument("--run-dir", required=True)
    t.add_argument("--task-revision", required=True)
    t.add_argument("--ledger", required=True)
    t.add_argument("--assignment")
    v = sub.add_parser("verify")
    v.add_argument("--run-dir", required=True)
    v.add_argument("--repo")
    v.add_argument("--no-recompute", action="store_true")
    v.add_argument("--tools-root", default=TOOLS_ROOT)
    a = ap.parse_args()
    if a.cmd == "check-tool":
        print(f"ok: {tool(a.tool_dir, PIN['commit'])} at {PIN['commit']}")
    else:
        {"profile": cmd_profile, "assign": cmd_assign, "attribute": cmd_attribute, "verify": cmd_verify}[a.cmd](a)


if __name__ == "__main__":
    main()
