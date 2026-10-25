#!/usr/bin/env python3
"""Recompute the chatboard-contract baselines from frozen inputs.

Usage
  python3 -I metrics/scripts/chatboard_baselines.py [--freeze F] [--out O]
  python3 -I metrics/scripts/chatboard_baselines.py freeze --evidence-dir E --out F   (creates the freeze; refuses to overwrite)

Every transcript is read only to its frozen line count and the board only to its frozen maximum
sequence; a prefix-digest or board-digest mismatch exits non-zero. Output is deterministic (no clock,
no host data), sorted-key JSON.

Each baseline carries: id, definition, label, recomputed value, the draft's figure beside it (never
adjusted), `status`, and where stated `variants` (other definitions, each labelled). Label is "recomputed" for all of them: none of the original aggregation is
available (see plan, slice M), so no baseline can be "reproduced" in the plan's sense, except that
the reader-parser count reuses the original script's regexes verbatim over different (frozen) inputs
and is labelled "recomputed (original regexes)".

Draft regexes are NOT recorded anywhere; the ones below are this script's own and are applied to the
`body` of board messages (draft l.28-40). Counts are of matching messages, not matching phrases.
"""
import hashlib, json, re, statistics, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import chatboard_board_use as bu
import chatboard_reader_parsers as rp

REPO = Path(__file__).resolve().parents[2]
BOARD_SCRIPT = REPO / "skills" / "durable-state" / "scripts" / "codex_chatboard.py"
sys.path.insert(0, str(BOARD_SCRIPT.parent))
from durable_state import GitRefDurableStateStore          # noqa: E402
import codex_chatboard as cb                                # noqa: E402

# Store revision whose board holds exactly the frozen messages (next_sequence 2882): the frozen board is
# read by this revision, independent of the live ref and of the adapter's read projection.
STORE_REVISION = "cf1e9b6ec961a132cdc221863bc4ae115c69e74f"
DRAFT_WINDOW = 2871          # the draft's "2,871 messages" (draft l.29): variant window sequence <= 2871

# ---- regexes (stated, recomputed; the draft's originals were not recorded) ----
# "no-authority or no-effect phrase" (draft l.35). Known misses (reviewer sample of 5 near-miss non-hits,
# n = 5): plausible effect declarations such as seq 1270 "no generation implementation grant", 1259 "No main
# implementation changes, commit, or activation", 398 "no commit or push". The pattern is narrower than the
# draft's phrase and was not tuned to the draft's 88.
RE_NO_EFFECT = re.compile(
    r"\bno (?:new )?(?:authority|effects?|shared writes?|external actions?|live (?:action|provider|calls?|writes?))\b"
    r"|\bgrants? (?:no|nothing)\b|\bnot an? (?:authority|grant)\b|\bno-(?:authority|effect)\b", re.I)
# succession (draft l.32-34): three independent patterns, each counted over messages.
# RE_ENDS is case-insensitive: over sequence <= 2871 this gives the draft's 24 where case-sensitive gives 19
# (reviewer's check, n = 1 window); the case-sensitive count is reported as a variant. The draft's own regex is unrecorded.
RE_ENDS = re.compile(r"ENDS availability", re.I)
RE_ENDS_CS = re.compile(r"ENDS availability")
# RE_HANDOFF_HASH rule: a word-bounded hex run of 12-64 characters within 120 characters after "handoff". The
# count is sensitive to this length rule: RE_HANDOFF_HASH_LOOSE (hex 8-64, no leading \b) is reported as a variant
# (reviewer's variant: 135 vs 32 at 2881; misses e.g. seq 2672 "handoff09afef73"). Not tuned to the draft's 48.
RE_HANDOFF_HASH = re.compile(r"handoff[^\n]{0,120}?\b[0-9a-f]{12,64}\b", re.I)
RE_HANDOFF_HASH_LOOSE = re.compile(r"handoff[^\n]{0,120}?[0-9a-f]{8,64}", re.I)
# RE_CLAIMS_RELEASED known miss (reviewer sample, n = 5): seq 1270 "Releasing planning claim" (only released/
# reclaimed/reclaim/releases are matched). Not tuned to the draft's 69.
RE_CLAIMS_RELEASED = re.compile(
    r"\bclaims?\b[^\n]{0,60}\b(?:released|reclaimed|reclaim)\b|\b(?:released|reclaimed|releases)\b[^\n]{0,40}\bclaims?\b", re.I)
# reply / correction candidates (denominator for resolvable references); prose forms, draft l.21.
RE_REPLY_PROSE = re.compile(
    r"\bRe:?\s*#?\d{3,4}\b|\b(?:supersedes?|correction (?:to|of)|corrects?)\b[^\n]{0,40}\b\d{3,4}\b|\banswers?\s+#?\d{3,4}\b", re.I)
# a `references` entry that is itself a board sequence number.
RE_SEQ_REFERENCE = re.compile(r"^(?:(?:message|msg|seq(?:uence)?)[: #]*)?#?\d{1,5}$", re.I)

ENVELOPE_FIELDS = ("kind", "role", "to", "in_reply_to", "supersedes", "effects")


def board_messages(after=0):
    """Messages with sequence > after, read through the board adapter in pages of 500."""
    out, since = [], after
    while True:
        r = subprocess.run([sys.executable, str(BOARD_SCRIPT), "--repository", str(REPO),
                            "read", "--since", str(since), "--limit", "500"],
                           capture_output=True, text=True)
        if r.returncode != 0:
            sys.exit(f"board read failed: {r.stderr.strip()[:200]}")
        rows = json.loads(r.stdout)["messages"]
        if not rows:
            return out
        out.extend(rows)
        since = rows[-1]["sequence"]


def board_digest(messages):
    return hashlib.sha256(json.dumps(messages, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def board_at_revision(revision):
    """All board messages stored at one store revision (adapter-independent read)."""
    try:
        value = GitRefDurableStateStore(REPO, cb.NAMESPACE).read_revision(cb.BOARD_KEY, revision)
        return cb.decode(value.payload)["messages"]
    except Exception as e:
        sys.exit(f"board revision unreadable: {revision}: {e}")


def _check_board(msgs, spec, how):
    msgs = [m for m in msgs if m["sequence"] <= spec["max_sequence"]]
    if len(msgs) != spec["count"] or board_digest(msgs) != spec["sha256"]:
        sys.exit(f"board prefix digest mismatch ({how})")
    return msgs


def frozen_board(freeze):
    """The frozen board prefix (sequence <= max_sequence), read at freeze.board.store_revision and verified;
    exits non-zero on mismatch."""
    spec = freeze["board"]
    return _check_board(board_at_revision(spec["store_revision"]), spec, "store revision")


def live_crosscheck(freeze):
    """Cross-check only: does the live adapter read still reproduce the frozen prefix digest?"""
    spec = freeze["board"]
    msgs = [m for m in board_messages(0) if m["sequence"] <= spec["max_sequence"]]
    return len(msgs) == spec["count"] and board_digest(msgs) == spec["sha256"]


def call_key(e):
    return (e["file"], e["line"])


def posters_help_first(events, include_help_posts):
    by_file = {}
    for e in events:
        if e["kind"] == "board":
            by_file.setdefault(e["file"], []).append(e)
    posters = helped = 0
    for evs in by_file.values():
        evs.sort(key=lambda e: (e["line"], e["idx"]))
        posts = [e for e in evs if e["sub"] == "post" and (include_help_posts or not e["help"])]
        if not posts:
            continue
        posters += 1
        first = (posts[0]["line"], posts[0]["idx"])
        if any(e["help"] and (e["line"], e["idx"]) < first for e in evs):
            helped += 1
    return posters, helped


def raw_read_calls(events, single_tool, single_board=False, exclude_help=True, single_command=False):
    """Tool calls whose board invocations are all read/wait, none piped to a formatter, with a result.
    single_tool: the call holds exactly one tool invocation (call_tools == 1), so its result is the board
    read's result; single_board: exactly one board invocation; exclude_help: drop --help/-h invocations."""
    calls = {}
    for e in events:
        if e["kind"] == "board":
            calls.setdefault(call_key(e), []).append(e)
    out = []
    for evs in calls.values():
        if not (all(e["sub"] in ("read", "wait") and not e["parsed"] for e in evs) and evs[0]["output_chars"] is not None):
            continue
        if exclude_help and any(e["help"] for e in evs):
            continue
        if single_tool and evs[0]["call_tools"] != 1:
            continue
        if single_command and evs[0]["call_class"] != "single_board_read":
            continue
        if single_board and len(evs) != 1:
            continue
        out.append(evs[0])
    return out


def unclassified(events, single_tool=True):
    """Calls the call-shape parser could not decide (call_class starts with 'unclassified'), by reason, among the
    single_tool_call set; never guessed, never in the primary figure."""
    from collections import Counter
    return dict(Counter(e["call_class"] for e in raw_read_calls(events, single_tool) if e["call_class"].startswith("unclassified")))


def read_stats(rr):
    chars = [e["output_chars"] for e in rr]
    return {"calls": len(rr), "median_output_chars": statistics.median(chars) if chars else None,
            "truncated": sum(1 for e in rr if e["truncated"]), "truncated_of": len(rr)}


def pct(n, d):
    return round(100.0 * n / d, 1) if d else None


def baselines(freeze):
    ev = {h: list(bu.events(freeze, h)) for h in ("codex", "claude")}
    msgs = frozen_board(freeze)
    out = []

    def add(id_, definition, label, value, draft, **extra):
        extra.setdefault("status", "draft figure compared" if draft is not None else "no draft figure")
        out.append({"id": id_, "definition": definition, "label": label, "recomputed": value,
                    "draft_figure": draft, **extra})

    w = [m for m in msgs if m["sequence"] <= DRAFT_WINDOW]

    for h in ("codex", "claude"):
        p, hp = posters_help_first(ev[h], False)
        p2, hp2 = posters_help_first(ev[h], True)
        helps = sum(1 for e in ev[h] if e["kind"] == "board" and e["help"])
        helps_calls = len({call_key(e) for e in ev[h] if e["kind"] == "board" and e["help"]})
        add(f"posters_help_first.{h}",
            "Transcripts with >=1 real (non --help) `post` invocation, and of those the ones with a --help invocation "
            "of any subcommand earlier in transcript order (line, idx). Variant counts a `post --help` as a post.",
            "recomputed", {"posters": p, "ran_help_before_first_post": hp,
                           "variant_help_posts_count_as_posts": {"posters": p2, "ran_help_before_first_post": hp2}},
            "103/104 Codex posters ran --help first" if h == "codex" else None,
            status="draft figure compared" if h == "codex" else "no draft figure")
        add(f"help_calls.{h}",
            "Board invocations with a --help/-h first argument (events); variant counts distinct tool calls.",
            "recomputed", {"events": helps, "distinct_tool_calls": helps_calls},
            301 if h == "codex" else None)

    RAW_DEF = ("Codex tool calls whose board invocations are all read/wait, none piped to python/jq, none a --help/-h "
               "invocation, with a recorded result, AND call_class == single_board_read (chatboard_board_use.call_class): "
               "exactly one tool invocation in the call, whose one shell command (shlex-tokenised; no ; && || | & < > ( ) "
               "token, no unquoted newline, so no heredoc, chain, cmds=[...] array or loop) is the board read, with the "
               "script only passing the result to text() unmodified (no JSON.parse/.map/.slice/... : that makes it a "
               "projection, class post_processed). Reported: median output_chars and count with a truncation marker "
               "(markers in chatboard_board_use.py). Calls the parser cannot decide (cmd not a plain literal, unrecognised "
               "wrapper, non-exec_command tool) are class unclassified:<reason>, excluded from the primary figure and "
               "counted in `unclassified_in_single_tool_call`. Variants: single_tool_call = exactly one tool invocation "
               "(call_tools == 1, the previous primary; its shell command may still chain others, 50 of its calls do and "
               "2 post-process); all_batched_calls = any number of tool invocations per call, help calls included (result "
               "text may belong to co-batched commands, so it does NOT measure board reads); single_board_invocation = "
               "all_batched_calls restricted to one board invocation, help excluded. Whether the draft used the primary "
               "definition is not established.")
    cx = ev["codex"]
    add("codex_raw_read_calls", RAW_DEF, "recomputed", read_stats(raw_read_calls(cx, True, single_command=True)),
        "median 6,295 chars; 28/133 truncated",
        unclassified_in_single_tool_call=unclassified(cx),
        variants={"single_tool_call": read_stats(raw_read_calls(cx, True)),
                  "all_batched_calls": read_stats(raw_read_calls(cx, False, exclude_help=False)),
                  "single_board_invocation": read_stats(raw_read_calls(cx, False, single_board=True))})
    cl = ev["claude"]
    add("claude_raw_read_calls", "Same primary definition as codex_raw_read_calls over Claude transcripts: tool_use Bash whose "
        "input.command is one simple codex_chatboard.py command (same shlex rule, no JS wrapper). Non-Bash tools are unclassified. "
        "No draft figure.",
        "recomputed", read_stats(raw_read_calls(cl, True, single_command=True)), None,
        unclassified_in_single_tool_call=unclassified(cl),
        variants={"single_tool_call": read_stats(raw_read_calls(cl, True)),
                  "all_batched_calls": read_stats(raw_read_calls(cl, False, exclude_help=False))})

    rpc = rp.count(freeze)
    for h in ("codex", "claude"):
        c = rpc[h]["counts"]
        add(f"reader_parsers.{h}",
            "chatboard_reader_parsers.py (original regexes): read/wait invocations and those piped to python/jq.",
            "recomputed (original regexes, frozen inputs)",
            {"read_wait": c.get("read/wait", 0), "piped_to_formatter": c.get("piped to formatter", 0),
             "searches_body_text": c.get("searches body text", 0)},
            "13/826" if h == "codex" else "87/178 (grown inputs gave 88/179)")

    def cnt(rx, ms):
        return sum(1 for m in ms if rx.search(m["body"]))

    WV = f"sequence_le_{DRAFT_WINDOW}"
    add("no_effect_posts", "Board messages whose body matches RE_NO_EFFECT: " + RE_NO_EFFECT.pattern + " (case-insensitive). "
        "Known misses (reviewer sample, n = 5): seq 1270, 1259, 398 (effect declarations the pattern does not cover).",
        "recomputed", cnt(RE_NO_EFFECT, msgs), "88 posts (draft l.35)", of_messages=len(msgs),
        variants={WV: {"value": cnt(RE_NO_EFFECT, w), "of_messages": len(w)}})
    ends_v = {WV: {"value": cnt(RE_ENDS, w), "of_messages": len(w)},
              "case_sensitive": {"value": cnt(RE_ENDS_CS, msgs), "of_messages": len(msgs)},
              f"case_sensitive_{WV}": {"value": cnt(RE_ENDS_CS, w), "of_messages": len(w)}}
    hand_v = {WV: {"value": cnt(RE_HANDOFF_HASH, w), "of_messages": len(w)},
              "loose_hex_8_64_no_leading_word_boundary": {"value": cnt(RE_HANDOFF_HASH_LOOSE, msgs), "of_messages": len(msgs)},
              f"loose_hex_8_64_no_leading_word_boundary_{WV}": {"value": cnt(RE_HANDOFF_HASH_LOOSE, w), "of_messages": len(w)}}
    claims_v = {WV: {"value": cnt(RE_CLAIMS_RELEASED, w), "of_messages": len(w)}}
    for id_, rx, fig, note, vv in (
            ("succession_ends_availability", RE_ENDS, "24", "", ends_v),
            ("succession_handoff_hash", RE_HANDOFF_HASH, "48",
             " Rule: word-bounded hex run of 12-64 characters within 120 characters after 'handoff'; the count is sensitive to "
             "this rule (see variants; the loose 8-64 variant also admits short and unbounded hashes).", hand_v),
            ("succession_claims_released", RE_CLAIMS_RELEASED, "69",
             " Known miss (reviewer sample, n = 5): seq 1270 'Releasing planning claim'.", claims_v)):
        add(id_, f"Board messages whose body matches /{rx.pattern}/ (flags {rx.flags & re.I and 'I' or 'none'})." + note,
            "recomputed", cnt(rx, msgs), fig + " (draft l.32-34)", of_messages=len(msgs), variants=vv)
    add("distinct_authors", "Distinct `author` strings over frozen board messages.", "recomputed",
        len({m["author"] for m in msgs}), "335 over 2,871 messages (draft l.29)", of_messages=len(msgs),
        variants={WV: {"value": len({m["author"] for m in w}), "of_messages": len(w)}})
    add("resolvable_references", "See chatboard_contract_uptake.resolvable(): messages with in_reply_to/supersedes "
        "naming an existing earlier sequence, over the frozen board.", "recomputed", resolvable(msgs),
        "0 (draft l.243)", variants={WV: resolvable(w)})
    return out


def resolvable(msgs):
    seqs = {m["sequence"] for m in msgs}
    n = 0
    for m in msgs:
        for f in ("in_reply_to", "supersedes"):
            v = m.get(f)
            vals = v if isinstance(v, list) else ([v] if v is not None else [])
            if any(isinstance(x, int) and x in seqs and x < m["sequence"] for x in vals):
                n += 1
                break
    seq_refs = sum(1 for m in msgs if any(isinstance(r, str) and RE_SEQ_REFERENCE.match(r) for r in m.get("references") or []))
    cands = sum(1 for m in msgs if RE_REPLY_PROSE.search(m["body"]))
    return {"messages_with_resolvable_in_reply_to_or_supersedes": n,
            "prose_reply_or_correction_candidates_by_RE_REPLY_PROSE": cands,
            "messages_with_references_entry_that_is_a_sequence_number": seq_refs}


# ---- freeze creation ----
EXPECTED = {"extract_board_use.py": "cc602321705d42c85696cdbc2675d1513ac58cbbd45db1e357291a5b71b43d39",
            "reader_parsers.py": "f6ee651e4c6be3eac5f06668652bd94900a8664bf31512a9650fb7c9a0afc9a1"}


def make_freeze(evidence_dir, out, transcripts_from=None):
    ed = Path(evidence_dir)
    if Path(out).exists():
        sys.exit(f"refusing to overwrite {out}")
    scripts = []
    for name, want in EXPECTED.items():
        data = (ed / name).read_bytes()
        got = hashlib.sha256(data).hexdigest()
        if got != want:
            sys.exit(f"{name} digest {got} != expected {want}")
        scripts.append({"name": name, "source": str(ed / name), "sha256": got, "lines": data.count(b"\n")})
    transcripts = []
    # --transcripts-from: carry the transcript prefixes of an earlier freeze unchanged (still-growing live sessions would
    # otherwise move the frozen line counts); each carried entry is re-verified against disk here.
    if transcripts_from:
        transcripts = json.loads(Path(transcripts_from).read_text())["transcripts"]
        for entry in transcripts:
            bu.frozen_lines(entry)
    for host in (() if transcripts_from else ("claude", "codex")):
        listing = (ed / f"{host}-logs.txt").read_text().split("\n")
        paths = [l.strip() for l in listing if l.strip()]
        for p in paths:
            data = Path(p).read_bytes()
            end = data.rfind(b"\n") + 1            # complete lines only
            prefix = data[:end]
            transcripts.append({"host": host, "path": p, "lines": prefix.count(b"\n"), "bytes": len(prefix),
                                "sha256": hashlib.sha256(prefix).hexdigest(), "file_bytes_at_freeze": len(data)})
    msgs = board_at_revision(STORE_REVISION)
    if not msgs or [m["sequence"] for m in msgs][-1] != len(msgs):
        sys.exit("store revision board is not a dense sequence prefix")
    freeze = {
        "artifact_type": "chatboard_baseline_inputs_v1",
        "plan": {"path": "planning/chatboard-message-contracts-implementation-plan.md",
                 "sha256": "d14a0084ec4aa323b965a09b74d144b9c22be9cdd6c5a682f06796d6c5556bb8"},
        "frozen_by": "chatboard_baselines.py freeze",
        "line_rule": "a file's frozen prefix is its first `lines` newline-terminated lines; `sha256`/`bytes` describe that prefix",
        "copied_scripts": scripts,
        "listing_files": [{"name": f"{h}-logs.txt", "sha256": hashlib.sha256((ed / f"{h}-logs.txt").read_bytes()).hexdigest(),
                           "lines": (ed / f"{h}-logs.txt").read_bytes().count(b"\n")} for h in ("claude", "codex")],
        "original_event_files": [{"name": f"{h}-events.jsonl", "sha256": hashlib.sha256((ed / f"{h}-events.jsonl").read_bytes()).hexdigest(),
                                  "lines": (ed / f"{h}-events.jsonl").read_bytes().count(b"\n")} for h in ("claude", "codex")],
        "board": {"read_through": "GitRefDurableStateStore.read_revision(work-engine/codex-chatboard/v1, store_revision) + "
                                  "codex_chatboard.decode (git ref store, namespace codex-chatboard); live adapter read is a cross-check only",
                  "store_revision": STORE_REVISION,
                  "max_sequence": msgs[-1]["sequence"], "count": len(msgs), "sha256": board_digest(msgs),
                  "digest_rule": "sha256 of json.dumps(messages with sequence<=max_sequence, sort_keys=True, ensure_ascii=False)"},
        "transcripts": transcripts,
    }
    Path(out).write_text(json.dumps(freeze, indent=1, sort_keys=True) + "\n")
    print("froze", len(transcripts), "transcripts,", len(msgs), "board messages", file=sys.stderr)


def main(argv):
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", nargs="?", choices=["run", "freeze"], default="run")
    ap.add_argument("--freeze", default=str(bu.default_freeze()))
    ap.add_argument("--out")
    ap.add_argument("--evidence-dir")
    ap.add_argument("--transcripts-from")
    a = ap.parse_args(argv)
    if a.mode == "freeze":
        if not (a.evidence_dir and a.out):
            sys.exit("freeze needs --evidence-dir and --out")
        return make_freeze(a.evidence_dir, a.out, a.transcripts_from)
    fr = bu.load_freeze(a.freeze)
    result = {"artifact_type": "chatboard_baselines_v1",
              "freeze_sha256": hashlib.sha256(Path(a.freeze).read_bytes()).hexdigest(),
              "board_live_adapter_crosscheck_matches_frozen": live_crosscheck(fr),
              "baselines": baselines(fr)}
    text = json.dumps(result, indent=1, sort_keys=True) + "\n"
    if a.out:
        Path(a.out).write_text(text)
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main(sys.argv[1:])
