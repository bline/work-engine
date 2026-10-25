#!/usr/bin/env python3
"""Extract every chatboard invocation and chatboard-doc read from Codex rollouts
and Claude transcripts. One JSON line per event. Deterministic; no judgment.

Derived from extract_board_use.py (sha256 cc602321705d42c85696cdbc2675d1513ac58cbbd45db1e357291a5b71b43d39,
120 lines). The classify/codex/claude extraction is kept; the changes are:
  * input is a frozen file set (metrics/probes/chatboard-baseline-inputs-*.json): only the
    first frozen line count of each file is read, and a prefix-digest mismatch exits non-zero;
  * each event carries `line` (1-based transcript line of the tool call), `idx` (position of
    the invocation within that call), `call_invocations` (board invocations in that call),
    `call_class` (see call_class(): single_board_read / multi_command / post_processed / unclassified:*),
    `call_tools` (tool invocations in that call: Codex custom_tool_call = count of
    r"tools\\.\\w+\\(" in the script input; Codex function_call and every Claude tool_use = 1;
    a single shell command that chains other commands still counts 1: not established);
  * each board event carries `output_chars` and `truncated` for its tool call's result.
    One tool call may hold several invocations (Codex batches them in one script), so these two
    fields describe the CALL result and are repeated on each of its events: count calls, not
    events, when aggregating them.

output_chars: total characters of the result text as stored in the transcript after JSON
decoding of the line, before any unescaping (Codex: sum over the text items, which
include the host wrapper such as {"chunk_id":..,"output":".."}; Claude: the tool_result
content string, or the JSON dump of a list). None when the transcript has no result.

Truncation markers (truncated=True when any matches the result text):
  TRUNC_CODEX_WARNING  r"Warning: truncated output \\(original token count: \\d+\\)"
  TRUNC_CODEX_ELISION  r"\\u2026\\d+ tokens truncated\\u2026"
  TRUNC_CLAUDE_PERSIST r"<persisted-output>\\s*Output too large \\(" 
The first two were sampled in Codex function_call_output items; the third in Claude
tool_result content (host persisted the full output to a file and returned a 2KB preview).
Claude Bash results truncated by other mechanisms would not match: not established.
"""
import hashlib, json, re, sys
from pathlib import Path

SUB = re.compile(r"codex_chatboard\.py(?:\s+--repository\s+\S+)?\s+(read|wait|post|topics|claim|release)\b")
DOC = re.compile(r"codex-chatboard(-channels)?\.md|coordination-channels/SKILL\.md")
ERR = re.compile(r"^error: |\nerror: |Process exited with code [1-9]|exit code [1-9]|Exit code [1-9]")
TRUNC = [re.compile(r"Warning: truncated output \(original token count: \d+\)"),
         re.compile(r"…\d+ tokens truncated…"),
         re.compile(r"<persisted-output>\s*Output too large \(")]


# ---- frozen-input helper (duplicated verbatim in chatboard_reader_parsers.py) ----
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


def classify(cmd):
    out = []
    for m in SUB.finditer(cmd):
        tail = cmd[m.end():m.end() + 400]
        out.append({"sub": m.group(1),
                    "help": bool(re.match(r"\s+(-h|--help)\b", tail)),
                    "parsed": bool(re.search(r"\|\s*(python3?|jq)\b", cmd)),
                    "parses_body": bool(re.search(r"\|\s*(python3?|jq)\b", cmd)) and ("body" in cmd),
                    "post_body_chars": None})
        if m.group(1) == "post":
            b = re.search(r"--body[ =](?:'([^']*)'|\"((?:[^\"\\]|\\.)*)\"|\$\(|(\S+))", tail + cmd[m.end() + 400:m.end() + 8000])
            if b and (b.group(1) or b.group(2)):
                out[-1]["post_body_chars"] = len(b.group(1) or b.group(2))
    return out


# ---- call-shape classifier (rule stated here, not by reference) ----
# call_class says whether a tool call's RESULT is the output of exactly one board read command.
#   single_board_read : exactly one tool invocation; its script is one of the bare shapes
#                       text(await tools.exec_command(ARGS)); text((await tools.exec_command(ARGS)).output);
#                       const r=await tools.exec_command(ARGS); text(r); text(r.output)  (an optional leading
#                       `// @exec:` pragma line allowed), where ARGS holds exactly one literal `cmd:` string
#                       that shlex-tokenises (newline is NOT a separator here) to ONE simple command: no
#                       ; && || | & < > ( ) token and no unquoted newline (heredocs are `<<`, so multi), whose
#                       program is [python3 [-I]] <path>codex_chatboard.py. Claude Bash: the same command rule on
#                       input.command, no JS wrapper.
#   multi_command     : the command contains a shell operator or newline, a different program, or the script
#                       uses a cmds=[...] array / loop / more than one exec_command.
#   post_processed    : one board command, but the script parses or transforms the output (JSON.parse, .map, ...)
#                       i.e. the result is a projection, not the raw read.
#   unclassified:<why>: the parser could not decide (cmd is not a plain literal, unrecognised wrapper, other
#                       tool such as apply_patch). Never guessed; counted and excluded from the primary figure.
import shlex
OPS = {";", "&&", "||", "|", "&", "<", ">", "<<", ">>", "(", ")", ";;", "|&", "&>", ">&", "<&"}
WRAP_A = re.compile(r"^(?://[^\n]*\n)?\s*text\(\(?await tools\.exec_command\(ARGS\)\)?(?:\.output)?\);?\s*$")
WRAP_B = re.compile(r"^(?://[^\n]*\n)?\s*const (\w+)\s*=\s*await tools\.exec_command\(ARGS\);\s*text\(\1(?:\.output)?\);?\s*$")
POST = re.compile(r"JSON\.parse|\.output\s*\.\s*\w|\.map\(|\.filter\(|\.split\(|\.slice\(|\.match\(|\.replace\(|JSON\.stringify")


def _js_string(s, i):
    """Read a JS string literal starting at s[i] (quote char). Returns (value, end) or None."""
    q = s[i]; out = []; i += 1
    esc = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\", "'": "'", '"': '"', "`": "`", "$": "$", "/": "/"}
    while i < len(s):
        c = s[i]
        if c == "\\":
            n = s[i + 1:i + 2]
            if n not in esc: return None
            out.append(esc[n]); i += 2; continue
        if q == "`" and c == "$" and s[i + 1:i + 2] == "{": return None
        if c == q: return "".join(out), i + 1
        out.append(c); i += 1
    return None


def _exec_args(script):
    """(script with the single exec_command(...) argument replaced by ARGS, args text) or an error string."""
    k = script.count("tools.exec_command(")
    if k != 1: return ("multi_command" if k > 1 else "unclassified:no_exec_command"), None
    a = script.index("tools.exec_command(") + len("tools.exec_command(")
    depth = 1; i = a
    while i < len(script) and depth:
        c = script[i]
        if c in "\"'`":
            r = _js_string(script, i)
            if r is None: return "unclassified:unreadable_string", None
            i = r[1]; continue
        depth += (c in "({[") - (c in ")}]"); i += 1
    if depth: return "unclassified:unbalanced", None
    return script[:a] + "ARGS" + script[i - 1:], script[a:i - 1]


def _cmd_literal(args):
    ms = list(re.finditer(r"(?<![\w.])[\"']?cmd[\"']?\s*:\s*", args))
    if len(ms) != 1: return "unclassified:cmd_key_count_%d" % len(ms)
    j = ms[0].end()
    if args[j:j + 1] not in ("'", '"', "`"): return "unclassified:cmd_not_literal"
    r = _js_string(args, j)
    if r is None: return "unclassified:cmd_literal_unreadable"
    return r[0]


def shell_shape(cmd):
    """'single' when cmd is exactly one simple command running codex_chatboard.py, else 'multi' or 'unclassified:..'."""
    lex = shlex.shlex(cmd, posix=True, punctuation_chars=True)
    lex.whitespace = " \t\r"; lex.whitespace_split = True
    try: toks = list(lex)
    except ValueError: return "unclassified:shlex_error"
    if any(t in OPS or ("\n" in t) or (t and set(t) <= set("();<>|&")) for t in toks): return "multi"
    if toks[:1] in (["python3"], ["python"]): toks = toks[1:]
    if toks[:1] == ["-I"]: toks = toks[1:]
    if not toks or not toks[0].endswith("codex_chatboard.py"): return "multi"
    return "single"


def call_class(kind, script):
    if kind == "claude_bash":
        return {"single": "single_board_read", "multi": "multi_command"}.get(shell_shape(script), shell_shape(script))
    if "tools.apply_patch(" in script or "tools.exec_command(" not in script: return "unclassified:not_exec_command"
    if re.search(r"\bcmds\b|\bfor\s*\(|\bwhile\s*\(|\.forEach\(", script): return "multi_command"
    shell, args = _exec_args(script)
    if args is None: return shell
    cmd = _cmd_literal(args)
    if cmd.startswith("unclassified"): return cmd
    sh = shell_shape(cmd)
    if sh != "single": return "multi_command" if sh == "multi" else sh
    if WRAP_A.match(shell) or WRAP_B.match(shell): return "single_board_read"
    return "post_processed" if POST.search(shell) else "unclassified:wrapper_shape"


TOOLS_RE = re.compile(r"tools\.\w+\(")


def _mark(evs, lineno, name, call_tools=1, cclass=None):
    for i, e in enumerate(evs):
        e["line"] = lineno
        e["call_tools"] = call_tools
        e["call_class"] = cclass
        e["idx"] = i
        e["call_invocations"] = sum(1 for x in evs if x["kind"] == "board")


def codex(path, lines):
    pending = {}
    awaiting_tokens = []
    done = []
    for lineno, line in enumerate(lines, 1):
        try: d = json.loads(line)
        except ValueError: continue
        p = d.get("payload") or {}
        t = p.get("type")
        if t in ("custom_tool_call", "function_call"):
            s = p.get("input") or p.get("arguments") or ""
            if not isinstance(s, str): s = json.dumps(s)
            s2 = s.replace('\\"', '"').replace("\\n", "\n")
            evs = []
            for c in classify(s2):
                evs.append({"host": "codex", "file": str(path), "ts": d.get("timestamp"), "kind": "board", **c})
            if DOC.search(s2) and re.search(r"\b(cat|sed|head|tail|less|rg|grep|nl|awk)\b", s2):
                evs.append({"host": "codex", "file": str(path), "ts": d.get("timestamp"), "kind": "doc_read",
                            "doc": sorted(set(m.group(0) for m in DOC.finditer(s2)))})
            if evs:
                _mark(evs, lineno, path, len(TOOLS_RE.findall(s)) if t == "custom_tool_call" else 1,
                      call_class("codex", s) if t == "custom_tool_call" else "unclassified:function_call")
                pending[p.get("call_id")] = evs
                awaiting_tokens.extend(evs)
        elif t in ("custom_tool_call_output", "function_call_output"):
            evs = pending.pop(p.get("call_id"), None)
            if evs:
                o = p.get("output")
                if isinstance(o, list):
                    items = [x.get("text", "") for x in o if isinstance(x, dict)]
                    o = "\n".join(items)
                    chars = sum(len(x) for x in items)
                else:
                    o = o if isinstance(o, str) else json.dumps(o)
                    chars = len(o)
                trunc = any(r.search(o) for r in TRUNC)
                o = o.replace("\\n", "\n")
                for e in evs:
                    e["error"] = bool(ERR.search(o)) if e["kind"] == "board" else None
                    if e["kind"] == "board":
                        e["output_chars"] = chars
                        e["truncated"] = trunc
                    done.append(e)
        elif t == "token_count" and awaiting_tokens:
            u = ((p.get("info") or {}).get("last_token_usage") or {})
            for e in awaiting_tokens:
                e["req_output_tokens"] = u.get("output_tokens")
                e["req_reasoning_tokens"] = u.get("reasoning_output_tokens")
            awaiting_tokens = []
    for evs in pending.values():
        for e in evs:
            e["error"] = None
            if e["kind"] == "board":
                e["output_chars"] = None
                e["truncated"] = None
            done.append(e)
    yield from done


def claude(path, lines):
    pending = {}
    for lineno, line in enumerate(lines, 1):
        try: d = json.loads(line)
        except ValueError: continue
        if d.get("type") == "assistant":
            msg = d.get("message") or {}
            u = msg.get("usage") or {}
            for c in msg.get("content") or []:
                if c.get("type") != "tool_use": continue
                inp = c.get("input") or {}
                evs = []
                cmd = inp.get("command") if isinstance(inp.get("command"), str) else ""
                for k in classify(cmd):
                    evs.append({"host": "claude", "file": str(path), "ts": d.get("timestamp"), "kind": "board", **k})
                fp = inp.get("file_path") or ""
                if (c.get("name") == "Read" and DOC.search(fp)) or (DOC.search(cmd) and re.search(r"\b(cat|sed|head|tail|less|rg|grep|nl|awk)\b", cmd)):
                    evs.append({"host": "claude", "file": str(path), "ts": d.get("timestamp"), "kind": "doc_read",
                                "doc": sorted(set(m.group(0) for m in DOC.finditer(fp + " " + cmd)))})
                for e in evs:
                    e["req_output_tokens"] = u.get("output_tokens")
                    e["req_reasoning_tokens"] = (u.get("output_tokens_details") or {}).get("thinking_tokens")
                if evs:
                    _mark(evs, lineno, path, 1, call_class("claude_bash", cmd) if c.get("name") == "Bash" else "unclassified:not_bash")
                    pending[c.get("id")] = evs
        elif d.get("type") == "user":
            content = (d.get("message") or {}).get("content")
            if not isinstance(content, list): continue
            for c in content:
                if c.get("type") != "tool_result": continue
                evs = pending.pop(c.get("tool_use_id"), None)
                if not evs: continue
                o = c.get("content"); o = o if isinstance(o, str) else json.dumps(o)
                trunc = any(r.search(o) for r in TRUNC)
                for e in evs:
                    e["error"] = (bool(c.get("is_error")) or bool(ERR.search(o))) if e["kind"] == "board" else None
                    if e["kind"] == "board":
                        e["output_chars"] = len(o)
                        e["truncated"] = trunc
                    yield e
    for evs in pending.values():
        for e in evs:
            e["error"] = None
            if e["kind"] == "board":
                e["output_chars"] = None
                e["truncated"] = None
            yield e


def events(freeze, host):
    """All events for one host over the frozen inputs, in listing order."""
    fn = codex if host == "codex" else claude
    for entry in freeze["transcripts"]:
        if entry["host"] == host:
            yield from fn(entry["path"], frozen_lines(entry))


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("host", choices=["codex", "claude"])
    ap.add_argument("out")
    ap.add_argument("--freeze", default=str(default_freeze()))
    a = ap.parse_args()
    fr = load_freeze(a.freeze)
    n = 0
    with open(a.out, "w") as w:
        for e in events(fr, a.host):
            w.write(json.dumps(e, sort_keys=True) + "\n"); n += 1
    print(a.host, "files", sum(1 for t in fr["transcripts"] if t["host"] == a.host), "events", n, file=sys.stderr)
