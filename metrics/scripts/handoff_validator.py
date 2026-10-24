#!/usr/bin/env python3
"""Deterministic root-handoff validator. No model, no network.

Checks what actually strands a successor: referents it cannot follow.
Structure is NOT checked -- across 38 real root handoffs the most common
heading appeared in 7, so requiring sections would reject real documents.

Exit codes follow the coordinator's convention:
  0   pass            every gating check ran and found no defect
  21  defect          a gating check ran and found a defect
  32  not_established a gating check could not run (never a negative result)
"""
import argparse, hashlib, json, os, re, subprocess, sys

EXT = r"md|json|jsonl|sh|py|rs|toml|txt|yaml|yml|lock|sha256|tbz2|tar|gz|rlib|so"
REPO_TOPS = ("skills", "docs", "metrics", "rust", "app-server", "planning",
             "proposals", "reviews", "experiments", "ideas", "bin", "scripts")
# Placeholders that mean "unfinished". "not established"/"not estimated" are
# legitimate recorded dispositions (CLAUDE.md rule 2) and are NOT placeholders.
PLACEHOLDER = re.compile(r"\b(TBD|TODO|FIXME|XXX+|PLACEHOLDER)\b|<(?:fill|insert|path|digest)[^>]*>|\?\?\?")
# Only hex explicitly labelled as a git object is checked; an unlabelled hex
# token may be a session id or a board sequence, not a ref.
ADJACENT = re.compile(r"""^[\s`'\"(),;:\[\]=-]*(?:is |at |with )?"""
                     r"""(?:sha-?256|sha|digest|hash|checksum)?[\s`'\"(),;:=-]*$""", re.I)
GIT_LABEL = re.compile(r"(?i)\b(?:commit|HEAD|git[- ]ref|tag)\b[^0-9a-f]{0,24}([0-9a-f]{7,40})(?![0-9a-f])")

def norm(line):
    """Split SHA256/SHA prefixes that are written flush against the digest."""
    return re.sub(r"(?i)\b(sha-?256|sha)(?=[0-9a-f]{64}(?![0-9a-f]))", r"\1 ", line)

def paths_in(line):
    out = []
    for m in re.finditer(r"[~\w.@+/-]*[\w]\.(?:" + EXT + r")(?![\w])", line):
        tok = m.group(0).strip("`'\"(),;:")
        if tok.startswith("http") or "://" in tok:
            continue
        out.append((m.start(), tok))
    return out

def bases(hdir, repo):
    """Every base a relative referent in a root handoff is written against."""
    b = [hdir]
    d = hdir
    for _ in range(4):                      # taskroot and its ancestors
        d = os.path.dirname(d)
        if not d or d == "/":
            break
        b.append(d)
    if repo:
        b.append(repo)
        b.append(os.path.join(repo, "skills", "manual-orchestration"))
        b.append(os.path.join(repo, "rust", "crates"))
    return b

def resolve(tok, hdir, repo):
    """-> (path, kind) where kind is absolute | relative | name."""
    if tok.startswith("~"):
        return os.path.expanduser(tok), "absolute"
    if os.path.isabs(tok):
        return tok, "absolute"
    if "/" not in tok:
        return tok, "name"                  # a filename mentioned in prose
    for b in bases(hdir, repo):
        p = os.path.normpath(os.path.join(b, tok))
        if os.path.exists(p):
            return p, "relative"
    return os.path.normpath(os.path.join(hdir, tok)), "relative"

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("handoff")
    ap.add_argument("--repo", default=os.environ.get("REPO", ""))
    ap.add_argument("--out", default="")
    ap.add_argument("--advisory-only", action="store_true",
                    help="report defects without failing (corpus survey)")
    ap.add_argument("--require", default="readable,path_referents,digest_claims,placeholders,git_refs",
                    help="comma-separated checks that must reach pass; others are "
                         "reported but do not gate. A check the handoff style "
                         "cannot satisfy (e.g. digests labelled by noun rather "
                         "than bound to a path) belongs outside this set, so "
                         "that 'unchecked' never reads as 'passed'.")
    a = ap.parse_args()

    r = {"handoff": a.handoff, "checks": {}, "defects": [], "not_established": []}
    # -- C1 readable -------------------------------------------------------
    try:
        raw = open(a.handoff, "rb").read()
        text = raw.decode("utf-8")
    except Exception as e:
        r["checks"]["readable"] = "not_established"
        r["not_established"].append(f"handoff unreadable: {e}")
        emit(r, a, 32); return
    if not raw.strip():
        r["checks"]["readable"] = "defect"; r["defects"].append("handoff is empty")
        emit(r, a, 21); return
    r["checks"]["readable"] = "pass"
    r["self_sha256"] = hashlib.sha256(raw).hexdigest()
    r["lines"] = text.count("\n") + 1
    hdir = os.path.dirname(os.path.abspath(a.handoff))
    repo = a.repo

    lines = text.splitlines()

    # -- C2 path referents exist ------------------------------------------
    seen, missing, ambiguous, names = {}, [], [], 0
    for i, line in enumerate(lines, 1):
        for _, tok in paths_in(line):
            p, kind = resolve(tok, hdir, repo)
            seen.setdefault(tok, (i, p, kind))
    for tok, (ln, p, kind) in sorted(seen.items()):
        if kind == "name":
            names += 1
            continue
        if os.path.exists(p):
            continue
        rec = {"line": ln, "token": tok, "resolved": p}
        (missing if kind == "absolute" else ambiguous).append(rec)
    checked = len(seen) - names
    r["path_referents_checked"] = checked
    r["path_referents_named_only"] = names
    if missing:
        r["checks"]["path_referents"] = "defect"
        r["defects"].append(f"{len(missing)} of {checked} absolute path referents do not exist")
        r["missing_paths"] = missing
    elif checked:
        r["checks"]["path_referents"] = "pass"
    else:
        r["checks"]["path_referents"] = "not_established"
        r["not_established"].append("no resolvable path referent found")
    if ambiguous:
        r["unresolved_relative_paths"] = ambiguous
        r["not_established"].append(
            f"{len(ambiguous)} relative referent(s) resolve under no known base")

    # -- C3 digest claims verify ------------------------------------------
    pairs, unpaired = [], 0
    for i, line in enumerate(lines, 1):
        nl = norm(line)
        ps = paths_in(nl)
        for m in re.finditer(r"(?<![0-9a-f])[0-9a-f]{64}(?![0-9a-f])", nl):
            # A digest is a FILE claim only when it is syntactically attached to
            # the path. In dense prose a nearby hex is often a supervisor id, a
            # review id or a binding -- attributing those to the nearest path
            # produced false mismatches, so only adjacency counts.
            cand = [(pos, t) for (pos, t) in ps if pos + len(t) <= m.start()]
            if not cand:
                unpaired += 1; continue
            pos, tok = cand[-1]
            gap = nl[pos + len(tok):m.start()]
            if not ADJACENT.match(gap):
                unpaired += 1; continue
            pairs.append((i, tok, m.group(0)))
    mismatch, unverifiable = [], []
    for ln, tok, dig in pairs:
        p, _kind = resolve(tok, hdir, repo)
        try:
            with open(p, "rb") as fh:
                h = hashlib.sha256(fh.read()).hexdigest()
        except Exception as e:
            unverifiable.append({"line": ln, "token": tok, "reason": str(e)}); continue
        if h != dig:
            mismatch.append({"line": ln, "token": tok, "claimed": dig, "actual": h})
    r["digest_claims_paired"] = len(pairs)
    r["digest_tokens_unpaired"] = unpaired
    if mismatch:
        r["checks"]["digest_claims"] = "defect"
        r["defects"].append(f"{len(mismatch)} of {len(pairs)} paired digest claims do not match the file")
        r["digest_mismatches"] = mismatch
    elif pairs:
        r["checks"]["digest_claims"] = "pass"
    else:
        r["checks"]["digest_claims"] = "not_established"
        r["not_established"].append("no path/digest pair could be extracted")
    if unverifiable:
        r["digest_unverifiable"] = unverifiable
        r["not_established"].append(f"{len(unverifiable)} digest claims unverifiable (file unreadable)")

    # -- C4 placeholders ---------------------------------------------------
    # Inline code spans carry command templates (`git cat-file -e <base>:<path>`),
    # which are instructions to the successor, not unfilled fields.
    ph = [{"line": i, "text": line.strip()[:120]}
          for i, line in enumerate(lines, 1)
          if PLACEHOLDER.search(re.sub(r"`[^`]*`", "", line))]
    r["checks"]["placeholders"] = "defect" if ph else "pass"
    if ph:
        r["defects"].append(f"{len(ph)} unresolved placeholder(s)")
        r["placeholders"] = ph

    # -- C5 labelled git refs resolve -------------------------------------
    refs = {}
    for i, line in enumerate(lines, 1):
        for m in GIT_LABEL.finditer(line):
            refs.setdefault(m.group(1), i)
    if not refs:
        r["checks"]["git_refs"] = "not_established"
        r["not_established"].append("no explicitly labelled git ref found")
    elif not repo or not os.path.isdir(os.path.join(repo, ".git")):
        r["checks"]["git_refs"] = "not_established"
        r["not_established"].append("repo not given or not a git repo; labelled refs unchecked")
    else:
        bad = []
        for h, ln in sorted(refs.items()):
            rc = subprocess.run(["git", "-C", repo, "cat-file", "-e", h + "^{object}"],
                                capture_output=True)
            if rc.returncode != 0:
                bad.append({"line": ln, "ref": h})
        r["git_refs_checked"] = len(refs)
        if bad:
            r["checks"]["git_refs"] = "defect"
            r["defects"].append(f"{len(bad)} of {len(refs)} labelled git refs do not resolve")
            r["git_refs_unresolved"] = bad
        else:
            r["checks"]["git_refs"] = "pass"

    # -- C6 doctrine elements (ADVISORY: presence of a referent, not adequacy)
    want = {
        "objective_and_authority": r"(?i)\b(objective|authority|authorized|mandate)\b",
        "open_questions_or_decisions": r"(?i)\b(open question|decision|accepted|awaiting)\b",
        "assignment_record": r"(?i)\b(assignment|continuation|run record|disposition)\b",
        "child_handoffs": r"(?i)\b(child|worker|builder|reviewer|supervisor|handoff-v|no children)\b",
        "claims_held": r"(?i)\b(claim|board|release[ds]?)\b",
        "destination_ref": r"(?i)\b(branch|HEAD|commit|ref|unpushed)\b",
        "next_actions": r"(?i)\b(next action|next step|next bounded|start here|successor must|first action)\b",
    }
    adv = {k: ("present" if re.search(v, text) else "not_established") for k, v in want.items()}
    r["doctrine_elements_advisory"] = adv
    r["checks"]["doctrine_elements"] = "advisory"

    gating = [c for c in (x.strip() for x in a.require.split(",")) if c]
    r["gating_checks"] = gating
    r["non_gating_outcomes"] = {k: v for k, v in r["checks"].items() if k not in gating}
    # A defect anywhere is still a defect, gating or not; the gating set only
    # decides whether an unrunnable check blocks.
    if any(v == "defect" for v in r["checks"].values()):
        emit(r, a, 21); return
    if any(r["checks"].get(k) == "not_established" for k in gating):
        emit(r, a, 32); return
    emit(r, a, 0)

def emit(r, a, code):
    r["verdict"] = {0: "pass", 21: "defect", 32: "not_established"}[code]
    r["exit"] = code
    s = json.dumps(r, indent=2, sort_keys=True)
    if a.out:
        open(a.out, "w").write(s + "\n")
    print(s)
    sys.exit(0 if a.advisory_only else code)

if __name__ == "__main__":
    main()
