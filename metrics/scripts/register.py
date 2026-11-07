#!/usr/bin/env python3
"""Research register: validate, build the current-knowledge view, nominate impact.

Standard library only. No network and no model calls. Writes only to --out.
Records what studies observed and what they support, kept apart from present
applicability and from authority to change decisions. See metrics/register/README.md.

    python3 -I metrics/scripts/register.py validate [--register DIR] [--repo DIR] [--check-offrepo]
    python3 -I metrics/scripts/register.py build --out DIR [--register DIR] [--repo DIR]
    python3 -I metrics/scripts/register.py nominate --out FILE [--register DIR] [--repo DIR]
        [--claims-host PATH --startup PATH --startup-sha256 HEX]
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
    args = ap.parse_args(argv)
    args.repo = os.path.abspath(args.repo)
    if args.register is None:
        args.register = os.path.join(args.repo, "metrics", "register")
    return {"validate": cmd_validate, "build": cmd_build, "nominate": cmd_nominate}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
