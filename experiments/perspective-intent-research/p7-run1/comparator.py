#!/usr/bin/env python3
"""P7 run 1 comparator, version 3. Research instrumentation only; no model calls.

Input: one extraction (schema obligation-extract/v3) plus the task's documents.
Output: atomic flags and per-property records. The comparator never types prose
and never decides support from prose: every reading comes from the extraction,
which is what P7 measures. It is not the parked production comparator (no owner
authentication).

Decision table (PROTOCOL section 5 summarizes it):

  Properties assessed for a derived entry: every property it types as binding,
  selected_realization or ambiguous; for a complete or unchanged entry also every
  property its designated baseline parent entries bind (enumerated from the
  baseline, independently of support_candidates). A complete or unchanged entry
  with no baseline gets baseline_missing (not established).

  Parent value of property P: the fields of P on the entry's candidates for P
  (support_candidates[P], plus the baseline for complete or unchanged entries).
    any candidate ambiguous on P            -> parent_ambiguous
    no candidate binds P                    -> parent_unspecified (no candidates at all: no_candidates)
    more than one binding candidate, not all in the baseline and P not marked
      candidate_alignment 'same_condition'  -> alignment_unresolved
    roles: intersection ('any' neutral), empty -> parents_conflict
    required items: union; independence: equal, else parents_conflict

  Comparison of a binding derived value D with parent value V:
    items: D - V added (stronger); V - D lost (weaker)
    roles: V - D narrowed (stronger); D - V widened (weaker); disjoint sets
      contradictory. 'any' in V and not in D: narrowed; 'any' in D only: widened.
    independence: required over not_required stronger; reverse weaker.
    'other' on either side: not_comparable.
    Partial scope: weaker components are 'entailed' (no flag). Complete scope or
    unchanged: every difference flags; a baseline property the entry leaves
    unspecified is dropped.
  Derived selected_realization against a binding parent (outside a
  realization-choice entry): status_weakened. Derived binding with no binding
  parent: parent_unspecified, counted as a change (an addition without parent
  text). Derived ambiguous: not_assessed.

  Categories. change: stronger, weaker, dropped, contradictory, status_weakened,
  parent_unspecified. not_established: not_assessed, parent_ambiguous,
  parents_conflict, alignment_unresolved, not_comparable, no_candidates,
  baseline_missing. Entailment per property and per entry: changed if any change;
  else not_established if anything is not established; else supported (only
  equal or entailed components). An entry with no assessed property is
  not_assessed, never supported.

  Citation, per property the entry types: adequate (a specific reference to a
  parent document offered for that property), unresolved_target (specific, but
  not to a parent document), general_only, or none. Anything but adequate flags
  deficient_provenance for that property. Citation never changes entailment.

  Authority, per changed property: the owners are the semantics_owner names on
  the candidates that bind P (all candidates of P if none binds). Outcomes:
  not_established (no candidate, or a candidate without a binding owner);
  owner_decision_not_shown (owners known, adopted_by not stated); matched (every
  candidate's owner names meet the adopter's names); mismatched (otherwise).
  Flags: authority_not_established, owner_decision_not_shown,
  adopted_without_owner (mismatched). Names are compared after lower-casing and
  removing punctuation and a leading 'the'; aliases count only if the extractor
  lists them.

  unchanged_claim_false: an unchanged entry with any change.

  Proposals are compared and recorded, and raise no flags.

  Realization choices: binding fields inside a choice are compared as above; a
  selected_realization field must lie within the parent: roles a subset of the
  permitted set (any subset of 'any'); a complete choice keeps every required
  item; independence 'required' is not weakened. parent_unspecified with
  candidates is within (the parent leaves it open). no_candidates, ambiguous,
  conflicting, unaligned or 'other' comparisons are not_established, never
  conflicts. Standing: realization_conflicts, realization_not_established,
  realization_not_assessed (nothing compared) or realization.

Plan-time flags (plan tasks only), per parent entry:
  owner_unresolved: outcome_owner ambiguous, or unspecified on an entry that binds
    acceptance_evidence or path_fact ('any' records a deliberately open party);
  verifier_unassigned: binding acceptance_evidence, verifier unspecified or ambiguous;
  independence_unstated: binding acceptance_evidence, verifier_independence
    unspecified or ambiguous, whatever the verifier's status. It records that
    independence is unstated, not that it was required.

Quoting (quote-norm/v1): every span, reference text, owner span and other_text
must be one contiguous passage of its document after both sides are normalized:
curly quotes made straight, Markdown emphasis characters (* _ `) removed,
whitespace runs collapsed. Quote failures are collected separately from fatal
shape errors; QUOTE_POLICY decides whether they invalidate the extraction.
"""

import copy
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCHEMA_PATH = os.path.join(HERE, "schema", "obligation-extract-v3.json")
with open(SCHEMA_PATH) as f:
    SCHEMA = json.load(f)

PROPERTIES = SCHEMA["properties"]
STATUSES = set(SCHEMA["statuses"])
VALUES = {"roles": set(SCHEMA["roles"]), "evidence_items": set(SCHEMA["evidence_items"]),
          "path_items": set(SCHEMA["path_items"]), "independence": set(SCHEMA["independence_values"])}
ASSERTED_AS = {"inherited", "proposed", "new", "realization_choice", "unchanged"}
ITEM_KINDS = {"evidence_items", "path_items"}
QUOTE_NORM = "quote-norm/v1"
QUOTE_POLICY = "strict"  # frozen before counted runs; see PROTOCOL section 5

CHANGE = {"stronger", "weaker", "dropped", "contradictory", "status_weakened", "parent_unspecified"}
NOT_ESTABLISHED = {"not_assessed", "parent_ambiguous", "parents_conflict", "alignment_unresolved",
                   "not_comparable", "no_candidates", "baseline_missing"}


class ExtractionError(ValueError):
    """The extraction is invalid; the run's typed output is recorded as invalid."""

    def __init__(self, message, quote_failures=None):
        super().__init__(message)
        self.quote_failures = quote_failures or []


_QUOTES = {ord("\u2018"): "'", ord("\u2019"): "'", ord("\u201c"): '"', ord("\u201d"): '"'}


def qnorm(text):
    text = text.translate(_QUOTES)
    text = re.sub(r"[*_`]", "", text)
    return re.sub(r"\s+", " ", text).strip()


def quoted(quote, document):
    return isinstance(quote, str) and bool(qnorm(quote)) and qnorm(quote) in qnorm(document)


def _name(n):
    n = re.sub(r"[^a-z0-9 -]", "", n.lower()).strip()
    return re.sub(r"^the ", "", n).strip()


def _names(owner):
    if not owner or owner.get("status") != "binding":
        return set()
    return {x for x in (_name(n) for n in owner.get("names", [])) if x}


def _is_str_list(x):
    return isinstance(x, list) and all(isinstance(v, str) for v in x)


# ---------------------------------------------------------------- validation

class _Validator:
    def __init__(self, documents, parent_docs, derived_docs):
        self.documents, self.parent_docs, self.derived_docs = documents, parent_docs, derived_docs
        self.quote_failures = []
        self.all_text = qnorm("\n".join(documents.values())).lower()

    def quote(self, eid, where, text, doc_id):
        if doc_id not in self.documents or not quoted(text, self.documents[doc_id]):
            self.quote_failures.append({"entry": eid, "where": where, "document": doc_id,
                                        "text": text[:160] if isinstance(text, str) else repr(text)[:160]})

    def field(self, eid, doc, prop, field):
        if not isinstance(field, dict):
            raise ExtractionError(f"{eid}.{prop}: field must be an object")
        status = field.get("status")
        if not isinstance(status, str) or status not in STATUSES:
            raise ExtractionError(f"{eid}.{prop}: bad status {status!r}")
        value, readings = field.get("value", []), field.get("readings", [])
        if not _is_str_list(value) or not isinstance(readings, list) or not all(_is_str_list(r) for r in readings):
            raise ExtractionError(f"{eid}.{prop}: value and readings must be lists of strings")
        allowed = VALUES[PROPERTIES[prop]["value_kind"]]
        for group in [value] + readings:
            for v in group:
                if v not in allowed:
                    raise ExtractionError(f"{eid}.{prop}: value {v!r} outside enumeration")
            if len(set(group)) != len(group):
                raise ExtractionError(f"{eid}.{prop}: repeated token")
            if "any" in group and len(group) > 1:
                raise ExtractionError(f"{eid}.{prop}: 'any' combined with another role")
        if status in ("binding", "selected_realization") and (not value or readings):
            raise ExtractionError(f"{eid}.{prop}: {status} needs a value and no readings")
        if status == "unspecified" and (value or readings):
            raise ExtractionError(f"{eid}.{prop}: unspecified must have no value or readings")
        if status == "ambiguous":
            if value or len(readings) < 2 or any(not r for r in readings):
                raise ExtractionError(f"{eid}.{prop}: ambiguous needs no value and at least two non-empty readings")
            if len({tuple(sorted(r)) for r in readings}) != len(readings):
                raise ExtractionError(f"{eid}.{prop}: duplicate readings")
        if PROPERTIES[prop]["value_kind"] == "independence":
            for v in [value] + readings:
                if v and len(v) != 1:
                    raise ExtractionError(f"{eid}.{prop}: independence takes exactly one value")
        if "other" in value or any("other" in r for r in readings):
            other = field.get("other_text")
            if not isinstance(other, str) or not other.strip():
                raise ExtractionError(f"{eid}.{prop}: 'other' needs other_text")
            self.quote(eid, f"{prop}.other_text", other, doc)

    def owner(self, eid, key, owner):
        if not isinstance(owner, dict) or owner.get("status") not in STATUSES:
            raise ExtractionError(f"{eid}: bad {key}")
        names = owner.get("names", [])
        if not _is_str_list(names):
            raise ExtractionError(f"{eid}: {key}.names must be a list of strings")
        if owner["status"] == "binding":
            if not [n for n in names if n.strip()]:
                raise ExtractionError(f"{eid}: binding {key} needs names")
            doc, span = owner.get("document"), owner.get("span")
            if not isinstance(doc, str) or doc not in self.documents or not isinstance(span, str):
                raise ExtractionError(f"{eid}: binding {key} needs a document id and a span")
            self.quote(eid, f"{key}.span", span, doc)
            for n in names:
                if qnorm(n).lower() not in self.all_text:
                    self.quote_failures.append({"entry": eid, "where": f"{key}.names", "document": None, "text": n[:160]})


def validate(extraction, documents, parent_docs, derived_docs):
    """Shape validation (raises ExtractionError) and quote checks (collected).
    Returns (entries by id, quote failures)."""
    v = _Validator(documents, parent_docs, derived_docs)
    if not isinstance(extraction, dict) or extraction.get("schema") != SCHEMA["schema"]:
        raise ExtractionError("wrong or missing schema id")
    entries = extraction.get("entries")
    if not isinstance(entries, list):
        raise ExtractionError("entries must be a list")
    ids = {}
    for e in entries:
        if not isinstance(e, dict):
            raise ExtractionError("entry must be an object")
        for key in ("id", "document", "role", "span", "fields"):
            if key not in e:
                raise ExtractionError(f"entry missing {key}: {e.get('id')!r}")
        eid = e["id"]
        if not isinstance(eid, str) or not eid or eid in ids:
            raise ExtractionError(f"bad or duplicate id {eid!r}")
        ids[eid] = e
        if e["role"] not in ("parent", "derived"):
            raise ExtractionError(f"{eid}: bad role")
        if not isinstance(e["document"], str):
            raise ExtractionError(f"{eid}: bad document")
        if e["role"] == "parent" and e["document"] not in parent_docs:
            raise ExtractionError(f"{eid}: parent entry from non-parent document {e['document']!r}")
        if e["role"] == "derived" and e["document"] not in derived_docs:
            raise ExtractionError(f"{eid}: derived entry from non-derived document {e['document']!r}")
        if not isinstance(e["span"], str) or not qnorm(e["span"]):
            raise ExtractionError(f"{eid}: span must be non-empty text")
        v.quote(eid, "span", e["span"], e["document"])
        if not isinstance(e["fields"], dict):
            raise ExtractionError(f"{eid}: fields must be an object")
        for prop, field in e["fields"].items():
            if prop not in PROPERTIES:
                raise ExtractionError(f"{eid}: unknown property {prop!r}")
            v.field(eid, e["document"], prop, field)
        owner_key = "semantics_owner" if e["role"] == "parent" else "adopted_by"
        if e.get(owner_key) is not None:
            v.owner(eid, owner_key, e[owner_key])
        if e["role"] == "derived":
            if e.get("asserted_as") not in ASSERTED_AS:
                raise ExtractionError(f"{eid}: bad asserted_as")
            if e.get("scope") not in ("partial", "complete"):
                raise ExtractionError(f"{eid}: bad scope")
            refs = e.get("references_made")
            if not isinstance(refs, list):
                raise ExtractionError(f"{eid}: references_made must be a list")
            for r in refs:
                if (not isinstance(r, dict) or r.get("kind") not in ("specific", "general")
                        or not isinstance(r.get("text"), str) or not r["text"].strip()):
                    raise ExtractionError(f"{eid}: bad reference")
                target = r.get("target_document")
                if target is not None and target not in documents:
                    raise ExtractionError(f"{eid}: reference target {target!r} is not a task document")
                props = r.get("properties")
                if not _is_str_list(props) or any(p not in PROPERTIES for p in props):
                    raise ExtractionError(f"{eid}: reference properties must list schema properties")
                v.quote(eid, "references_made.text", r["text"], e["document"])
            cands = e.get("support_candidates")
            if not isinstance(cands, dict):
                raise ExtractionError(f"{eid}: support_candidates must be an object")
            if not _is_str_list(e.get("baseline", [])):
                raise ExtractionError(f"{eid}: baseline must be a list of entry ids")
            align = e.get("candidate_alignment", {})
            if not isinstance(align, dict) or any(p not in PROPERTIES or a != "same_condition" for p, a in align.items()):
                raise ExtractionError(f"{eid}: bad candidate_alignment")
    for e in entries:
        if e["role"] != "derived":
            continue
        for prop, cand in e["support_candidates"].items():
            if prop not in PROPERTIES or not _is_str_list(cand):
                raise ExtractionError(f"{e['id']}: bad support_candidates for {prop!r}")
            for pid in cand:
                if pid not in ids or ids[pid]["role"] != "parent":
                    raise ExtractionError(f"{e['id']}: support candidate {pid!r} is not a parent entry")
        for pid in e.get("baseline", []):
            if pid not in ids or ids[pid]["role"] != "parent":
                raise ExtractionError(f"{e['id']}: baseline {pid!r} is not a parent entry")
    return ids, v.quote_failures


# ---------------------------------------------------------------- comparison

def _field(entry, prop):
    return entry["fields"].get(prop, {"status": "unspecified", "value": []})


def _candidates(e, prop):
    out = list(e["support_candidates"].get(prop, []))
    if _is_complete(e):
        out += [b for b in e.get("baseline", []) if b not in out]
    return out


def _is_complete(e):
    return e["scope"] == "complete" or e["asserted_as"] == "unchanged"


def _parent_value(e, prop, ids):
    """Returns (state, value, binding candidate ids)."""
    cands = _candidates(e, prop)
    if not cands:
        return "no_candidates", None, []
    fields = [(pid, _field(ids[pid], prop)) for pid in cands]
    if any(f["status"] == "ambiguous" for _, f in fields):
        return "parent_ambiguous", None, []
    binding = [(pid, set(f["value"])) for pid, f in fields if f["status"] == "binding"]
    if not binding:
        return "parent_unspecified", None, []
    bids = [pid for pid, _ in binding]
    baseline = set(e.get("baseline", [])) if _is_complete(e) else set()
    aligned = (len(binding) == 1 or set(bids) <= baseline
               or e.get("candidate_alignment", {}).get(prop) == "same_condition")
    if not aligned:
        return "alignment_unresolved", None, bids
    sets = [s for _, s in binding]
    kind = PROPERTIES[prop]["value_kind"]
    if kind in ITEM_KINDS:
        return "ok", set().union(*sets), bids
    if kind == "independence":
        return ("ok", sets[0], bids) if all(s == sets[0] for s in sets) else ("parents_conflict", None, bids)
    specific = [s for s in sets if "any" not in s]
    if not specific:
        return "ok", {"any"}, bids
    combined = set.intersection(*specific)
    return ("ok", combined, bids) if combined else ("parents_conflict", None, bids)


def _relation(prop, d, v):
    """Returns (components, added values, lost values)."""
    kind = PROPERTIES[prop]["value_kind"]
    if "other" in d or "other" in v:
        return {"not_comparable"}, [], []
    added, lost = sorted(d - v), sorted(v - d)
    if kind in ITEM_KINDS:
        comps = ({"stronger"} if added else set()) | ({"weaker"} if lost else set())
        return comps or {"equal"}, added, lost
    if kind == "independence":
        if d == v:
            return {"equal"}, [], []
        return ({"stronger"} if d == {"required"} else {"weaker"}), added, lost
    if not added and not lost:
        return {"equal"}, [], []
    if "any" not in d and "any" not in v and not d & v:
        return {"contradictory"}, added, lost
    if "any" in v:
        return {"stronger"}, added, lost
    if "any" in d:
        return {"weaker"}, added, lost
    # roles: losing a permitted party narrows (stronger); adding one widens (weaker)
    return ({"stronger"} if lost else set()) | ({"weaker"} if added else set()), added, lost


def _choice_conflict(prop, d, v, complete):
    kind = PROPERTIES[prop]["value_kind"]
    if "other" in d or "other" in v:
        return "not_comparable"
    if kind in ITEM_KINDS:
        return "weaker" if complete and (v - d) else None
    if kind == "independence":
        return "weaker" if v == {"required"} and d != v else None
    if "any" in v:
        return None
    if "any" in d or not d <= v:
        return "contradictory" if "any" not in d and not d & v else "outside_permitted"
    return None


def _entailment(components):
    if components & CHANGE:
        return "changed"
    if components & NOT_ESTABLISHED:
        return "not_established"
    return "supported"


def _citation(e, prop, parent_docs):
    refs = [r for r in e["references_made"] if prop in r.get("properties", [])]
    if any(r["kind"] == "specific" and r.get("target_document") in parent_docs for r in refs):
        return "adequate"
    if any(r["kind"] == "specific" for r in refs):
        return "unresolved_target"
    return "general_only" if refs else "none"


def _authority(e, prop, ids):
    cands = _candidates(e, prop)
    relevant = [pid for pid in cands if _field(ids[pid], prop)["status"] == "binding"] or cands
    owners = [_names(ids[pid].get("semantics_owner")) for pid in relevant]
    adopter = _names(e.get("adopted_by"))
    rec = {"owners": sorted(set().union(*owners)) if owners else [], "adopter": sorted(adopter),
           "owner_candidates": relevant}
    if not relevant or any(not o for o in owners):
        rec["outcome"] = "not_established"
    elif not adopter:
        rec["outcome"] = "owner_decision_not_shown"
    elif all(o & adopter for o in owners):
        rec["outcome"] = "matched"
    else:
        rec["outcome"] = "mismatched"
    return rec


AUTHORITY_FLAGS = {"not_established": "authority_not_established", "owner_decision_not_shown": "owner_decision_not_shown",
                   "mismatched": "adopted_without_owner"}


def _assessed_props(e, ids, statuses):
    props = {p for p, f in e["fields"].items() if f["status"] in statuses}
    if _is_complete(e):
        for pid in e.get("baseline", []):
            props |= {p for p, f in ids[pid]["fields"].items() if f["status"] == "binding"}
    return props


def _compare_binding(e, p, ids, complete):
    """Normal comparison of one property. Returns a property record."""
    state, v, bids = _parent_value(e, p, ids)
    df = _field(e, p)
    rec = {"parent_state": state, "derived_status": df["status"], "derived_value": df["value"],
           "parent_value": sorted(v) if v is not None else None, "candidates": _candidates(e, p),
           "binding_candidates": bids, "added": [], "lost": []}
    if df["status"] == "ambiguous":
        comps = {"not_assessed"}
    elif df["status"] == "unspecified":
        comps = {"dropped"} if complete and state == "ok" else {"not_assessed"}
        if state == "ok":
            rec["lost"] = sorted(v)
    elif state in ("parent_unspecified", "no_candidates") and df["status"] == "binding":
        comps = {"parent_unspecified"}
        rec["added"] = df["value"]
    elif state == "parent_unspecified":
        comps = {"entailed"}  # a revisable choice where the parent leaves the property open
    elif state != "ok":
        comps = {state}
    else:
        comps, rec["added"], rec["lost"] = _relation(p, set(df["value"]), v)
        if df["status"] == "selected_realization":
            comps = (comps - {"equal"}) | {"status_weakened"}
        if not complete and "weaker" in comps:
            comps = (comps - {"weaker"}) | {"entailed"}
        if len(comps) > 1:
            comps.discard("equal")
    rec["relation"] = sorted(comps)
    rec["entailment"] = _entailment(comps)
    return rec


def derived_records(ids, parent_docs):
    flags, records = [], []
    for e in ids.values():
        if e["role"] != "derived":
            continue
        complete = _is_complete(e)
        choice = e["asserted_as"] == "realization_choice"
        props = _assessed_props(e, ids, ("binding", "selected_realization", "ambiguous"))
        prop_records, entry_flags = {}, []
        for p in sorted(props):
            df = _field(e, p)
            if choice and df["status"] in ("selected_realization", "unspecified"):
                rec = _compare_choice(e, p, ids, complete)
                if rec["conflict"]:
                    entry_flags.append({"property": p, "flag": "realization_conflicts", "detail": rec["conflict"],
                                        "category": "change"})
                elif rec["entailment"] == "not_established":
                    entry_flags.append({"property": p, "flag": rec["parent_state"] if rec["parent_state"] in NOT_ESTABLISHED else "not_comparable",
                                        "category": "not_established"})
            else:
                rec = _compare_binding(e, p, ids, complete)
                for comp in rec["relation"]:
                    if comp in CHANGE:
                        entry_flags.append({"property": p, "flag": comp, "category": "change",
                                            "added": rec["added"], "lost": rec["lost"]})
                    elif comp in NOT_ESTABLISHED:
                        entry_flags.append({"property": p, "flag": comp, "category": "not_established"})
            if df["status"] in ("binding", "selected_realization") and not (choice and df["status"] == "selected_realization"):
                rec["citation"] = _citation(e, p, parent_docs)
                if rec["citation"] != "adequate":
                    entry_flags.append({"property": p, "flag": "deficient_provenance", "detail": rec["citation"],
                                        "category": "citation"})
            if rec.get("entailment") == "changed" and not rec.get("conflict"):
                rec["authority"] = _authority(e, p, ids)
                af = AUTHORITY_FLAGS.get(rec["authority"]["outcome"])
                if af:
                    entry_flags.append({"property": p, "flag": af, "category": "authority",
                                        "owners": rec["authority"]["owners"], "adopter": rec["authority"]["adopter"]})
            prop_records[p] = rec
        if complete and not e.get("baseline"):
            entry_flags.append({"property": None, "flag": "baseline_missing", "category": "not_established"})
        ent = {r["entailment"] for r in prop_records.values()}
        if complete and not e.get("baseline"):
            ent.add("not_established")
        if "changed" in ent:
            entailment = "changed"
        elif "not_established" in ent:
            entailment = "not_established"
        elif prop_records:
            entailment = "supported"
        else:
            entailment = "not_assessed"
        changed_props = [p for p, r in prop_records.items() if r["entailment"] == "changed"]
        if e["asserted_as"] == "unchanged" and changed_props:
            entry_flags.append({"property": None, "flag": "unchanged_claim_false", "category": "change",
                                "detail": changed_props})
        citations = sorted({r["citation"] for r in prop_records.values() if "citation" in r})
        record = {"entry": e["id"], "document": e["document"], "asserted_as": e["asserted_as"],
                  "scope": e["scope"], "span": e["span"], "baseline": e.get("baseline", []),
                  "properties": prop_records, "references_made": e["references_made"],
                  "coverage": {"properties": len(prop_records),
                               "established": sum(1 for r in prop_records.values() if r["entailment"] != "not_established")},
                  "entailment": entailment, "citation": citations}
        if choice:
            conflicts = [f for f in entry_flags if f["category"] == "change"]
            if conflicts:
                record["standing"] = "realization_conflicts"
            elif not prop_records:
                record["standing"] = "realization_not_assessed"
            elif entailment == "not_established":
                record["standing"] = "realization_not_established"
            else:
                record["standing"] = "realization"
        elif e["asserted_as"] == "proposed":
            record["standing"] = "proposal"
        else:
            record["standing"] = entailment
        if e["asserted_as"] != "proposed":
            flags += [dict(f, entry=e["id"]) for f in entry_flags]
        record["flags"] = [f["flag"] for f in entry_flags]
        records.append(record)
    return flags, records


def _compare_choice(e, p, ids, complete):
    state, v, bids = _parent_value(e, p, ids)
    df = _field(e, p)
    rec = {"parent_state": state, "derived_status": df["status"], "derived_value": df["value"],
           "parent_value": sorted(v) if v is not None else None, "candidates": _candidates(e, p),
           "binding_candidates": bids, "conflict": None}
    if state == "parent_unspecified":
        rec["entailment"] = "within"
    elif state != "ok":
        rec["entailment"] = "not_established"
    elif df["status"] != "selected_realization":
        rec["conflict"] = "dropped" if complete else None
        rec["entailment"] = "changed" if complete else "within"
    else:
        result = _choice_conflict(p, set(df["value"]), v, complete)
        if result == "not_comparable":
            rec["parent_state"] = "not_comparable"
            rec["entailment"] = "not_established"
        else:
            rec["conflict"] = result
            rec["entailment"] = "changed" if result else "within"
    return rec


def plan_flags(ids):
    flags, extracted = [], 0
    for e in ids.values():
        if e["role"] != "parent":
            continue
        extracted += 1
        status = lambda p: _field(e, p)["status"]
        evidence = status("acceptance_evidence") == "binding"
        outcome_bearing = evidence or status("path_fact") == "binding"
        base = {"entry": e["id"], "document": e["document"], "span": e["span"]}
        if status("outcome_owner") == "ambiguous" or (status("outcome_owner") == "unspecified" and outcome_bearing):
            flags.append(dict(base, flag="owner_unresolved", owner_status=status("outcome_owner")))
        if evidence and status("verifier") in ("unspecified", "ambiguous"):
            flags.append(dict(base, flag="verifier_unassigned", verifier_status=status("verifier")))
        if evidence and status("verifier_independence") in ("unspecified", "ambiguous"):
            flags.append(dict(base, flag="independence_unstated", verifier_status=status("verifier"),
                              independence_status=status("verifier_independence")))
    return flags, extracted


# ---------------------------------------------------------------- fallback

def fallback_targets(extraction):
    """Fixed list of ambiguous property fields, computed from the original extraction."""
    return [{"entry": e["id"], "property": p, "readings": f["readings"], "span": e["span"]}
            for e in extraction["entries"] for p, f in e["fields"].items() if f["status"] == "ambiguous"]


def check_fallback_response(target, response):
    """Validate one fallback reply against its own indexed target. Returns an error string or None."""
    if not isinstance(response, dict):
        return "response is not an object"
    if response.get("entry") != target["entry"] or response.get("property") != target["property"]:
        return "response names a different target"
    if response.get("status") not in ("binding", "selected_realization", "ambiguous"):
        return "bad status"
    value = response.get("value", [])
    if not _is_str_list(value):
        return "value must be a list of strings"
    if response["status"] == "ambiguous" and value:
        return "ambiguous must have an empty value"
    if response["status"] != "ambiguous" and not value:
        return "a resolution needs a value"
    reason = response.get("reason")
    if not isinstance(reason, str) or not reason.strip():
        return "missing reason"
    return None


def apply_fallback(extraction, targets, outcomes):
    """outcomes[i] pairs with targets[i]: None (no reply recorded) or {"status": recorded status,
    "response": parsed reply, "error": ...}. Returns (new extraction, log). Every target gets one
    log row; an unanswered or invalid target keeps its readings."""
    if len(outcomes) != len(targets):
        raise ExtractionError("fallback outcomes do not pair one-to-one with targets")
    out = copy.deepcopy(extraction)
    by_id = {e["id"]: e for e in out["entries"]}
    log = []
    for i, (t, o) in enumerate(zip(targets, outcomes)):
        row = {"index": i, "target": [t["entry"], t["property"]], "before": {"status": "ambiguous", "readings": t["readings"]}}
        if o is None:
            row.update(outcome="missing", applied=False)
        elif o.get("status") != "valid":
            row.update(outcome="invalid_response", applied=False, reason=o.get("error") or o.get("status"))
        else:
            r = o["response"]
            err = check_fallback_response(t, r)
            if err:
                row.update(outcome="invalid_response", applied=False, reason=err)
            elif r["status"] == "ambiguous":
                row.update(outcome="abstained", applied=True, reason=r["reason"])
            elif sorted(r["value"]) in [sorted(x) for x in t["readings"]]:
                field = by_id[t["entry"]]["fields"][t["property"]]
                field["status"], field["value"] = r["status"], r["value"]
                field.pop("readings", None)
                row.update(outcome="resolved_within_list", applied=True, reason=r["reason"],
                           after={"status": r["status"], "value": r["value"]})
            else:
                row.update(outcome="out_of_list", applied=False, invented=True, reason=r["reason"], response=r)
        log.append(row)
    return out, log


# ---------------------------------------------------------------- report

def report(extraction, documents, parent_docs, derived_docs, task_kind, fallback=None, quote_policy=None):
    """task_kind: 'plan' or 'review'. fallback: (targets, outcomes) for M3."""
    policy = quote_policy or QUOTE_POLICY
    ids, failures = validate(extraction, documents, parent_docs, derived_docs)
    if failures and policy == "strict":
        raise ExtractionError(f"{len(failures)} quote failure(s), first: {failures[0]}", failures)
    result = {"schema": SCHEMA["schema"], "quote_norm": QUOTE_NORM, "quote_policy": policy,
              "quote_failures": failures, "task_kind": task_kind}
    result["pre"] = _evaluate(ids, task_kind, parent_docs)
    if fallback is not None:
        post_extraction, log = apply_fallback(extraction, *fallback)
        post_ids, _ = validate(post_extraction, documents, parent_docs, derived_docs)
        result["post"] = _evaluate(post_ids, task_kind, parent_docs)
        result["fallback_log"] = log
    return result


def _evaluate(ids, task_kind, parent_docs):
    out = {"status_counts": {s: 0 for s in sorted(STATUSES)}}
    for e in ids.values():
        for f in e["fields"].values():
            out["status_counts"][f["status"]] += 1
    if task_kind == "plan":
        out["plan_flags"], out["plan_entries_extracted"] = plan_flags(ids)
    flags, records = derived_records(ids, parent_docs)
    out["derived_flags"], out["derived_records"] = flags, records
    return out


def main(argv):
    if len(argv) != 2:
        sys.stderr.write("usage: comparator.py INPUT.json  (keys: extraction, documents, parent_docs, derived_docs, task_kind, fallback?)\n")
        return 2
    try:
        with open(argv[1]) as f:
            spec = json.load(f)
        fb = spec.get("fallback")
        rep = report(spec["extraction"], spec["documents"], set(spec["parent_docs"]), set(spec["derived_docs"]),
                     spec["task_kind"], (fb["targets"], fb["outcomes"]) if fb else None)
    except (ExtractionError, ValueError, KeyError, TypeError) as exc:
        print(json.dumps({"error": f"{type(exc).__name__}: {exc}"}))
        return 21
    print(json.dumps(rep, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
