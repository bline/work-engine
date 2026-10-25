#!/usr/bin/env python3
"""Count chatboard read/wait invocations piped to one-off formatters.

Derived from reader_parsers.py (sha256 f6ee651e4c6be3eac5f06668652bd94900a8664bf31512a9650fb7c9a0afc9a1,
34 lines). The regexes SUB, PIPE, SEARCH and the two counting conditions are the original's, unchanged.
The only change: input is the frozen file set (metrics/probes/chatboard-baseline-inputs-*.json) instead of
`<dir>/{host}-logs.txt` listings read live; each file is read to its frozen line count and a prefix-digest
mismatch exits non-zero.

Counts (per host): "read/wait" = invocations matching SUB; "piped to formatter" = those whose text up to the
next `codex_chatboard.py` matches PIPE; "searches body text" = piped ones whose segment mentions body and
matches the in-segment search regex below.
"""
import hashlib, json, re, sys, collections
from pathlib import Path

# ---- frozen-input helper (duplicated verbatim in chatboard_board_use.py) ----
def default_freeze():
    return Path(__file__).resolve().parents[1] / "probes" / "chatboard-baseline-inputs-20261025.json"

def load_freeze(path):
    return json.loads(Path(path).read_text())

def frozen_lines(entry):
    """First entry['lines'] newline-terminated lines of entry['path'] as str; exits non-zero on any mismatch."""
    try:
        data = Path(entry["path"]).read_bytes()
    except OSError as e:
        sys.exit(f"frozen input unreadable: {entry['path']}: {e}")
    parts = data.split(b"\n")
    n = entry["lines"]
    if len(parts) - 1 < n:
        sys.exit(f"frozen input shorter than frozen line count: {entry['path']}")
    prefix = b"\n".join(parts[:n]) + b"\n"
    if len(prefix) != entry["bytes"] or hashlib.sha256(prefix).hexdigest() != entry["sha256"]:
        sys.exit(f"prefix digest mismatch: {entry['path']}")
    return prefix.decode("utf-8", errors="replace").split("\n")[:n]
# ---- end helper ----

SUB=re.compile(r"codex_chatboard\.py(?:\s+--repository\s+\S+)?\s+(read|wait)\b")
PIPE=re.compile(r"\|\s*(python3?|jq)\b")
SEARCH=re.compile(r"in\s+\w+\[['\"]body['\"]\]|re\.(search|findall|match)\([^)]*\w+\[['\"]body['\"]\]|\.get\(['\"]body['\"][^)]*\)\s*(\.lower\(\))?\s*\)?\s*$|pat\.search\(\s*\w+\[['\"]body['\"]\]|search\(\w+\)|\bb=\w+\[['\"]body['\"]\]")
BODY_SEARCH=re.compile(r"\bin\s+\w+(\.get\(['\"]body['\"][^)]*\)|\[['\"]body['\"]\])|re\.\w+\([^\n]*body|pat\.search|\.search\(\s*\w*b\w*\)")

def cmds(host,lines):
    for line in lines:
        try:d=json.loads(line)
        except ValueError: continue
        if host=="codex":
            p=d.get("payload") or {}
            if p.get("type") in("custom_tool_call","function_call"):
                s=p.get("input") or p.get("arguments") or ""
                if not isinstance(s,str): s=json.dumps(s)
                yield s.replace('\\"','"').replace("\\n","\n").replace("\\'","'")
        elif d.get("type")=="assistant":
            for c in (d.get("message") or {}).get("content") or []:
                if c.get("type")=="tool_use":
                    x=(c.get("input") or {}).get("command")
                    if isinstance(x,str): yield x

def count(freeze):
    """{host: {"counts": {...}, "examples": [...], "files": n}} over the frozen inputs."""
    result={}
    for host in ("codex","claude"):
        c=collections.Counter(); ex=[]; nf=0
        for entry in freeze["transcripts"]:
            if entry["host"]!=host: continue
            nf+=1
            for s in cmds(host,frozen_lines(entry)):
                for m in SUB.finditer(s):
                    c["read/wait"]+=1
                    rest=s[m.end():m.end()+3000]
                    seg=rest.split("codex_chatboard.py")[0]
                    if PIPE.search(seg):
                        c["piped to formatter"]+=1
                        if "body" in seg and BODY_SEARCH.search(seg):
                            c["searches body text"]+=1
                            if len(ex)<6: ex.append(seg[:260].replace("\n"," | "))
        result[host]={"counts":dict(c),"examples":ex,"files":nf}
    return result

if __name__=="__main__":
    import argparse
    ap=argparse.ArgumentParser(); ap.add_argument("--freeze",default=str(default_freeze()))
    a=ap.parse_args()
    for host,r in count(load_freeze(a.freeze)).items():
        print(host,r["counts"]); [print("   ",e) for e in r["examples"]]
