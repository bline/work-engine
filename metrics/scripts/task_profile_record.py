#!/usr/bin/env python3
"""Shadow-mode task-profile records for Work Engine dispatches
(metrics/dogfood-selection-plan-20261027.md). NOT ACTIVE: nothing in skills/ calls this yet.

Uses the pinned agent-routing-lab release (`taskprofile`) and writes, never overwriting:

  <run_dir>/task-profiles/<task_revision>/
    sidecar-stageB.json     the coordinator's declaration, byte for byte
    profile-stageB.json     taskprofile compute output (profile-failed.json if compute failed)
    validation.json         taskprofile validate --production result
    assignment.json         first assignment; later retained continuations add assignment-02.json, ...
    overhead.json           declaration timing, tool pin, unknown-field count, call ids
    SHA256SUMS              index of the files above (rewritten when a file is added)

Install the pinned tool once (outside this script):
  python3 -m venv ~/.local/state/work-engine/tools/agent-routing-lab/<commit>
  ~/.local/state/work-engine/tools/agent-routing-lab/<commit>/bin/pip install \
      "git+https://github.com/bline/agent-routing-lab@<commit>"

Usage
  task_profile_record.py check-tool [--tool-dir DIR]
  task_profile_record.py profile --run-dir R --sidecar S --repo REPO --declare-started-at ISO8601
                                 [--call-id ID ...] [--tool-dir DIR]
  task_profile_record.py assign  --run-dir R --task-revision ID --role builder|reviewer
                                 --requested-model M --requested-effort E --harness H --run NAME
                                 --dispatched-packet PATH [--observed-model M] [--observed-effort E]
                                 [--retained]
  task_profile_record.py verify  --run-dir R [--repo REPO] [--tool-dir DIR]

A profiling failure never blocks dispatch: `profile` exits 0 after recording the failure, and only
argument or write-once errors exit non-zero.
"""
import argparse
import datetime as dt
import glob
import hashlib
import json
import os
import re
import subprocess
import sys

PIN = {"repository": "https://github.com/bline/agent-routing-lab", "tag": "v0.1.0",
       "commit": "54f9b35b5e2af9b80873205ff61e7636f0017d49"}
DEFAULT_TOOL_DIR = os.path.expanduser(f"~/.local/state/work-engine/tools/agent-routing-lab/{PIN['commit']}")


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def sha256(path):
    with open(path, "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def read_json(path):
    with open(path) as fh:
        return json.load(fh)


def write_once(path, data):
    if os.path.exists(path):
        raise SystemExit(f"refusing to overwrite {path}: records are write-once")
    with open(path, "x") as fh:
        if isinstance(data, (dict, list)):
            json.dump(data, fh, indent=1, sort_keys=True)
            fh.write("\n")
        else:
            fh.write(data)


def write_sums(record_dir):
    names = sorted(n for n in os.listdir(record_dir) if n != "SHA256SUMS" and os.path.isfile(os.path.join(record_dir, n)))
    with open(os.path.join(record_dir, "SHA256SUMS"), "w") as fh:
        fh.writelines(f"{sha256(os.path.join(record_dir, n))}  {n}\n" for n in names)


def tool(tool_dir):
    """Path to the pinned taskprofile CLI after checking the installed commit; exits if it does not match."""
    exe = os.path.join(tool_dir, "bin", "taskprofile")
    found = glob.glob(os.path.join(tool_dir, "lib", "python*", "site-packages", "agent_routing_lab-*.dist-info", "direct_url.json"))
    if not os.path.isfile(exe) or not found:
        raise SystemExit(f"pinned tool not installed in {tool_dir} (see the install note in this script)")
    commit = (read_json(found[0]).get("vcs_info") or {}).get("commit_id")
    if commit != PIN["commit"]:
        raise SystemExit(f"installed agent-routing-lab is {commit}, pin is {PIN['commit']}")
    return exe


def record_dir(run_dir, rev):
    safe = re.sub(r"[^A-Za-z0-9._@-]", "_", rev)
    return os.path.join(run_dir, "task-profiles", safe)


def cmd_profile(a):
    exe = tool(a.tool_dir)
    raw = open(a.sidecar, "rb").read()
    staging = os.path.join(a.run_dir, "task-profiles", ".staging")
    os.makedirs(staging, exist_ok=True)
    out = os.path.join(staging, f"profile-{os.getpid()}.json")
    comp = subprocess.run([exe, "compute", a.sidecar, "--repo", a.repo, "-o", out], capture_output=True, text=True)
    frozen_at = now()
    if comp.returncode != 0:
        sidecar = json.loads(raw)
        rev = f"{sidecar.get('identity', {}).get('task_id', 'unknown')}@compute-failed-{hashlib.sha256(raw).hexdigest()[:16]}"
        d = record_dir(a.run_dir, rev)
        os.makedirs(d, exist_ok=False)
        write_once(os.path.join(d, "sidecar-stageB.json"), raw.decode())
        write_once(os.path.join(d, "profile-failed.json"), {"stage": "compute", "returncode": comp.returncode,
                                                            "stderr": comp.stderr[-4000:]})
        validation = {"valid": False, "errors": ["compute failed"]}
        unknown = None
    else:
        prof = read_json(out)
        rev = prof["computed"]["task_revision"]["id"] or f"{prof['identity']['task_id']}@no-source-documents"
        d = record_dir(a.run_dir, rev)
        if os.path.exists(d):
            os.remove(out)
            raise SystemExit(f"{d} exists: this task revision already has a profile")
        os.makedirs(d)
        write_once(os.path.join(d, "sidecar-stageB.json"), raw.decode())
        os.replace(out, os.path.join(d, "profile-stageB.json"))
        try:
            os.rmdir(staging)
        except OSError:
            pass  # another profile is staging concurrently
        val = subprocess.run([exe, "validate", os.path.join(d, "profile-stageB.json"), "--production"], capture_output=True, text=True)
        errors = [] if val.returncode == 0 else [ln for ln in val.stdout.splitlines() if ln.strip()]
        validation = {"valid": val.returncode == 0, "errors": errors}
        unknown = sum(1 for c in prof["coverage"] if c["status"] == "unknown" and c["field"].startswith("declared."))
    write_once(os.path.join(d, "validation.json"), validation)
    started = dt.datetime.fromisoformat(a.declare_started_at.replace("Z", "+00:00"))
    write_once(os.path.join(d, "overhead.json"), {
        "declare_started_at": a.declare_started_at, "frozen_at": frozen_at,
        "declare_wall_seconds": round((dt.datetime.fromisoformat(frozen_at) - started).total_seconds(), 1),
        "coordinator_call_ids": a.call_id or [], "tool": {**PIN, "tool_dir": a.tool_dir},
        "validation_errors": len(validation["errors"]), "unknown_declared_fields": unknown})
    write_sums(d)
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
    packet = open(a.dispatched_packet, encoding="utf-8", errors="replace").read()
    tokens = [a.task_revision, "task-profile/v0", "sidecar-stageB", "profile-stageB", os.path.basename(d)]
    leaked = sorted({t for t in tokens if t and t in packet})
    write_once(os.path.join(d, name), {
        "task_revision": a.task_revision, "profile_file": os.path.basename(prof), "profile_sha256": sha256(prof),
        "profile_frozen_before_assignment": frozen_at < assigned_at,
        "frozen_at": frozen_at, "assigned_at": assigned_at, "role": a.role, "run": a.run,
        "requested": {"model": a.requested_model, "effort": a.requested_effort, "harness": a.harness},
        "observed": {"model": a.observed_model or "unknown", "effort": a.observed_effort or "unknown"},
        "retained_continuation": bool(existing),
        "dispatched_packet": {"path": os.path.abspath(a.dispatched_packet), "sha256": sha256(a.dispatched_packet)},
        "profile_absent_from_dispatch": not leaked, "leaked_tokens": leaked})
    write_sums(d)
    print(json.dumps({"assignment": os.path.join(d, name), "profile_absent_from_dispatch": not leaked}, indent=1))


def cmd_verify(a):
    exe = tool(a.tool_dir) if a.repo else None
    bad = 0
    for d in sorted(glob.glob(os.path.join(a.run_dir, "task-profiles", "*@*"))):
        problems = []
        sums = os.path.join(d, "SHA256SUMS")
        for line in open(sums).read().splitlines() if os.path.exists(sums) else []:
            digest, name = line.split("  ", 1)
            if not os.path.exists(os.path.join(d, name)) or sha256(os.path.join(d, name)) != digest:
                problems.append(f"hash mismatch: {name}")
        prof = os.path.join(d, "profile-stageB.json")
        if exe and os.path.exists(prof):
            tmp = os.path.join(d, ".verify.json")
            subprocess.run([exe, "compute", os.path.join(d, "sidecar-stageB.json"), "--repo", a.repo, "-o", tmp], check=True)
            same = open(tmp, "rb").read() == open(prof, "rb").read()
            os.remove(tmp)
            if not same:
                problems.append("recompute differs from profile-stageB.json")
        bad += bool(problems)
        print(f"{os.path.basename(d)}: {'ok' if not problems else '; '.join(problems)}")
    sys.exit(1 if bad else 0)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("check-tool")
    c.add_argument("--tool-dir", default=DEFAULT_TOOL_DIR)
    p = sub.add_parser("profile")
    p.add_argument("--run-dir", required=True)
    p.add_argument("--sidecar", required=True)
    p.add_argument("--repo", required=True)
    p.add_argument("--declare-started-at", required=True)
    p.add_argument("--call-id", action="append")
    p.add_argument("--tool-dir", default=DEFAULT_TOOL_DIR)
    s = sub.add_parser("assign")
    s.add_argument("--run-dir", required=True)
    s.add_argument("--task-revision", required=True)
    s.add_argument("--role", choices=("builder", "reviewer"), required=True)
    s.add_argument("--requested-model", required=True)
    s.add_argument("--requested-effort", required=True)
    s.add_argument("--harness", required=True)
    s.add_argument("--observed-model")
    s.add_argument("--observed-effort")
    s.add_argument("--run", required=True)
    s.add_argument("--dispatched-packet", required=True)
    s.add_argument("--retained", action="store_true")
    v = sub.add_parser("verify")
    v.add_argument("--run-dir", required=True)
    v.add_argument("--repo")
    v.add_argument("--tool-dir", default=DEFAULT_TOOL_DIR)
    a = ap.parse_args()
    if a.cmd == "check-tool":
        print(f"ok: {tool(a.tool_dir)} at {PIN['commit']}")
    else:
        {"profile": cmd_profile, "assign": cmd_assign, "verify": cmd_verify}[a.cmd](a)


if __name__ == "__main__":
    main()
