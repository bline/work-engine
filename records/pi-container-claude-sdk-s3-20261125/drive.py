#!/usr/bin/env python3
"""S3 live smoke driver (operator-approved 2026-11-25): <= 6 Claude subscription requests.

A: Haiku launch (read tool, cap 2)  B: Haiku delivery (cap 2)  C: retire + custody check
E: Opus, --append-system-prompt, no tools, cap 1   D: Opus, --system-prompt, no tools, cap 1
Stops at the first failure. Never prints the credential.
"""
import json, os, subprocess, sys, time
from pathlib import Path

REPO = Path("/home/bline/code/work-engine")
HOST = REPO / "skills/manual-orchestration/scripts/role_host.py"
RUN = Path(sys.argv[1])
STATE, WS = RUN / "state", RUN / "ws"
for d in (STATE, WS):
    d.mkdir(parents=True, exist_ok=True)
(WS / "hello.txt").write_text("s3-smoke-file-contents\n")
OUT = RUN / "results.jsonl"
PROMPT = "You are a scorer in a test of a scoring pipeline. Reply with exactly one word: READY"


def rh(*args):
    r = subprocess.run([sys.executable, str(HOST), "--state", str(STATE), *args], capture_output=True, text=True, timeout=600)
    return r.returncode, (json.loads(r.stdout) if r.returncode == 0 and r.stdout.strip() else
                          (json.loads(r.stderr) if r.stderr.strip().startswith("{") else {"raw": r.stderr[-500:]}))


def wait(sid):
    for _ in range(3000):
        code, st = rh("status", "pi", sid)
        if code != 0 or st.get("state") != "running":
            return code, st
        time.sleep(0.2)
    return 1, {"error": "timeout"}


def summarize(st):
    t = st.get("last_turn") or {}
    sdk = (t.get("egress") or {}).get("claude_sdk") or {}
    return {"outcome": t.get("outcome"), "stop_reason": t.get("stop_reason"), "error": (t.get("error") or "")[:300],
            "evidence": sdk.get("evidence"), "outcome_known": sdk.get("outcome_known"), "admitted": sdk.get("admitted"),
            "completed": sdk.get("completed"), "refused": sdk.get("refused"), "billing": sdk.get("billing_verdicts"),
            "models": sdk.get("response_models"), "usage": sdk.get("usage"), "requests": sdk.get("requests"),
            "container_removed": (t.get("container") or {}).get("removed")}


def ok(s, max_admitted):
    return (s["outcome"] == "completed" and s["evidence"] == "complete" and s["outcome_known"]
            and s["billing"] == ["allowed"] and 1 <= (s["admitted"] or 0) <= max_admitted and s["container_removed"])


def record(step, **kw):
    row = {"step": step, "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **kw}
    with OUT.open("a") as fh:
        fh.write(json.dumps(row) + "\n")
    print(json.dumps(row))


def launch(step, model, prompt, cap, *host_args):
    p = RUN / f"{step}.prompt"
    p.write_text(prompt)
    code, row = rh("launch", "pi", "--cwd", str(WS), "--name", f"s3-{step}", "--prompt-file", str(p),
                   "--auth", "subscription", "--container", "--max-requests", str(cap),
                   "--host-arg=--model", f"--host-arg=claude-sdk/{model}", *host_args)
    if code != 0:
        record(step, ok=False, launch_error=row)
        sys.exit(1)
    code, st = wait(row["session_id"])
    s = summarize(st)
    good = code == 0 and ok(s, cap) and s["models"] == [model]
    record(step, ok=good, session_id=row["session_id"], **s)
    if not good:
        sys.exit(1)
    return row["session_id"]


sid = launch("A-haiku-launch", "claude-haiku-5-5",
             "Use the read tool to read hello.txt, then reply with its contents on one line.", 2,
             "--host-arg=--tools", "--host-arg=read")
p = RUN / "B.prompt"
p.write_text("Reply with the single word DONE.")
code, got = rh("deliver", "pi", sid, "--cwd", str(WS), "--message-file", str(p))
if code != 0:
    record("B-haiku-deliver", ok=False, deliver_error=got)
    sys.exit(1)
code, st = wait(sid)
s = summarize(st)
good = code == 0 and ok(s, 2) and s["models"] == ["claude-haiku-5-5"]
record("B-haiku-deliver", ok=good, turn=got.get("turn"), **s)
if not good:
    sys.exit(1)
code, retired = rh("retire", "pi", sid)
token = json.loads(Path("~/.claude/.credentials.json").expanduser().read_text())["claudeAiOauth"]["accessToken"].encode()
leaks = [str(f) for f in RUN.rglob("*") if f.is_file() and token in f.read_bytes()]
del token
record("C-retire-custody", ok=code == 0 and not leaks, retire=retired.get("retired"), token_found_in=leaks)
if code != 0 or leaks:
    sys.exit(1)
launch("E-opus-append-prompt", "claude-opus-5-5", "Reply now.", 1,
       "--host-arg=--no-tools", "--host-arg=--thinking", "--host-arg=high", "--host-arg=--append-system-prompt",
       f"--host-arg={PROMPT}")
launch("D-opus-replaced-prompt", "claude-opus-5-5", "Reply now.", 1,
       "--host-arg=--no-tools", "--host-arg=--thinking", "--host-arg=high", "--host-arg=--system-prompt",
       f"--host-arg={PROMPT}")
record("done", ok=True)
