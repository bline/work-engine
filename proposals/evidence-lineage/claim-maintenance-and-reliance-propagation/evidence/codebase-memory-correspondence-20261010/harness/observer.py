"""Disposable prototype of evidence-anchor's CodeEvidenceAdapter / document adapter.

Produces RAW facts first, then maps them onto the accepted five-state
comparator (matches / differs / unknown / unsupported / failed). The mapping
is deliberately separate so cases the five states cannot express are visible
as evidence for open question #7, not silently absorbed.

Resolution is strict: an anchor resolves only to an entity of the declared
kind, in the declared file, with the declared name and owner. There is no
file-level fallback. Successor discovery is a separate, non-binding step.
"""
import hashlib
import json
import os

import cbm

STATES = ("matches", "differs", "unknown", "unsupported", "failed")
SUPPORTED_KINDS = {"code_structure", "text"}
MAX_RETRIES = 3


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


class Canonical:
    """Stand-in for claim-evidence canonical records (JSON file outside the repo)."""

    def __init__(self, path):
        self.path = path

    def load(self):
        with open(self.path) as f:
            return json.load(f)

    def save(self, data):
        data["revision"] += 1
        with open(self.path, "w") as f:
            json.dump(data, f, indent=1)

    def revision(self):
        return self.load()["revision"]


# ── raw resolution (supported interface only) ─────────────────────────────
def resolve_code(project, loc):
    """All entities matching the locator exactly. loc = {path, symbol, label}."""
    owner, _, name = loc["symbol"].rpartition(".")
    rows = cbm.query(project, (
        f"MATCH (n) WHERE n.file_path = {cbm.q(loc['path'])} AND n.name = {cbm.q(name)} "
        "RETURN n.qualified_name, n.label, n.start_line, n.end_line"))
    hits = []
    for r in rows:
        if r["n.label"] != loc["label"]:
            continue
        if owner and not r["n.qualified_name"].endswith("." + loc["symbol"]):
            continue
        hits.append(r)
    return hits


def tail_digest(s):
    return sha("\n".join(s.get("source", "").split("\n")[1:]))


def body_digest(project, qn):
    s = cbm.snippet(project, qn)
    return sha(s.get("source", "")), s


def successor_candidates(project, loc, bound_digest, bound_tail=None):
    """Same name + kind elsewhere; evidence only, never a rebinding."""
    name = loc["symbol"].rpartition(".")[2]
    rows = cbm.query(project, (
        f"MATCH (n) WHERE n.name = {cbm.q(name)} "
        "RETURN n.qualified_name, n.label, n.file_path"))
    out = []
    for r in rows:
        if r["n.label"] != loc["label"]:
            continue
        d, _ = body_digest(project, r["n.qualified_name"])
        out.append({"qn": r["n.qualified_name"], "file": r["n.file_path"],
                    "body_digest_equal": d == bound_digest})
    # renames: same kind and same body under another name in the declared file
    seen = {o["qn"] for o in out}
    rows = cbm.query(project, (
        f"MATCH (n) WHERE n.file_path = {cbm.q(loc['path'])} "
        "RETURN n.qualified_name, n.label, n.file_path"))
    for r in rows:
        if r["n.label"] != loc["label"] or r["n.qualified_name"] in seen:
            continue
        d, s = body_digest(project, r["n.qualified_name"])
        out.append({"qn": r["n.qualified_name"], "file": r["n.file_path"],
                    "body_digest_equal": d == bound_digest,
                    # a rename changes the def line, so also compare without it
                    "tail_digest_equal": tail_digest(s) == bound_tail, "via": "same_file"})
    return out


def coverage_facts(project, path, lines=None):
    c = cbm.coverage(project, [path])
    p = c["paths"][0]
    issues = []
    for cov in p.get("coverage", []):
        issues.append(cov)
    meta = c.get("metadata", {})
    return {
        "generation": meta.get("generation"),
        "recorded_at": meta.get("recorded_at"),
        "recording_complete": meta.get("recording_status") == "complete",
        "hash_records_complete": meta.get("hash_records_complete"),
        "freshness": p.get("freshness"),
        "status": p.get("status"),
        "issues": issues,
    }


# ── observation ───────────────────────────────────────────────────────────
def observe_code_once(project, repo, anchor, bound):
    loc = anchor["locator"]
    cov = coverage_facts(project, loc["path"])
    raw = {"coverage": cov}
    on_disk = os.path.exists(os.path.join(repo, loc["path"]))
    raw["file_on_disk"] = on_disk
    hits = resolve_code(project, loc)
    raw["hits"] = [h["n.qualified_name"] for h in hits]
    if len(hits) == 1:
        d, s = body_digest(project, hits[0]["n.qualified_name"])
        raw["observed_digest"] = d
        raw["graph_lines"] = [hits[0]["n.start_line"], hits[0]["n.end_line"]]
        raw["snippet_lines"] = [s.get("start_line"), s.get("end_line")]
    if not hits:
        raw["successors"] = successor_candidates(project, loc, bound["digest"], bound.get("tail"))
    return raw


def classify_code(raw, bound):
    """Map raw facts to the five states. Returns (state, reason)."""
    cov = raw["coverage"]
    fresh = cov["freshness"]
    complete = cov["recording_complete"] and cov["hash_records_complete"]
    if fresh == "metadata_changed":
        return "unknown", "index_stale_for_file (observed subject != requested subject)"
    if fresh == "missing" and raw["hits"]:
        return "unknown", "index_stale: file removed on disk but still in graph"
    if fresh == "not_tracked" and raw["file_on_disk"]:
        return "unknown", "file_on_disk_but_not_indexed"
    if cov["issues"]:
        return "unknown", "coverage_issue_in_file: " + ",".join(i.get("kind", "?") for i in cov["issues"])
    if len(raw["hits"]) > 1:
        return "unknown", "locator_ambiguous"
    if len(raw["hits"]) == 1:
        if raw["observed_digest"] == bound["digest"]:
            return "matches", "same_entity_same_body"
        return "differs", "same_entity_body_changed"
    # no hit
    if not complete:
        return "unknown", "absent_but_coverage_recording_incomplete"
    if fresh in ("metadata_match", "missing"):
        return "differs", "absence_established" + ("" if fresh == "metadata_match" else " (file removed)")
    return "unknown", f"absent_with_freshness_{fresh}"


def observe_text_once(repo, anchor):
    loc = anchor["locator"]
    p = os.path.join(repo, loc["path"])
    if not os.path.exists(p):
        return {"file_on_disk": False}
    lines = open(p).read().split("\n")
    a, b = loc["start"], loc["end"]
    seg = "\n".join(lines[a - 1:b])
    raw = {"file_on_disk": True, "observed_digest": sha(seg)}
    if raw["observed_digest"] != loc["digest"]:
        # discovery: does the bound text now sit elsewhere in the file?
        n = b - a + 1
        raw["relocated_at"] = [i + 1 for i in range(len(lines) - n + 1)
                               if sha("\n".join(lines[i:i + n])) == loc["digest"]]
    return raw


def classify_text(raw, anchor):
    if not raw["file_on_disk"]:
        return "differs", "document_removed"
    if raw["observed_digest"] == anchor["locator"]["digest"]:
        return "matches", "same_text"
    return "differs", "text_changed" + (" (bound text relocated)" if raw.get("relocated_at") else "")


def observe(project, repo, canonical, anchor_id, hook=None):
    """One consistent observation, or an explicit inconsistency outcome.

    Consistency = same index generation signal AND same canonical revision at
    begin and end. `hook(attempt)` lets an experiment inject concurrent change.
    """
    attempts = []
    for attempt in range(MAX_RETRIES):
        try:
            rev0 = canonical.revision()
            data = canonical.load()
            anchor, bound = _find(data, anchor_id)
            kind = anchor["kind"]
            if kind not in SUPPORTED_KINDS:
                return _result(anchor_id, rev0, None, "unsupported", f"anchor_kind {kind}", {}, attempts)
            gen0 = coverage_facts(project, anchor["locator"]["path"])["generation"]
            if hook:
                hook(attempt)
            if kind == "code_structure":
                raw = observe_code_once(project, repo, anchor, bound)
                state, reason = classify_code(raw, bound)
            else:
                raw = observe_text_once(repo, anchor)
                state, reason = classify_text(raw, anchor)
            gen1 = coverage_facts(project, anchor["locator"]["path"])["generation"]
            rev1 = canonical.revision()
        except cbm.CbmError as e:
            return _result(anchor_id, None, None, "failed", f"tool_error: {e}", {}, attempts)
        consistent = gen0 == gen1 and rev0 == rev1
        attempts.append({"gen": [gen0, gen1], "rev": [rev0, rev1], "consistent": consistent})
        if consistent:
            return _result(anchor_id, rev0, gen0, state, reason, raw, attempts)
    return _result(anchor_id, None, None, "unknown", "no_consistent_snapshot_after_retries", raw, attempts)


def _find(data, anchor_id):
    for c in data["claims"].values():
        for a in c["anchors"]:
            if a["id"] == anchor_id:
                return a, a.get("bound")
    raise KeyError(anchor_id)


def _result(anchor_id, rev, gen, state, reason, raw, attempts):
    assert state in STATES
    return {"anchor": anchor_id, "canonical_revision": rev, "index_generation": gen,
            "state": state, "reason": reason, "raw": raw, "attempts": attempts}


# ── declaration (binding) ─────────────────────────────────────────────────
def declare_code_anchor(project, anchor):
    """Bind at declaration time. Refuses to bind anything but exactly one entity."""
    hits = resolve_code(project, anchor["locator"])
    if len(hits) != 1:
        return None, f"refused: {len(hits)} exact matches"
    qn = hits[0]["n.qualified_name"]
    d, s = body_digest(project, qn)
    cov = coverage_facts(project, anchor["locator"]["path"])
    anchor["bound"] = {"qn": qn, "digest": d, "tail": tail_digest(s), "generation": cov["generation"]}
    return anchor, "bound"


def declare_text_anchor(repo, anchor):
    loc = anchor["locator"]
    lines = open(os.path.join(repo, loc["path"])).read().split("\n")
    loc["digest"] = sha("\n".join(lines[loc["start"] - 1:loc["end"]]))
    anchor["bound"] = {"digest": loc["digest"]}
    return anchor, "bound"
