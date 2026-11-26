#!/usr/bin/env python3
"""P7 run 1 step 3 (operator-approved 2026-11-25): smoke (arm + scorer) then 15 calibration arm runs.

Each launch: role_host claude-sdk container lane, 1 request, Opus 5.5 high, no tools. Stops at the first
launch failure, non-clean receipt, or spread in the observed receipt values. Never prints the credential.
"""
import json, os, subprocess, sys, time
from pathlib import Path

REPO = Path("/home/bline/code/work-engine")
P = REPO / "experiments/perspective-intent-research/p7-run1"
HOST = REPO / "skills/manual-orchestration/scripts/role_host.py"
H = Path("~/cbm-work/p7-run1-host").expanduser()
S = Path("~/cbm-work/p7-run1-scratch").expanduser()
LOG = H / "drive.jsonl"
TASKS = ["A1-F.A1", "A2-F.M1", "A2-F.M2", "A3-F.M1", "A3-F.M2", "NC-A.M1", "NC-A.M2", "NC-B.M1", "NC-B.M2",
         "NC-C.M1", "NC-C.M2", "T1.M1", "T1.M2", "T2.M1", "T2.M2"]
H.mkdir(parents=True, exist_ok=True)
if len(sys.argv) > 1:  # re-calibration of named tasks only (no smoke)
    TASKS = sys.argv[1:]
seen = {}


def log(**kw):
    row = {"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **kw}
    with LOG.open("a") as fh:
        fh.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)


def stop(**kw):
    log(ok=False, **kw)
    sys.exit(1)


def rh(*args, timeout=600):
    r = subprocess.run([sys.executable, str(HOST), "--state", str(H), *args], capture_output=True, text=True,
                       timeout=timeout, cwd=str(REPO))
    try:
        return r.returncode, json.loads(r.stdout or r.stderr)
    except ValueError:
        return r.returncode, {"raw": (r.stdout + r.stderr)[-600:]}


def launch(label, agent, prompt_file):
    if any(S.iterdir()):
        stop(step=label, error="scratch cwd not empty")
    prompt = (P / "agents" / f"{agent}.md").read_text().rstrip("\n")
    code, row = rh("launch", "pi", "--cwd", str(S), "--name", label, "--prompt-file", str(prompt_file),
                   "--auth", "subscription", "--container", "--max-requests", "1",
                   "--host-arg=--model", "--host-arg=claude-sdk/claude-opus-5-5", "--host-arg=--thinking",
                   "--host-arg=high", "--host-arg=--no-tools", "--host-arg=--no-skills",
                   "--host-arg=--no-context-files", "--host-arg=--system-prompt", f"--host-arg={prompt}")
    if code != 0 or "session_id" not in row:
        stop(step=label, launch_error=row)
    sid = row["session_id"]
    deadline = time.time() + 1800
    while True:
        code, st = rh("status", "pi", sid)
        if code != 0 or st.get("state") != "running":
            break
        if time.time() > deadline:
            stop(step=label, session_id=sid, error="still running after 30 min")
        time.sleep(2)
    code, ret = rh("retire", "pi", sid)
    if code != 0:
        stop(step=label, session_id=sid, retire_error=ret)
    return sid


def pipeline(*args):
    r = subprocess.run([sys.executable, "-I", "-B", str(P / "pipeline.py"), *args], capture_output=True,
                       text=True, cwd=str(REPO), timeout=300)
    try:
        return r.returncode, json.loads(r.stdout)
    except ValueError:
        return r.returncode, {"raw": r.stdout[-600:] + r.stderr[-300:]}


def consistent(label, agent, rc):
    for key in ("system_sha256", "request_messages", "user_wrapper_sha256"):
        k = (agent if key == "system_sha256" else "all", key)
        if rc.get(key) is None:
            stop(step=label, error=f"{key} not observed")
        if seen.setdefault(k, rc[key]) != rc[key]:
            stop(step=label, error=f"{key} spread", first=seen[k], now=rc[key])


def summary(rc):
    return {k: rc.get(k) for k in ("session_id", "system_sha256", "request_messages", "user_wrapper_sha256",
                                   "fingerprint_sha256", "requests", "usage", "violations", "provider_failure",
                                   "unestablished")}


smoke_file = H / "smoke-task.md"
smoke_file.write_text("Reply with the single word READY.\n")
for agent in (("p7-arm", "p7-scorer") if len(sys.argv) == 1 else ()):
    label = f"smoke-{agent}"
    sid = launch(label, agent, smoke_file)
    code, rec = pipeline("record-smoke", str(H), sid, *([agent] if agent != "p7-arm" else []))
    rc = rec.get("receipt") or {}
    if code != 0 or rec.get("status") != "ok":
        stop(step=label, code=code, record=rec if "receipt" not in rec else summary(rc), status=rec.get("status"))
    consistent(label, agent, rc)
    log(ok=True, step=label, **summary(rc))

for tid in TASKS:
    label = f"calib-{tid}"
    sid = launch(label, "p7-arm", P / "tasks" / f"{tid}.md")
    code, rec = pipeline("record-calibration", tid, str(H), sid)
    rc = rec.get("receipt") or {}
    if code not in (0,) or "receipt" not in rec:
        stop(step=label, session_id=sid, code=code, record=rec)
    if rec.get("receipt_status") is not None:
        stop(step=label, receipt_status=rec["receipt_status"], **summary(rc))
    consistent(label, "p7-arm", rc)
    log(ok=True, step=label, assessed=rec.get("assessed_status"), error=rec.get("error"),
        quote_failures=len(rec.get("quote_failures") or []), leakage=rec.get("leakage_hits"),
        chars=rec.get("output_chars"), **summary(rc))

token = json.loads(Path("~/.claude/.credentials.json").expanduser().read_text())["claudeAiOauth"]["accessToken"].encode()
leaks = [str(f) for f in [*H.rglob("*"), *(P / "runs").rglob("*")] if f.is_file() and token in f.read_bytes()]
del token
log(ok=not leaks, step="custody", token_found_in=leaks)
log(ok=True, step="done", observed={f"{a}:{k}": v for (a, k), v in seen.items()})
