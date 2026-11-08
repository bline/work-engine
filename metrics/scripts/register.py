#!/usr/bin/env python3
"""Research register: validate, build the current-knowledge view, nominate impact.

Standard library only. No network and no model calls. Writes only to --out.
Records what studies observed and what they support, kept apart from present
applicability and from authority to change decisions. See metrics/register/README.md.

    python3 -I metrics/scripts/register.py validate [--register DIR] [--repo DIR] [--check-offrepo]
    python3 -I metrics/scripts/register.py build --out DIR [--register DIR] [--repo DIR]
    python3 -I metrics/scripts/register.py nominate --out FILE [--register DIR] [--repo DIR]
        [--claims-host PATH --startup PATH --startup-sha256 HEX]
    python3 -I metrics/scripts/register.py anchors --out DIR [--register DIR] [--repo DIR] [--at COMMIT]
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys

SCHEMA_VIEW = "register_current_knowledge_v1"
SCHEMA_NOMINATIONS = "register_impact_nominations_v1"

QUESTION_KEYS = {"id", "text", "area"}
STUDY_KEYS = {"id", "title", "kind", "questions", "lifecycle", "stop_reason_class", "findings", "sources", "aliases"}
FINDING_KEYS = {"id", "claim", "value", "n", "basis", "scope", "caveats", "sources"}
LIFECYCLE_KEYS = {"date", "state", "basis"}
EVENT_KEYS = {"event_id", "kind", "date", "subject_finding", "by_finding", "basis", "source"}
JUDGMENT_KEYS = {"finding", "axis", "value", "basis", "assessor", "date"}
DECISION_KEYS = {"decision_id", "document", "anchor", "revision", "owner", "findings"}
DECISION_OPTIONAL = {"lines"}
APPL_KEYS = {"applicability_id", "finding", "decision_id", "decision_revision", "relation", "rationale",
             "evidence", "proposed_by", "proposed_at", "authorization"}
AUTH_KEYS = {"status", "owner"}
AUTH_OPTIONAL = {"at"}
MATERIALIZED_KEYS = {"finding", "label", "claim_id", "revision_id", "revision_sha256"}
PIN_KEYS = {"path", "commit", "blob"}
PIN_OPTIONAL = {"lines"}
OFFREPO_KEYS = {"location", "sha256", "custody", "status"}
OFFREPO_OPTIONAL = {"note"}

STUDY_KINDS = {"probe", "pilot", "controlled-experiment", "retrospective-analysis", "qualification",
               "instrument", "baseline", "planned-only"}
LIFECYCLE_STATES = {"planned", "preregistered", "running", "concluded", "partial", "stopped", "abandoned",
                    "superseded"}
STOP_CLASSES = {"budget_or_stop_rule", "gate_failed", "direction_changed", "quota", "premise_false",
                "scope_misread"}
FINDING_BASES = {"measured", "modelled", "estimated", "single-observation", "owner-report", "not-established"}
EVENT_KINDS = {"correction", "challenge", "status_discrepancy"}
AXES = {
    "claim_kind": {"observation", "measurement", "comparison", "explanation", "recommendation"},
    "evidence_strength": {"high", "moderate", "low", "very-low"},
    "standing": {"standing", "qualified", "corrected", "withdrawn"},
    "currency": {"current", "drifting", "historical", "not-assessed"},
}
RELATIONS = {"supports", "constrains", "challenges", "does_not_apply_directly", "not_established"}
AUTH_STATUSES = {"pending", "authorized", "declined"}
OFFREPO_STATUSES = {"verified", "unverified", "unavailable"}

KEBAB = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
FINDING_ID = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*#[a-z0-9]+(?:-[a-z0-9]+)*$")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
HEX_COMMIT = re.compile(r"^[0-9a-f]{7,40}$")
HEX_BLOB = re.compile(r"^[0-9a-f]{40}$")
HEX_SHA256 = re.compile(r"^[0-9a-f]{64}$")
REVISION = re.compile(r"^git:([0-9a-f]{7,40})$")
GLOBAL_FIELD = re.compile(r"useful|obsolet", re.I)


def dumps(obj):
    return json.dumps(obj, sort_keys=True, indent=1, ensure_ascii=False) + "\n"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


class Errors:
    def __init__(self):
        self.items = []

    def add(self, name, location, message):
        self.items.append((name, location, message))

    def __bool__(self):
        return bool(self.items)

    def names(self):
        return [n for n, _, _ in self.items]


class Git:
    def __init__(self, repo):
        self.repo = repo
        self.cache = {}

    def run(self, *args):
        key = args
        if key not in self.cache:
            p = subprocess.run(["git", "-C", self.repo, *args], capture_output=True, text=True, check=False)
            self.cache[key] = p.stdout.strip() if p.returncode == 0 else None
        return self.cache[key]

    def blob(self, commit, path):
        return self.run("rev-parse", "--verify", "-q", f"{commit}:{path}")

    def object_type(self, commit, path):
        return self.run("cat-file", "-t", f"{commit}:{path}")

    def line_count(self, commit, path):
        key = ("line_count", commit, path)
        if key not in self.cache:
            p = subprocess.run(["git", "-C", self.repo, "cat-file", "blob", f"{commit}:{path}"],
                               capture_output=True, check=False)
            if p.returncode != 0:
                self.cache[key] = None
            else:
                data = p.stdout
                self.cache[key] = data.count(b"\n") + (1 if data and not data.endswith(b"\n") else 0)
        return self.cache[key]

    def commit_exists(self, commit):
        return self.run("rev-parse", "--verify", "-q", f"{commit}^{{commit}}") is not None


class Register:
    """Loaded register data. Parse problems are collected in .errors."""

    def __init__(self, root, errors):
        self.root = root
        self.errors = errors
        self.questions = self._json("questions.json", [])
        self.studies = []
        sdir = os.path.join(root, "studies")
        if os.path.isdir(sdir):
            for name in sorted(os.listdir(sdir)):
                if name.endswith(".json"):
                    doc = self._json(os.path.join("studies", name), None)
                    if doc is not None:
                        self.studies.append((name, doc))
        else:
            errors.add("missing-file", "studies/", "directory not found")
        self.events = self._jsonl("events.jsonl")
        self.judgments = self._jsonl("judgments.jsonl")
        self.decisions = self._json("decisions.json", [])
        self.applicability = self._jsonl("applicability.jsonl")
        self.materialized = self._jsonl("materialized.jsonl")

    def _read(self, rel):
        path = os.path.join(self.root, rel)
        try:
            with open(path, encoding="utf-8") as fh:
                return fh.read()
        except OSError as exc:
            self.errors.add("missing-file", rel, str(exc))
            return None

    def _json(self, rel, default):
        text = self._read(rel)
        if text is None:
            return default
        try:
            return json.loads(text)
        except ValueError as exc:
            self.errors.add("bad-json", rel, str(exc))
            return default

    def _jsonl(self, rel):
        text = self._read(rel)
        out = []
        for i, line in enumerate(text.splitlines() if text else [], 1):
            if not line.strip():
                continue
            try:
                out.append((f"{rel}:{i}", json.loads(line)))
            except ValueError as exc:
                self.errors.add("bad-json", f"{rel}:{i}", str(exc))
        return out

    # accessors over structurally usable records
    def findings(self):
        out = {}
        for _, study in self.studies:
            if isinstance(study, dict):
                for f in study.get("findings") or []:
                    if isinstance(f, dict) and isinstance(f.get("id"), str):
                        out.setdefault(f["id"], (study, f))
        return out


def check_keys(errors, loc, rec, required, optional=frozenset()):
    """Closed key set. Returns True when the record is a dict with all required keys."""
    if not isinstance(rec, dict):
        errors.add("bad-value", loc, "record is not an object")
        return False
    for k in sorted(rec):
        if GLOBAL_FIELD.search(k):
            errors.add("forbidden-global-field", loc, f"key {k!r}: usefulness or obsolescence is never recorded globally")
        elif k not in required and k not in optional:
            errors.add("unknown-key", loc, f"key {k!r} is not in the closed key set")
    ok = True
    for k in sorted(required):
        if k not in rec:
            errors.add("missing-key", loc, f"required key {k!r} is absent")
            ok = False
    return ok


def is_text(v):
    return isinstance(v, str) and v.strip() != ""


def has(collection, value):
    """Membership that never raises: a non-string value is never a member."""
    return isinstance(value, str) and value in collection


def check_enum(errors, loc, key, value, allowed):
    if not has(allowed, value):
        errors.add("bad-value", loc, f"{key} {value!r} not in {sorted(allowed)}")


LINES = re.compile(r"^(\d+)(?:-(\d+))?$")


def check_lines(errors, git, loc, lines, commit, path):
    """`lines` is N or N-M, 1-based, and must lie within the file at commit:path."""
    m = LINES.match(lines) if isinstance(lines, str) else None
    if not m:
        errors.add("bad-value", loc, f"lines {lines!r} is not N or N-M")
        return
    lo = int(m.group(1))
    hi = int(m.group(2)) if m.group(2) else lo
    if lo < 1 or hi < lo:
        errors.add("bad-value", loc, f"lines {lines!r} is not an ascending range from 1")
        return
    count = git.line_count(commit, path)
    if count is not None and hi > count:
        errors.add("pin-lines-out-of-range", loc, f"lines {lines} exceed the {count} lines of {commit}:{path}")


def check_date(errors, loc, key, value):
    if not (isinstance(value, str) and DATE.match(value)):
        errors.add("bad-value", loc, f"{key} {value!r} is not YYYY-MM-DD")


def check_source(errors, git, loc, src):
    if not isinstance(src, dict):
        errors.add("bad-source", loc, "source is not an object")
        return
    if "location" in src:
        if not check_keys(errors, loc, src, OFFREPO_KEYS, OFFREPO_OPTIONAL):
            return
        if not is_text(src["location"]):
            errors.add("offrepo-malformed", loc, "location must be non-empty text")
        sha = src["sha256"]
        if sha is not None and not (isinstance(sha, str) and HEX_SHA256.match(sha)):
            errors.add("offrepo-malformed", loc, "sha256 must be 64 lowercase hex or null")
        if src["custody"] != "off_repo":
            errors.add("offrepo-malformed", loc, "custody must be 'off_repo'")
        if not has(OFFREPO_STATUSES, src["status"]):
            errors.add("offrepo-malformed", loc, f"status {src['status']!r} not in {sorted(OFFREPO_STATUSES)}")
        elif src["status"] == "verified" and sha is None:
            errors.add("offrepo-malformed", loc, "status verified requires a sha256")
        return
    if not check_keys(errors, loc, src, PIN_KEYS, PIN_OPTIONAL):
        return
    commit, path, blob = src["commit"], src["path"], src["blob"]
    if not (isinstance(commit, str) and HEX_COMMIT.match(commit)):
        errors.add("bad-value", loc, f"commit {commit!r} is not 7-40 lowercase hex")
        return
    if not (isinstance(blob, str) and HEX_BLOB.match(blob)):
        errors.add("bad-value", loc, f"blob {blob!r} is not 40 lowercase hex")
        return
    if not is_text(path):
        errors.add("bad-value", loc, "path must be non-empty text")
        return
    actual = git.blob(commit, path)
    if actual is None:
        errors.add("pin-unresolved", loc, f"git rev-parse {commit}:{path} does not resolve")
    elif actual != blob:
        errors.add("pin-blob-mismatch", loc, f"{commit}:{path} is blob {actual}, register says {blob}")
    elif git.object_type(commit, path) != "blob":
        errors.add("pin-not-blob", loc, f"{commit}:{path} is a {git.object_type(commit, path)}, not a blob")
    elif "lines" in src:
        check_lines(errors, git, loc, src["lines"], commit, path)


def check_sources(errors, git, loc, sources, nonempty=False):
    if not isinstance(sources, list):
        errors.add("bad-value", loc, "sources must be a list")
        return
    if nonempty and not sources:
        errors.add("bad-value", loc, "sources must not be empty")
    for i, s in enumerate(sources):
        check_source(errors, git, f"{loc}[{i}]", s)


def validate_register(reg, git):
    e = reg.errors
    # questions
    qids = set()
    if not isinstance(reg.questions, list):
        e.add("bad-value", "questions.json", "must be a list")
    else:
        for i, q in enumerate(reg.questions):
            loc = f"questions.json[{i}]"
            if check_keys(e, loc, q, QUESTION_KEYS):
                if not (isinstance(q["id"], str) and re.match(r"^Q\d+$", q["id"])):
                    e.add("bad-value", loc, f"id {q['id']!r} is not Q<number>")
                elif q["id"] in qids:
                    e.add("duplicate-id", loc, f"question id {q['id']}")
                else:
                    qids.add(q["id"])
                for k in ("text", "area"):
                    if not is_text(q[k]):
                        e.add("bad-value", loc, f"{k} must be non-empty text")
    # studies and findings
    fids, aliases, study_ids = set(), {}, set()
    for fname, study in reg.studies:
        loc = f"studies/{fname}"
        if not check_keys(e, loc, study, STUDY_KEYS):
            continue
        sid = study["id"]
        if not (isinstance(sid, str) and KEBAB.match(sid)):
            e.add("bad-value", loc, f"study id {sid!r} is not kebab-case")
            continue
        if fname != sid + ".json":
            e.add("study-filename-mismatch", loc, f"file name must be {sid}.json")
        if sid in study_ids:
            e.add("duplicate-id", loc, f"study id {sid}")
        study_ids.add(sid)
        check_enum(e, loc, "kind", study["kind"], STUDY_KINDS)
        if study["stop_reason_class"] is not None:
            check_enum(e, loc, "stop_reason_class", study["stop_reason_class"], STOP_CLASSES)
        if not is_text(study["title"]):
            e.add("bad-value", loc, "title must be non-empty text")
        if not isinstance(study["questions"], list):
            e.add("bad-value", loc, "questions must be a list")
        else:
            for q in study["questions"]:
                if not has(qids, q):
                    e.add("unknown-question-reference", loc, f"question {q!r} is not in questions.json")
        if not isinstance(study["lifecycle"], list) or not study["lifecycle"]:
            e.add("bad-value", loc, "lifecycle must be a non-empty list")
        else:
            for i, lc in enumerate(study["lifecycle"]):
                lloc = f"{loc}.lifecycle[{i}]"
                if check_keys(e, lloc, lc, LIFECYCLE_KEYS):
                    check_date(e, lloc, "date", lc["date"])
                    check_enum(e, lloc, "state", lc["state"], LIFECYCLE_STATES)
                    check_source(e, git, lloc + ".basis", lc["basis"])
        check_sources(e, git, loc + ".sources", study["sources"])
        if not isinstance(study["aliases"], list) or not all(is_text(a) for a in study["aliases"]):
            e.add("bad-value", loc, "aliases must be a list of text")
        else:
            for a in study["aliases"]:
                if a in aliases:
                    e.add("duplicate-id", loc, f"alias {a} also on study {aliases[a]}")
                aliases[a] = sid
        if not isinstance(study["findings"], list) or not study["findings"]:
            e.add("bad-value", loc, "findings must be a non-empty list")
            continue
        for i, f in enumerate(study["findings"]):
            floc = f"{loc}.findings[{i}]"
            if not check_keys(e, floc, f, FINDING_KEYS):
                continue
            fid = f["id"]
            if not (isinstance(fid, str) and FINDING_ID.match(fid)):
                e.add("bad-value", floc, f"finding id {fid!r} is not <study-id>#<slug>")
                continue
            if fid.split("#")[0] != sid:
                e.add("bad-value", floc, f"finding id {fid} does not start with study id {sid}#")
            if fid in fids:
                e.add("duplicate-id", floc, f"finding id {fid}")
            fids.add(fid)
            check_enum(e, floc, "basis", f["basis"], FINDING_BASES)
            for k in ("claim", "scope"):
                if not is_text(f[k]):
                    e.add("bad-value", floc, f"{k} must be non-empty text")
            if not (f["n"] is None or (isinstance(f["n"], (int, str)) and not isinstance(f["n"], bool))):
                e.add("bad-value", floc, "n must be an integer, text or null")
            if not isinstance(f["caveats"], list) or not all(is_text(c) for c in f["caveats"]):
                e.add("bad-value", floc, "caveats must be a list of text")
            check_sources(e, git, floc + ".sources", f["sources"], nonempty=True)

    # decisions
    dids, drev = set(), {}
    if not isinstance(reg.decisions, list):
        e.add("bad-value", "decisions.json", "must be a list")
    else:
        for i, d in enumerate(reg.decisions):
            loc = f"decisions.json[{i}]"
            if not check_keys(e, loc, d, DECISION_KEYS, DECISION_OPTIONAL):
                continue
            did = d["decision_id"]
            if not (isinstance(did, str) and KEBAB.match(did.replace(".", "-"))):
                e.add("bad-value", loc, f"decision_id {did!r} is not kebab-case with optional dots")
            elif did in dids:
                e.add("duplicate-id", loc, f"decision id {did}")
            else:
                dids.add(did)
                drev[did] = d["revision"]
            for k in ("document", "anchor", "owner"):
                if not is_text(d[k]):
                    e.add("bad-value", loc, f"{k} must be non-empty text")
            m = REVISION.match(d["revision"]) if isinstance(d["revision"], str) else None
            if not m:
                e.add("bad-value", loc, f"revision {d['revision']!r} must be a string of the form git:<commit>")
            elif not git.commit_exists(m.group(1)):
                e.add("pin-unresolved", loc, f"revision commit {m.group(1)} does not resolve")
            elif is_text(d["document"]) and git.blob(m.group(1), d["document"]) is None:
                e.add("pin-unresolved", loc, f"{m.group(1)}:{d['document']} does not resolve")
            elif "lines" in d and is_text(d["document"]):
                check_lines(e, git, loc + ".lines", d["lines"], m.group(1), d["document"])
            elif "lines" in d:
                e.add("bad-value", loc, "lines needs a document")
            if not isinstance(d["findings"], list) or not d["findings"]:
                e.add("bad-value", loc, "findings must be a non-empty list")
            else:
                for fid in d["findings"]:
                    if not has(fids, fid):
                        e.add("unknown-finding-reference", loc, f"finding {fid!r} is not in any study")

    # events
    eids = set()
    for loc, ev in reg.events:
        if not check_keys(e, loc, ev, EVENT_KEYS):
            continue
        if not (isinstance(ev["event_id"], str) and KEBAB.match(ev["event_id"])):
            e.add("bad-value", loc, f"event_id {ev['event_id']!r} is not kebab-case")
        elif ev["event_id"] in eids:
            e.add("duplicate-id", loc, f"event id {ev['event_id']}")
        else:
            eids.add(ev["event_id"])
        check_enum(e, loc, "kind", ev["kind"], EVENT_KINDS)
        check_date(e, loc, "date", ev["date"])
        if not has(fids, ev["subject_finding"]):
            e.add("unknown-finding-reference", loc, f"subject_finding {ev['subject_finding']!r} is not in any study")
        if ev["by_finding"] is not None and not has(fids, ev["by_finding"]):
            e.add("unknown-finding-reference", loc, f"by_finding {ev['by_finding']!r} is not in any study")
        if not is_text(ev["basis"]):
            e.add("bad-value", loc, "basis must be non-empty text")
        check_source(e, git, loc + ".source", ev["source"])

    # judgments
    for loc, j in reg.judgments:
        if not check_keys(e, loc, j, JUDGMENT_KEYS):
            continue
        if not has(fids, j["finding"]):
            e.add("unknown-finding-reference", loc, f"finding {j['finding']!r} is not in any study")
        if not has(AXES, j["axis"]):
            e.add("bad-value", loc, f"axis {j['axis']!r} not in {sorted(AXES)}")
        else:
            check_enum(e, loc, "value", j["value"], AXES[j["axis"]])
        check_date(e, loc, "date", j["date"])
        for k in ("basis", "assessor"):
            if not is_text(j[k]):
                e.add("bad-value", loc, f"{k} must be non-empty text")

    # applicability
    aids = set()
    for loc, a in reg.applicability:
        rev_present = True
        if isinstance(a, dict) and a.get("decision_revision") is None:
            e.add("missing-decision-revision", loc, "applicability is per (finding, decision@revision); "
                  "decision_revision is absent or null")
            a = {**a, "decision_revision": ""}
            rev_present = False
        auth = a.get("authorization") if isinstance(a, dict) else None
        if isinstance(a, dict) and (not isinstance(auth, dict) or "status" not in auth):
            e.add("missing-authorization-status", loc, "authorization.status is required")
            a = {**a, "authorization": {"status": "pending", "owner": (auth or {}).get("owner", "x") if isinstance(auth, dict) else "x"}}
        if not check_keys(e, loc, a, APPL_KEYS):
            continue
        if not (isinstance(a["applicability_id"], str) and KEBAB.match(a["applicability_id"])):
            e.add("bad-value", loc, f"applicability_id {a['applicability_id']!r} is not kebab-case")
        elif a["applicability_id"] in aids:
            e.add("duplicate-id", loc, f"applicability id {a['applicability_id']}")
        else:
            aids.add(a["applicability_id"])
        if not has(fids, a["finding"]):
            e.add("unknown-finding-reference", loc, f"finding {a['finding']!r} is not in any study")
        rev_valid = isinstance(a["decision_revision"], str) and bool(REVISION.match(a["decision_revision"]))
        if rev_present and not rev_valid:
            e.add("bad-value", loc, f"decision_revision {a['decision_revision']!r} must be a string of the form git:<commit>")
        if not has(dids, a["decision_id"]):
            e.add("unknown-decision-reference", loc, f"decision {a['decision_id']!r} is not in decisions.json")
        elif rev_valid and a["decision_revision"] != drev[a["decision_id"]]:
            e.add("decision-revision-mismatch", loc,
                  f"decision_revision {a['decision_revision']!r} differs from decisions.json {drev[a['decision_id']]!r}")
        check_enum(e, loc, "relation", a["relation"], RELATIONS)
        for k in ("rationale", "proposed_by"):
            if not is_text(a[k]):
                e.add("bad-value", loc, f"{k} must be non-empty text")
        check_date(e, loc, "proposed_at", a["proposed_at"])
        check_sources(e, git, loc + ".evidence", a["evidence"], nonempty=True)
        au = a["authorization"]
        if check_keys(e, loc + ".authorization", au, AUTH_KEYS, AUTH_OPTIONAL):
            check_enum(e, loc + ".authorization", "status", au["status"], AUTH_STATUSES)
            if not is_text(au["owner"]):
                e.add("bad-value", loc + ".authorization", "owner must be non-empty text")
            if au["status"] != "pending" and "at" not in au:
                e.add("bad-value", loc + ".authorization", "a decided authorization needs 'at'")

    # materialized
    for loc, m in reg.materialized:
        if check_keys(e, loc, m, MATERIALIZED_KEYS - {"label"}, {"label"}):
            if not has(fids, m["finding"]):
                e.add("unknown-finding-reference", loc, f"finding {m['finding']!r} is not in any study")
    return fids


def iter_offrepo(reg):
    """Yield (location-in-register, source) for every well-formed off-repo source."""
    def walk(loc, src):
        if isinstance(src, dict) and "location" in src:
            yield loc, src
    for fname, study in reg.studies:
        if not isinstance(study, dict):
            continue
        for i, s in enumerate(study.get("sources") or []):
            yield from walk(f"studies/{fname}.sources[{i}]", s)
        for i, lc in enumerate(study.get("lifecycle") or []):
            if isinstance(lc, dict):
                yield from walk(f"studies/{fname}.lifecycle[{i}].basis", lc.get("basis"))
        for f in study.get("findings") or []:
            if isinstance(f, dict):
                for i, s in enumerate(f.get("sources") or []):
                    yield from walk(f"studies/{fname}:{f.get('id')}.sources[{i}]", s)
    for loc, ev in reg.events:
        if isinstance(ev, dict):
            yield from walk(loc + ".source", ev.get("source"))
    for loc, a in reg.applicability:
        if isinstance(a, dict):
            for i, s in enumerate(a.get("evidence") or []):
                yield from walk(f"{loc}.evidence[{i}]", s)


def check_offrepo(reg):
    """Return (mismatches, report lines). Absence is reported, never a violation."""
    mismatches, report, seen = [], [], set()
    for loc, src in iter_offrepo(reg):
        path = os.path.expanduser(src["location"])
        key = (src["location"], src["sha256"])
        if key in seen:
            continue
        seen.add(key)
        if not os.path.exists(path):
            report.append(f"ABSENT {src['location']} (status {src['status']}): not found on this host")
        elif not os.path.isfile(path):
            report.append(f"SKIPPED {src['location']}: not a regular file")
        elif src["sha256"] is None:
            report.append(f"UNHASHED {src['location']}: present, no digest recorded (status {src['status']})")
        else:
            actual = sha256_file(path)
            if actual == src["sha256"]:
                report.append(f"MATCH {src['location']} sha256 {actual[:12]}")
            else:
                mismatches.append(("offrepo-digest-mismatch", loc,
                                   f"{src['location']} is sha256 {actual}, register says {src['sha256']}"))
                report.append(f"MISMATCH {src['location']}")
    return mismatches, report


def load_valid(register_dir, repo, extra=None):
    errors = Errors()
    reg = Register(register_dir, errors)
    git = Git(repo)
    fids = validate_register(reg, git)
    return reg, errors, fids


def cmd_validate(args):
    reg, errors, fids = load_valid(args.register, args.repo)
    report = []
    if args.check_offrepo:
        mism, report = check_offrepo(reg)
        for m in mism:
            errors.add(*m)
    for name, loc, msg in errors.items:
        print(f"ERROR {name} {loc}: {msg}")
    for line in report:
        print(line)
    if errors:
        print(f"validate: FAILED with {len(errors.items)} violation(s)")
        return 1
    print(f"validate: ok ({len(reg.studies)} studies, {len(fids)} findings, {len(reg.events)} events, "
          f"{len(reg.judgments)} judgments, {len(reg.decisions)} decisions, "
          f"{len(reg.applicability)} applicability records)")
    return 0


def k_sort(alias):
    m = re.match(r"^([A-Za-z]+)(\d+)$", alias)
    return (m.group(1), int(m.group(2)), alias) if m else (alias, 0, alias)


def derive(reg):
    """The pure derivation shared by build and nominate."""
    findings = reg.findings()
    events = sorted((ev for _, ev in reg.events), key=lambda x: x["event_id"])
    decisions = sorted(reg.decisions, key=lambda d: d["decision_id"])
    dec_by_id = {d["decision_id"]: d for d in decisions}
    appl = sorted((a for _, a in reg.applicability), key=lambda a: a["applicability_id"])
    latest = {}
    for _, j in reg.judgments:  # file order breaks date ties; the later line wins
        cur = latest.get((j["finding"], j["axis"]))
        if cur is None or j["date"] >= cur["date"]:
            latest[(j["finding"], j["axis"])] = j
    out = {}
    for fid in sorted(findings):
        study, f = findings[fid]
        subj = [ev for ev in events if ev["subject_finding"] == fid]
        kinds = {ev["kind"] for ev in subj}
        if {"correction", "challenge"} <= kinds:
            derived = "corrected_and_challenged"
        elif "correction" in kinds:
            derived = "corrected"
        elif "challenge" in kinds:
            derived = "challenged"
        else:
            derived = "none"
        judg = {}
        for axis in sorted(AXES):
            j = latest.get((fid, axis))
            judg[axis] = ({k: j[k] for k in ("value", "basis", "assessor", "date")} if j
                          else {"value": "not established", "basis": "no judgment recorded"})
        lifecycle = sorted(study["lifecycle"], key=lambda lc: lc["date"])
        out[fid] = {
            "study": study["id"],
            "study_title": study["title"],
            "aliases": sorted(study["aliases"], key=k_sort),
            "claim": f["claim"],
            "value": f["value"],
            "n": f["n"],
            "basis": f["basis"],
            "scope": f["scope"],
            "caveats": f["caveats"],
            "lifecycle": {"latest": lifecycle[-1]["state"] if lifecycle else "not established",
                          "history": [{"date": lc["date"], "state": lc["state"]} for lc in lifecycle]},
            "judgments": judg,
            "standing_from_events": derived,
            "status_discrepancy": any(ev["kind"] == "status_discrepancy" for ev in subj),
            "events": sorted(ev["event_id"] for ev in events
                             if fid in (ev["subject_finding"], ev["by_finding"])),
            "applicability": [
                {"applicability_id": a["applicability_id"], "decision_id": a["decision_id"],
                 "decision_revision": a["decision_revision"],
                 "document": dec_by_id[a["decision_id"]]["document"],
                 "anchor": dec_by_id[a["decision_id"]]["anchor"],
                 "relation": a["relation"], "authorization": a["authorization"]["status"]}
                for a in appl if a["finding"] == fid],
            "listed_in_decisions": sorted(d["decision_id"] for d in decisions if fid in d["findings"]),
        }
    return findings, events, decisions, appl, out


def inside(path, directory):
    """True when path is the directory or lies under it (symlinks resolved)."""
    path, directory = os.path.realpath(path), os.path.realpath(directory)
    return os.path.commonpath([path, directory]) == directory


def refuse_out_in_register(cmd, out, register):
    if inside(out, register):
        print(f"{cmd}: refused, --out {out} is inside the register directory {register}")
        return True
    return False


def cmd_build(args):
    if refuse_out_in_register("build", args.out, args.register):
        return 2
    reg, errors, _ = load_valid(args.register, args.repo)
    if errors:
        for name, loc, msg in errors.items:
            print(f"ERROR {name} {loc}: {msg}")
        print("build: refused, the register does not validate")
        return 1
    findings, events, decisions, appl, per = derive(reg)
    k_index = {}
    for fid, row in per.items():
        for alias in row["aliases"]:
            k_index.setdefault(alias, {"study": row["study"], "findings": []})["findings"].append(fid)
    questions = {}
    for q in sorted(reg.questions, key=lambda q: int(q["id"][1:])):
        questions[q["id"]] = {"text": q["text"], "area": q["area"], "findings": []}
    for _, study in reg.studies:
        for q in study["questions"]:
            questions[q]["findings"].extend(f["id"] for f in study["findings"])
    for q in questions.values():
        q["findings"] = sorted(set(q["findings"]))
    doc = {"schema": SCHEMA_VIEW,
           "authority": "generated view; edit the register, never this file; decision owners decide",
           "k_index": {k: k_index[k] for k in sorted(k_index, key=k_sort)},
           "findings": per, "questions": questions}
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "current-knowledge.json"), "w", encoding="utf-8") as fh:
        fh.write(dumps(doc))
    with open(os.path.join(args.out, "current-knowledge.md"), "w", encoding="utf-8") as fh:
        fh.write(render_md(doc))
    print(f"build: wrote current-knowledge.json and current-knowledge.md to {args.out}")
    return 0


def render_md(doc):
    L = ["# Current knowledge (generated)", "",
         "Generated from the register by `metrics/scripts/register.py build`. Do not edit. "
         "Standing and currency are assessments, not decisions; applicability is per (finding, decision@revision) "
         "and is advisory until its owner authorizes it.", "",
         "## Findings", ""]
    entries = [(alias, fid) for alias, ent in doc["k_index"].items() for fid in ent["findings"]]
    aliased = {fid for _, fid in entries}
    entries += [("no alias", fid) for fid in sorted(doc["findings"]) if fid not in aliased]
    for alias, fid in entries:
        r = doc["findings"][fid]
        j = r["judgments"]
        L += [f"### {alias}: `{fid}`", "",
              f"- Study: {r['study_title']} (lifecycle: {r['lifecycle']['latest']})",
              f"- Claim: {r['claim']}",
              f"- Basis: {r['basis']}; n = {r['n'] if r['n'] is not None else 'not established'}",
              f"- Claim kind: {j['claim_kind']['value']}; evidence strength: {j['evidence_strength']['value']}",
              f"- Standing (judged): {j['standing']['value']}; standing from events: {r['standing_from_events']}"
              + ("; status discrepancy recorded" if r["status_discrepancy"] else ""),
              f"- Currency: {j['currency']['value']}",
              f"- Events: {', '.join(r['events']) or 'none'}"]
        if r["applicability"]:
            L.append("- Applicability:")
            for a in r["applicability"]:
                L.append(f"  - {a['relation']} | {a['document']} ({a['anchor']}) at {a['decision_revision']} | "
                         f"authorization: {a['authorization']} | {a['applicability_id']}")
        else:
            L.append("- Applicability: none recorded")
        L.append("")
    L += ["## Questions", ""]
    for qid, q in doc["questions"].items():
        L.append(f"- {qid} ({q['area']}): {q['text']} Findings: {', '.join(q['findings']) or 'none linked'}")
    return "\n".join(L) + "\n"


def resolve_claim(host, startup, startup_sha, ref):
    req = json.dumps({"action": "resolve", "claim_id": ref["claim_id"], "revision_id": ref["revision_id"],
                      "revision_sha256": ref["revision_sha256"]})
    try:
        p = subprocess.run([host, "read", startup, startup_sha], input=req, capture_output=True, text=True,
                           check=False, timeout=60)
    except (OSError, subprocess.SubprocessError) as exc:
        return {"error": str(exc)[:300]}
    if p.returncode != 0:
        return {"error": p.stderr.strip()[:300]}
    try:
        doc = json.loads(p.stdout)
        proj = json.loads(doc["canonical_projection"]) if "canonical_projection" in doc else doc
        item = proj["items"][0]
        return {"current_head": item["current_head"], "head_revision_ids": item["head_revision_ids"],
                "reliances": sorted(
                    ({"consumer": r["record"]["consumer"], "consumer_revision": r["record"]["consumer_revision"],
                      "state": r["record"]["state"]} for r in item["reliances"]),
                    key=lambda r: (r["consumer"], r["consumer_revision"]))}
    except (ValueError, KeyError, IndexError, TypeError) as exc:
        return {"error": f"unreadable claims host output: {exc}"}


def cmd_nominate(args):
    claims = (args.claims_host, args.startup, args.startup_sha256)
    if any(claims) and not all(claims):
        print("nominate: --claims-host, --startup and --startup-sha256 go together")
        return 2
    if refuse_out_in_register("nominate", args.out, args.register):
        return 2
    reg, errors, _ = load_valid(args.register, args.repo)
    if errors:
        for name, loc, msg in errors.items:
            print(f"ERROR {name} {loc}: {msg}")
        print("nominate: refused, the register does not validate")
        return 1
    findings, events, decisions, appl, per = derive(reg)
    mats = [m for _, m in reg.materialized]
    noms = []
    for ev in events:
        subj, by = ev["subject_finding"], ev["by_finding"]
        reached = {}
        for d in decisions:
            listed = [f for f in (subj, by) if f is not None and f in d["findings"]]
            if listed:
                via = ("both" if len(set(listed)) == 2 else
                       "subject_finding" if listed[0] == subj else "by_finding")
                reached[d["decision_id"]] = {k: d[k] for k in ("decision_id", "document", "anchor", "revision", "owner")
                                             } | ({"lines": d["lines"]} if "lines" in d else {}) | {
                                                 "via": via, "listed_findings": sorted(set(listed))}
        recs = [{"applicability_id": a["applicability_id"], "finding": a["finding"], "decision_id": a["decision_id"],
                 "decision_revision": a["decision_revision"], "relation": a["relation"],
                 "authorization": a["authorization"]["status"]}
                for a in appl if a["finding"] in (subj, by)]
        refs = [m for m in mats if m["finding"] in (subj, by)]
        if all(claims):
            claim_rev = {"status": "queried", "revisions": [
                {"finding": m["finding"], "revision_id": m["revision_id"], "revision_sha256": m["revision_sha256"],
                 **resolve_claim(args.claims_host, args.startup, args.startup_sha256, m)}
                for m in sorted(refs, key=lambda m: m["revision_id"])]}
        else:
            claim_rev = {"status": "not_queried",
                         "registered_revisions": [{"finding": m["finding"], "revision_id": m["revision_id"]}
                                                  for m in sorted(refs, key=lambda m: m["revision_id"])]}
        not_est = ["whether the subject finding is still supported for any consumer",
                   "whether any decision should change",
                   "whether any applicability record has been authorized by its owner"
                   if recs else "whether applicability to any decision has been assessed"]
        if claim_rev["status"] == "not_queried":
            not_est.append("claim revisions and reliances (claims host not queried)")
        noms.append({"event_id": ev["event_id"], "kind": ev["kind"], "date": ev["date"],
                     "subject_finding": subj, "by_finding": by, "basis": ev["basis"], "source": ev["source"],
                     "decisions": [reached[k] for k in sorted(reached)],
                     "applicability_records": sorted(recs, key=lambda r: r["applicability_id"]),
                     "claim_revisions": claim_rev,
                     "disposition": "review_nominated", "not_established": not_est})
    doc = {"schema": SCHEMA_NOMINATIONS,
           "authority": "nomination only; decision owners decide; nothing here changes a claim, reliance, decision or record",
           "completeness": {"scope": "events.jsonl and the decisions that list the subject or by_finding in decisions.json",
                            "events": len(events), "claims_queried": all(claims)},
           "nominations": noms}
    parent = os.path.dirname(os.path.abspath(args.out))
    if not os.path.isdir(parent):
        print(f"nominate: output directory {parent} does not exist")
        return 2
    with open(args.out, "w", encoding="utf-8") as fh:
        fh.write(dumps(doc))
    print(f"nominate: {len(noms)} event nomination(s) written to {args.out}")
    return 0


# ---------------------------------------------------------------------------
# anchors: read-only qualification of decision anchors against a pinned commit
# ---------------------------------------------------------------------------

SCHEMA_ANCHORS = "register_anchor_report_v1"
ANCHOR_CLASSES = ("unchanged", "moved", "modified", "relocated_verbatim", "ambiguous", "absent")
# Exit codes of `anchors` (0 ok; 2 is shared with argument errors and the refusals: out inside the register or a git directory, output path a symlink).
EXIT_ANCHOR_DECISION_ERRORS = 3  # outputs written; at least one decision (or register line) has an error entry
EXIT_ANCHOR_CANNOT_RUN = 4       # target or register records unusable, or writing to --out failed
CONTEXT_LINES = 3
FULL_COMMIT = re.compile(r"^[0-9a-f]{40}$")
HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", re.M)
MD_HEADING = re.compile(r"^ {0,3}#{1,6}(?:[ \t]|$)")
MD_EXT = (".md", ".markdown")
SKILL_NOTE = ("skill document: observation is repository source at the pinned commit; "
              "the installed skill release was not examined (comparing the two is a later question, O7)")
HISTORICAL_PINS = {
    "model-selection.claude-host-builder-rows-at-df288c3d":
        "deliberate historical pin (named in the register README prose, no field marks it); "
        "reported with its class like any other decision",
}
AFTER_STATES = ("carried_unchanged", "owner_review_required", "pending_judgment_required", "not_established")


class AnchorGit:
    """Read-only git access. Every call that is interpreted carries explicit options and the process
    environment is rebuilt, so the result depends neither on the caller's GIT_* variables (GIT_DIR,
    GIT_DIFF_OPTS, GIT_CONFIG_*, ...) nor on replace refs nor on system, global or repository
    configuration that changes diff, rename or object reading. It never refreshes the index."""

    DIFF = ["diff", "--no-ext-diff", "--no-textconv", "--no-color", "--text", "--minimal", "--no-renames",
            "--no-indent-heuristic", "--inter-hunk-context=0", "-U0"]
    # configuration keys that change diff or rename output, pinned on the command line
    CONFIG = ("diff.algorithm=myers", "diff.renames=false", "diff.context=0",
              "diff.interHunkContext=0", "diff.indentHeuristic=false", "diff.noprefix=false",
              "diff.mnemonicPrefix=false", "diff.relative=false",
              "core.quotepath=false", "core.autocrlf=false", "core.safecrlf=false")

    def __init__(self, repo):
        self.repo = repo
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        self.env.update({"GIT_OPTIONAL_LOCKS": "0", "GIT_CONFIG_GLOBAL": os.devnull,
                         "GIT_CONFIG_SYSTEM": os.devnull, "GIT_CONFIG_NOSYSTEM": "1",
                         "GIT_NO_REPLACE_OBJECTS": "1"})
        self.cache = {}

    def argv(self, *args):
        pins = [x for kv in self.CONFIG for x in ("-c", kv)]
        return ["git", "--no-optional-locks", "--no-replace-objects", *pins, "-C", self.repo, *args]

    def run(self, *args):
        if args not in self.cache:
            p = subprocess.run(self.argv(*args), capture_output=True, check=False, env=self.env)
            self.cache[args] = p.stdout if p.returncode == 0 else None
        return self.cache[args]

    def commit_id(self, rev):
        """A commit named by a ref, tag or expression (used for --at)."""
        if not isinstance(rev, str) or not rev or rev.startswith("-"):
            return None
        out = self.run("rev-parse", "--verify", "-q", f"{rev}^{{commit}}")
        text = out.decode("utf-8", "replace").strip() if out else ""
        return text if FULL_COMMIT.match(text) else None

    def commit_by_hex(self, hexid):
        """The one commit object whose id starts with hexid; refs are never consulted."""
        if not isinstance(hexid, str) or not HEX_COMMIT.match(hexid):
            return None
        out = self.run("rev-parse", f"--disambiguate={hexid}")
        found = []
        for cand in (out.decode("utf-8", "replace").split() if out else []):
            if FULL_COMMIT.match(cand):
                kind = self.run("cat-file", "-t", cand)
                if kind is not None and kind.strip() == b"commit":
                    found.append(cand)
        return found[0] if len(found) == 1 else None

    def git_dirs(self):
        """Real paths of this repository's git directory and common git directory."""
        dirs = set()
        for flag in ("--absolute-git-dir", "--git-common-dir"):
            out = self.run("rev-parse", flag)
            text = out.decode("utf-8", "replace").strip() if out else ""
            if text:
                dirs.add(os.path.realpath(os.path.join(self.repo, text)))
        return sorted(dirs)

    def blob_id(self, commit, path):
        kind = self.run("cat-file", "-t", f"{commit}:{path}")
        if kind is None or kind.strip() != b"blob":
            return None
        out = self.run("rev-parse", "--verify", "-q", f"{commit}:{path}")
        return out.decode().strip() if out else None

    def blob(self, blob_id):
        return self.run("cat-file", "blob", blob_id)

    def hunks(self, a_blob, b_blob):
        out = self.run(*self.DIFF, a_blob, b_blob)
        if out is None:
            return None
        res = []
        for m in HUNK.finditer(out.decode("utf-8", "replace")):
            res.append((int(m.group(1)), 1 if m.group(2) is None else int(m.group(2)),
                        int(m.group(3)), 1 if m.group(4) is None else int(m.group(4))))
        return res

    def rename_of(self, old_commit, new_commit, path):
        """(new_path, status) when the diff old_commit..new_commit, with rename detection, renames path."""
        out = self.run("diff", "--no-ext-diff", "--no-textconv", "--no-color", "-M50%", "-l0",
                       "--name-status", "--diff-filter=R", "-z", old_commit, new_commit)
        if not out:
            return None
        parts = out.decode("utf-8", "replace").split("\0")
        i = 0
        while i + 2 < len(parts) and parts[i]:
            if parts[i].startswith("R") and parts[i + 1] == path:
                return parts[i + 2], parts[i]
            i += 3
        return None

    def is_ancestor(self, a, b):
        key = ("is-ancestor", a, b)
        if key not in self.cache:
            p = subprocess.run(self.argv("merge-base", "--is-ancestor", a, b),
                               capture_output=True, check=False, env=self.env)
            self.cache[key] = p.returncode == 0 if p.returncode in (0, 1) else None
        return self.cache[key]


def split_lines(data):
    parts = data.split(b"\n")
    if parts and parts[-1] == b"":
        parts.pop()
    return parts


def text_of(lines):
    return [ln.decode("utf-8", "replace") for ln in lines]


def fmt_lines(lo, hi):
    return str(lo) if lo == hi else f"{lo}-{hi}"


def parse_lines(lines):
    m = LINES.match(lines) if isinstance(lines, str) else None
    if not m:
        return None
    lo = int(m.group(1))
    hi = int(m.group(2)) if m.group(2) else lo
    return (lo, hi) if 1 <= lo <= hi else None


def anchored_sha(lines):
    return hashlib.sha256(b"\n".join(lines) + b"\n").hexdigest()


def find_occurrences(target, anchor):
    k = len(anchor)
    return [i + 1 for i in range(len(target) - k + 1) if target[i] == anchor[0] and target[i:i + k] == anchor]


def normalise_hunks(hunks, old_lines, new_lines):
    """Drop lines a hunk marks as changed although their text is equal on both sides (git reports the
    last line of a file as changed when only its final newline was added or removed). Hunks keep the
    -U0 convention: for a pure insertion `a` is the old line after which it goes, for a pure deletion
    `c` is the new line after which it was removed."""
    out = []
    for a, b, c, d in hunks:
        if b == 0 or d == 0:
            out.append((a, b, c, d))
            continue
        i = 0
        while i < b and i < d and old_lines[a - 1 + i] == new_lines[c - 1 + i]:
            i += 1
        j = 0
        while j < b - i and j < d - i and old_lines[a + b - 2 - j] == new_lines[c + d - 2 - j]:
            j += 1
        nb, nd = b - i - j, d - i - j
        if nb == 0 and nd == 0:
            continue
        if nb == 0:
            out.append((a + i - 1, 0, c + i, nd))
        elif nd == 0:
            out.append((a + i, nb, c + i - 1, 0))
        else:
            out.append((a + i, nb, c + i, nd))
    return out


def track(hunks, lo, hi):
    """Follow old lines lo..hi through the hunks of a -U0 diff, each line on its own.
    Returns (touched, new_lo, span): touched is true when any anchored line lies in a changed hunk or
    something was inserted between anchored lines; new_lo is where line lo would sit if the range
    were untouched; span is (first, last) new line over the anchored lines that survive and the
    replacement lines of changed hunks, or None when every anchored line was deleted outright."""
    touched = any(a >= lo and a < hi for a, b, _c, _d in hunks if b == 0)
    nlo = lo + sum(d - b if b > 0 and a + b - 1 < lo else d if b == 0 and a < lo else 0 for a, b, _c, d in hunks)
    nums = []
    for line in range(lo, hi + 1):
        off, inside = 0, None
        for a, b, c, d in hunks:
            if b > 0:
                if a + b - 1 < line:
                    off += d - b
                elif a <= line:
                    inside = (c, d)
            elif a < line:
                off += d
        if inside is None:
            nums.append(line + off)
        else:
            touched = True
            nums.extend(range(inside[0], inside[0] + inside[1]))
    return touched, nlo, ((min(nums), max(nums)) if nums else None)


FENCE_OPEN = re.compile(r"^\s*(`{3,}|~{3,})(.*)$")


def markdown_chain(lines, lineno):
    """The chain of Markdown headings that enclose line `lineno` (1-based, a heading on that line
    counts), outermost first, each {"level", "line", "text"}; fenced code is excluded. A fence opened
    by backticks or tildes closes only on the same character, at least as long, with nothing else on
    the line. Setext headings are not recognised."""
    chain, fence = [], None
    for i, raw in enumerate(lines[:lineno], 1):
        text = raw.decode("utf-8", "replace")
        if fence is not None:
            m = FENCE_OPEN.match(text)
            if m and m.group(1)[0] == fence[0] and len(m.group(1)) >= fence[1] and not m.group(2).strip():
                fence = None
            continue
        m = FENCE_OPEN.match(text)
        if m and not (m.group(1)[0] == "`" and "`" in m.group(2)):
            fence = (m.group(1)[0], len(m.group(1)))
            continue
        if MD_HEADING.match(text):
            level = len(text.lstrip()) - len(text.lstrip().lstrip("#"))
            while chain and chain[-1]["level"] >= level:
                chain.pop()
            chain.append({"level": level, "line": i, "text": text.strip()})
    return chain


def section_identity(chain):
    return [(h["level"], h["text"]) for h in chain]


def surroundings(lines, lo, hi, is_md):
    chain = markdown_chain(lines, lo) if is_md else None
    return {"lines": fmt_lines(lo, hi),
            "heading": (chain[-1] if chain else None) if is_md else "not applicable (not a Markdown document)",
            "section_chain": chain if is_md else "not applicable (not a Markdown document)",
            "before": text_of(lines[max(0, lo - 1 - CONTEXT_LINES):lo - 1]),
            "after": text_of(lines[hi:hi + CONTEXT_LINES])}


def anchor_error(entry, kind, reason):
    entry["class"] = None
    entry["error"] = {"kind": kind, "reason": reason}
    return entry


def blank_entry(d):
    return {"decision_id": d.get("decision_id"), "document": d.get("document"), "anchor": d.get("anchor"),
            "revision": d.get("revision"), "lines": d.get("lines"), "owner": d.get("owner"),
            "class": None, "error": None, "owner_review_required": False, "owner_review_reasons": [],
            "document_at_target": None, "candidates": [], "before": None, "after": None,
            "context_before": None, "context_after": None, "notes": []}


def classify_decision(git, d, target):
    """One entry for decision d against the pinned target commit. Never raises on bad data."""
    entry = blank_entry(d)
    doc = d.get("document")
    if not (isinstance(entry["decision_id"], str) and entry["decision_id"] and isinstance(doc, str) and doc.strip()):
        return anchor_error(entry, "malformed_decision", "decision_id and document must be non-empty text")
    m = REVISION.match(d["revision"]) if isinstance(d.get("revision"), str) else None
    if not m:
        return anchor_error(entry, "revision_unresolved", f"revision {d.get('revision')!r} is not of the form git:<commit>")
    rev = git.commit_by_hex(m.group(1))
    if rev is None:
        return anchor_error(entry, "revision_unresolved", f"revision commit {m.group(1)} does not resolve to one commit")
    entry["revision_commit"] = rev
    entry["revision_is_ancestor_of_target"] = git.is_ancestor(rev, target)
    rev_blob = git.blob_id(rev, doc)
    if rev_blob is None:
        return anchor_error(entry, "document_missing_at_revision", f"{doc} does not exist at {rev}")
    rev_lines = split_lines(git.blob(rev_blob))
    whole = "lines" not in d
    span = None
    if not whole:
        span = parse_lines(d["lines"])
        if span is None:
            return anchor_error(entry, "lines_invalid", f"lines {d['lines']!r} is not an ascending N or N-M from 1")
        if span[1] > len(rev_lines):
            return anchor_error(entry, "lines_out_of_range",
                                f"lines {d['lines']} exceed the {len(rev_lines)} lines of {doc} at {rev}")
    if doc.startswith("skills/"):
        entry["notes"].append(SKILL_NOTE)
    if entry["decision_id"] in HISTORICAL_PINS:
        entry["notes"].append(HISTORICAL_PINS[entry["decision_id"]])
    # the document as it exists at the target, following a rename
    tpath, tblob, rename = doc, git.blob_id(target, doc), None
    if tblob is None:
        found = git.rename_of(rev, target, doc)
        if found:
            tpath, rename = found
            tblob = git.blob_id(target, tpath)
    if tblob is not None:
        entry["document_at_target"] = {"path": tpath, "renamed": rename is not None,
                                       "rename_status": rename, "blob": tblob}
    before = {"revision": d["revision"], "revision_commit": rev, "document_blob": rev_blob}
    if whole:
        entry["before"] = before
        if tblob is None:
            entry["class"] = "absent"
        else:
            entry["class"] = "unchanged" if tblob == rev_blob else "modified"
            entry["after"] = {"document_blob": tblob}
            if entry["class"] == "modified":
                entry["candidates"] = [{"document": tpath, "basis": "whole document differs; candidate only"}]
        return entry
    lo, hi = span
    anchor = rev_lines[lo - 1:hi]
    before.update({"lines": fmt_lines(lo, hi), "anchored_text_sha256": anchored_sha(anchor)})
    entry["before"] = before
    if tblob is None:
        entry["class"] = "absent"
        return entry
    t_lines = split_lines(git.blob(tblob))
    hunks = git.hunks(rev_blob, tblob)
    if hunks is None:
        return anchor_error(entry, "internal", "git diff between the two blobs failed")
    hunks = normalise_hunks(hunks, rev_lines, t_lines)
    is_md = doc.lower().endswith(MD_EXT) and tpath.lower().endswith(MD_EXT)
    occ = find_occurrences(t_lines, anchor)
    touched, nlo, span_new = track(hunks, lo, hi)
    n = hi - lo + 1
    if len(occ) > 1:
        entry["class"] = "ambiguous"
        entry["candidates"] = [{"lines": fmt_lines(o, o + n - 1), "basis": "exact anchored text occurs here"} for o in occ]
        return entry
    if len(occ) == 1:
        o = occ[0]
        entry["after"] = {"lines": fmt_lines(o, o + n - 1)}
        if not touched and o == nlo and o == lo:  # hunks are normalised, so a final newline alone is no change
            entry["class"] = "unchanged"
            return entry
        entry["class"] = "moved" if (not touched and o == nlo) else "relocated_verbatim"
        entry["context_before"] = surroundings(rev_lines, lo, hi, is_md)
        entry["context_after"] = surroundings(t_lines, o, o + n - 1, is_md)
        if entry["class"] == "relocated_verbatim":
            entry["owner_review_required"] = True
            entry["owner_review_reasons"].append("relocated_verbatim")
        elif is_md:
            cb, ca = entry["context_before"]["section_chain"], entry["context_after"]["section_chain"]
            if section_identity(cb) != section_identity(ca):
                entry["owner_review_required"] = True
                entry["owner_review_reasons"].append("section_identity_changed")
        return entry
    if span_new is not None:
        entry["class"] = "modified"
        entry["candidates"] = [{"lines": fmt_lines(span_new[0], min(span_new[1], len(t_lines))),
                                "basis": "line tracking through the diff; candidate only, not chosen"}]
    else:
        entry["class"] = "absent"
    return entry


def read_register_at(git, commit, register_dir, repo):
    """Records read from the pinned commit. Returns (decisions, applicability by decision, inputs, problems)."""
    rel = os.path.relpath(os.path.realpath(register_dir), os.path.realpath(repo))
    if rel == ".." or rel.startswith(".." + os.sep) or os.path.isabs(rel):
        raise ValueError(f"register directory {register_dir} is outside the repository {repo}; "
                         "anchors reads the register from the pinned commit")
    rel = rel.replace(os.sep, "/")
    inputs, problems = {}, []

    def read(name):
        path = f"{rel}/{name}"
        data = git.run("cat-file", "blob", f"{commit}:{path}")
        if data is not None:
            inputs[name] = {"path": path, "sha256": hashlib.sha256(data).hexdigest()}
        return path, data

    path, data = read("decisions.json")
    if data is None:
        raise ValueError(f"{path} not found at {commit}")
    try:
        decisions = json.loads(data.decode("utf-8"))
    except ValueError as exc:
        raise ValueError(f"{path} at {commit} is not valid JSON: {exc}")
    if not isinstance(decisions, list):
        raise ValueError(f"{path} at {commit} is not a list")
    appl = {}
    path, data = read("applicability.jsonl")
    if data is None:
        problems.append({"where": path, "reason": f"not found at {commit}"})
    else:
        for i, line in enumerate(data.decode("utf-8", "replace").splitlines(), 1):
            if not line.strip():
                continue
            try:
                rec = json.loads(line)
            except ValueError as exc:
                problems.append({"where": f"{path}:{i}", "reason": f"not valid JSON: {exc}"})
                continue
            if isinstance(rec, dict) and isinstance(rec.get("decision_id"), str):
                appl.setdefault(rec["decision_id"], []).append(
                    (i, rec, hashlib.sha256(line.encode("utf-8")).hexdigest()))
            else:
                problems.append({"where": f"{path}:{i}", "reason": "record has no decision_id"})
    return decisions, appl, inputs, problems


def applicability_after(entry, records):
    out = []
    cls = entry["class"]
    if cls is None:  # an errored decision: nothing was classified and nothing is transferred
        state = "not_established"
    elif cls == "moved" and not entry["owner_review_required"]:
        state = "carried_unchanged"
    elif cls == "relocated_verbatim" or (cls == "moved" and entry["owner_review_required"]):
        state = "owner_review_required"
    else:
        state = "pending_judgment_required"
    for line, rec, line_sha in sorted(records, key=lambda t: (str(t[1].get("applicability_id")), t[0])):
        auth = rec.get("authorization")
        authorized = isinstance(auth, dict) and auth.get("status") == "authorized"
        row = {"source": f"applicability.jsonl:{line}", "source_line_sha256": line_sha, "record": rec,
               "after_state": state,
               "authorization_after": None,
               "record_revision_matches_decision": rec.get("decision_revision") == entry["revision"]}
        if authorized:
            row["authorization_after"] = "carried_unchanged" if state == "carried_unchanged" \
                else "authorized_at_prior_revision"
        out.append(row)
    return out


def build_anchor_report(git, commit, at, register_dir, repo):
    decisions, appl, inputs, problems = read_register_at(git, commit, register_dir, repo)
    entries = []
    for i, d in enumerate(decisions):
        if not isinstance(d, dict):
            e = anchor_error(blank_entry({"decision_id": f"decisions.json[{i}]"}), "malformed_decision",
                             "record is not an object")
        else:
            try:
                e = classify_decision(git, d, commit)
            except Exception as exc:  # the run never crashes on one decision
                e = anchor_error(blank_entry({**d, "decision_id": d.get("decision_id", f"decisions.json[{i}]")}),
                                 "internal", f"{type(exc).__name__}: {exc}")
        did = e["decision_id"]
        records = appl.get(did, []) if isinstance(did, str) else []
        e["applicability_records_citing"] = len(records)
        if e["class"] != "unchanged":
            e["applicability"] = applicability_after(e, records)
        e["_order"] = (str(did), i)
        entries.append(e)
    entries.sort(key=lambda e: e["_order"])
    for e in entries:
        del e["_order"]
    summary = {c: sum(1 for e in entries if e["class"] == c) for c in ANCHOR_CLASSES}
    summary["error"] = sum(1 for e in entries if e["class"] is None)
    report = {
        "schema": SCHEMA_ANCHORS,
        "authority": "read-only report; the register owner decides what any result means",
        "statement": "Nothing was re-anchored, no register record was changed and no authorization was transferred.",
        "provenance": "every observation is repository source at the pinned commit",
        "pinned_commit": commit, "target_requested": at,
        "register": {"directory": os.path.relpath(os.path.realpath(register_dir), os.path.realpath(repo)).replace(os.sep, "/"),
                     "read_from": "the pinned commit (not the working tree)", "inputs": inputs},
        "tool": {"register_py_sha256": sha256_file(os.path.abspath(__file__))},
        "summary": summary, "decisions": entries, "register_read_problems": problems,
    }
    return report


def ctx_md(c):
    chain = c["section_chain"]
    if isinstance(chain, list):
        head = " > ".join(f"{h['text']} (line {h['line']})" for h in chain) or "none before this line"
    else:
        head = chain
    rows = [f"  - lines {c['lines']}; section chain: {head}"]
    rows += [f"    - before: `{x[:160]}`" for x in c["before"]] + [f"    - after: `{x[:160]}`" for x in c["after"]]
    return rows


def render_anchors_md(rep):
    L = ["# Decision anchor report", "",
         f"Pinned commit: `{rep['pinned_commit']}` (requested `{rep['target_requested']}`).", "",
         rep["statement"], "", f"Provenance: {rep['provenance']}.", "",
         f"Register read from {rep['register']['read_from']}, directory `{rep['register']['directory']}`.", ""]
    for name, info in sorted(rep["register"]["inputs"].items()):
        L.append(f"- input `{info['path']}` sha256 `{info['sha256']}`")
    L += [f"- `register.py` sha256 `{rep['tool']['register_py_sha256']}`", "", "## Summary", ""]
    L += [f"- {k}: {v}" for k, v in sorted(rep["summary"].items())]
    L += ["", "## Decisions", "", "| decision | class | lines before | lines after | owner review | document at target |",
          "|---|---|---|---|---|---|"]
    for e in rep["decisions"]:
        before = (e["before"] or {}).get("lines", "whole document" if e["before"] else "")
        after = (e["after"] or {}).get("lines", "whole document" if e["after"] else "")
        cls = e["class"] or f"ERROR {e['error']['kind']}"
        dat = e["document_at_target"]
        L.append(f"| `{e['decision_id']}` | {cls} | {before} | {after} | "
                 f"{'yes' if e['owner_review_required'] else 'no'} | "
                 f"{(dat['path'] + (' (renamed)' if dat['renamed'] else '')) if dat else 'none'} |")
    skill = [e for e in rep["decisions"] if SKILL_NOTE in e["notes"]]
    if skill:
        L += ["", "## Skill documents", ""]
        L += [f"- `{e['decision_id']}` ({e['document']}): {SKILL_NOTE}" for e in skill]
    for e in rep["decisions"]:
        if e["class"] == "unchanged":
            continue
        L += ["", f"### `{e['decision_id']}`: {e['class'] or 'ERROR'}", ""]
        if e["error"]:
            L.append(f"- error `{e['error']['kind']}`: {e['error']['reason']}")
        else:
            L += decision_lines_md(e)
        L += applicability_lines_md(e)
    if rep["register_read_problems"]:
        L += ["", "## Register read problems", ""] + [f"- {p['where']}: {p['reason']}" for p in rep["register_read_problems"]]
    return "\n".join(L) + "\n"


def decision_lines_md(e):
    b = e["before"]
    L = [f"- before: revision `{b['revision']}`, lines {b.get('lines', 'whole document')}"
         + (f", anchored text sha256 `{b['anchored_text_sha256']}`" if "anchored_text_sha256" in b else "")]
    if e["after"]:
        L.append(f"- after: lines {e['after'].get('lines', 'whole document')}")
    for c in e["candidates"]:
        L.append(f"- candidate (not chosen): {c.get('lines', c.get('document'))}; {c['basis']}")
    if e["owner_review_required"]:
        L.append(f"- OWNER REVIEW REQUIRED: {', '.join(e['owner_review_reasons'])}")
    for note in e["notes"]:
        L.append(f"- note: {note}")
    if e["context_before"]:
        L += ["- at the revision:"] + ctx_md(e["context_before"]) + ["- at the target:"] + ctx_md(e["context_after"])
    return L


def applicability_lines_md(e):
    L = []
    for a in e.get("applicability", []):
        r = a["record"]
        auth = r.get("authorization")
        L.append(f"- applicability `{r.get('applicability_id')}` ({a['source']}): finding `{r.get('finding')}`, "
                 f"relation `{r.get('relation')}`, recorded authorization `{(auth or {}).get('status')}`; "
                 f"after-state `{a['after_state']}`"
                 + (f"; authorization `{a['authorization_after']}`" if a["authorization_after"] else ""))
    return L


def write_outputs(out, files):
    """Write {name: text} into the real directory `out`. Never writes through a symbolic link: an output
    path that is a link is refused (the caller checks first), and each file is created under a fresh
    name (O_EXCL, O_NOFOLLOW) and renamed onto its final name, which replaces a link instead of
    following it. Raises OSError."""
    temps = []
    try:
        for name, text in sorted(files.items()):
            tmp = os.path.join(out, f".{name}.{os.getpid()}.tmp")
            fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o644)
            temps.append((tmp, os.path.join(out, name)))
            with os.fdopen(fd, "w", encoding="utf-8") as fh:
                fh.write(text)
        for tmp, final in temps:
            os.replace(tmp, final)
        temps = []
    finally:
        for tmp, _final in temps:
            try:
                os.unlink(tmp)
            except OSError:
                pass


def cmd_anchors(args):
    if refuse_out_in_register("anchors", args.out, args.register):
        return 2
    git = AnchorGit(args.repo)
    for gd in [os.path.join(args.repo, ".git"), *git.git_dirs()]:
        if inside(args.out, gd):
            print(f"anchors: refused, --out {args.out} is inside the repository's git directory {gd}")
            return 2
    for name in ("anchors.json", "anchors.md"):
        path = os.path.join(args.out, name)
        if os.path.islink(path):
            print(f"anchors: refused, output path {path} is a symbolic link; nothing was written")
            return 2
    commit = git.commit_id(args.at)
    if commit is None:
        print(f"anchors: cannot run, --at {args.at!r} does not resolve to a commit in {args.repo}")
        return EXIT_ANCHOR_CANNOT_RUN
    try:
        report = build_anchor_report(git, commit, args.at, args.register, args.repo)
    except ValueError as exc:
        print(f"anchors: cannot run, {exc}")
        return EXIT_ANCHOR_CANNOT_RUN
    try:
        os.makedirs(args.out, exist_ok=True)
        real_out = os.path.realpath(args.out)
        if refuse_out_in_register("anchors", real_out, args.register) or any(inside(real_out, gd) for gd in git.git_dirs()):
            print(f"anchors: refused, --out resolves to {real_out}, inside the register or a git directory")
            return 2
        write_outputs(real_out, {"anchors.json": dumps(report), "anchors.md": render_anchors_md(report)})
    except OSError as exc:
        print(f"anchors: cannot run, writing to --out failed: {exc}")
        return EXIT_ANCHOR_CANNOT_RUN
    s = report["summary"]
    print(f"anchors: wrote anchors.json and anchors.md to {args.out} (pinned {commit}; "
          + ", ".join(f"{k} {s[k]}" for k in (*ANCHOR_CLASSES, "error")) + ")")
    if s["error"] or report["register_read_problems"]:
        print("anchors: one or more decisions or register lines have an error entry")
        return EXIT_ANCHOR_DECISION_ERRORS
    return 0


def main(argv=None):
    here = os.path.dirname(os.path.abspath(__file__))
    default_repo = os.path.abspath(os.path.join(here, "..", ".."))
    ap = argparse.ArgumentParser(prog="register.py", description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)

    def common(p):
        p.add_argument("--repo", default=default_repo, help="git repository for source pins")
        p.add_argument("--register", default=None, help="register directory (default REPO/metrics/register)")

    v = sub.add_parser("validate")
    common(v)
    v.add_argument("--check-offrepo", action="store_true")
    b = sub.add_parser("build")
    common(b)
    b.add_argument("--out", required=True)
    n = sub.add_parser("nominate")
    common(n)
    n.add_argument("--out", required=True)
    n.add_argument("--claims-host")
    n.add_argument("--startup")
    n.add_argument("--startup-sha256")
    a = sub.add_parser("anchors")
    common(a)
    a.add_argument("--out", required=True)
    a.add_argument("--at", default="HEAD", help="commit to check against (default HEAD), resolved once")
    args = ap.parse_args(argv)
    args.repo = os.path.abspath(args.repo)
    if args.register is None:
        args.register = os.path.join(args.repo, "metrics", "register")
    return {"validate": cmd_validate, "build": cmd_build, "nominate": cmd_nominate,
            "anchors": cmd_anchors}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
