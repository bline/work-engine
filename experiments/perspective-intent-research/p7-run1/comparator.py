#!/usr/bin/env python3
"""P7 run 1 comparator, version 2. Research instrumentation only; no model calls.

Input: one extraction (schema obligation-extract/v2) plus the task's
documents. Output: atomic flags and per-property records. The comparator never
types prose and never decides support from prose: every reading comes from the
extraction, which is what P7 measures. It is not the parked production
comparator (no owner authentication).

Decision table (PROTOCOL section 5 reproduces it):

  For each derived entry and each property P it asserts as binding, or that a
  complete/unchanged entry must carry, combine the binding values of P from
  its support candidates:
    roles: intersection of binding sets ('any' is neutral); empty -> parents_conflict
    required items (evidence, path): union
    independence: equal binding values, else parents_conflict
  Parent state: no candidates or only unspecified/selected -> parent_unspecified;
  no binding but some ambiguous -> parent_ambiguous.

  Comparison of derived D with combined parent V:
    roles:    D == V equal; D strict subset of V stronger; D strict superset of V
              weaker; disjoint contradictory; other overlap changed.
              'any' in V: D 'any' equal, else stronger. 'any' in D only: weaker.
    items:    added = D - V (stronger); dropped = V - D (weaker).
    independence: equal; required vs not_required stronger; reverse weaker.
  Scope partial: weaker/dropped components are entailed (no flag); stronger,
    contradictory and changed flag.
  Scope complete or asserted unchanged: any difference flags; a parent binding
    property the derived entry omits flags as dropped.
  Derived field ambiguous or unspecified on a property it must carry -> not_assessed
  (reported, never treated as supported).

  Flags (atomic, never collapsed): stronger, weaker, dropped, contradictory,
  changed, parent_unspecified, parent_ambiguous, parents_conflict, not_assessed,
  not_comparable ('other'), deficient_provenance (no or only general references),
  unchanged_claim_false, realization_conflicts (a realization choice that is
  stronger, weaker or contradictory against a binding parent),
  adopted_without_owner (a stronger, weaker, dropped or changed requirement whose
  adopter does not match the semantics owner named on its support candidates,
  or whose owner or adopter is unspecified).
  Proposals are compared and reported but raise no standing flags.

  Realization choices are judged by a separate rule: a choice must lie within
  what the binding parent permits. Roles: the chosen set must be a subset of the
  parent's permitted set (any subset of 'any' is fine). Items: a complete choice
  must keep every parent-required item; extra items are fine. Independence: a
  choice may not weaken 'required'. Anything else is realization_conflicts.

Plan-time flags (computed only for plan tasks): owner_ambiguous (outcome_owner
ambiguous); verifier_unassigned (binding acceptance_evidence, verifier
unspecified or ambiguous); independence_unstated (binding acceptance_evidence
and binding verifier, verifier_independence unspecified or ambiguous).
"""

import copy
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCHEMA_PATH = os.path.join(HERE, "schema", "obligation-extract-v2.json")
with open(SCHEMA_PATH) as f:
    SCHEMA = json.load(f)

PROPERTIES = SCHEMA["properties"]
STATUSES = set(SCHEMA["statuses"])
VALUES = {"roles": set(SCHEMA["roles"]), "evidence_items": set(SCHEMA["evidence_items"]),
          "path_items": set(SCHEMA["path_items"]), "independence": set(SCHEMA["independence_values"])}
ASSERTED_AS = {"inherited", "proposed", "new", "realization_choice", "unchanged"}
ITEM_KINDS = {"evidence_items", "path_items"}


class ExtractionError(ValueError):
    """The extraction is invalid; the run's typed output is recorded as invalid."""


def _norm(text):
    return re.sub(r"\s+", " ", text).strip()


def _names(owner):
    out = set()
    for n in (owner or {}).get("names", []) or []:
        n = re.sub(r"[^a-z0-9 -]", "", n.lower()).strip()
        n = re.sub(r"^the ", "", n)
        if n:
            out.add(n)
    return out


def _check_field(eid, prop, field):
    if not isinstance(field, dict):
        raise ExtractionError(f"{eid}.{prop}: field must be an object")
    status = field.get("status")
    if status not in STATUSES:
        raise ExtractionError(f"{eid}.{prop}: bad status {status!r}")
    value = field.get("value", [])
    readings = field.get("readings", [])
    if not isinstance(value, list) or not isinstance(readings, list):
        raise ExtractionError(f"{eid}.{prop}: value and readings must be lists")
    allowed = VALUES[PROPERTIES[prop]["value_kind"]]
    for v in value + [x for r in readings for x in (r if isinstance(r, list) else [None])]:
        if v not in allowed:
            raise ExtractionError(f"{eid}.{prop}: value {v!r} outside enumeration")
    if status in ("binding", "selected_realization") and (not value or readings):
        raise ExtractionError(f"{eid}.{prop}: {status} needs a value and no readings")
    if status == "unspecified" and (value or readings):
        raise ExtractionError(f"{eid}.{prop}: unspecified must have no value or readings")
    if status == "ambiguous" and (value or len(readings) < 2 or any(not r for r in readings)):
        raise ExtractionError(f"{eid}.{prop}: ambiguous needs no value and at least two non-empty readings")
    if PROPERTIES[prop]["value_kind"] == "independence":
        for v in [value] + readings:
            if v and len(v) != 1:
                raise ExtractionError(f"{eid}.{prop}: independence takes exactly one value")
    uses_other = "other" in value or any("other" in r for r in readings)
    if uses_other and not str(field.get("other_text", "")).strip():
        raise ExtractionError(f"{eid}.{prop}: 'other' needs other_text")


def validate(extraction, documents, parent_docs, derived_docs):
    """Strict validation. documents: id -> text; parent_docs/derived_docs: id sets."""
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
                raise ExtractionError(f"entry missing {key}: {e.get('id')}")
        eid = e["id"]
        if not isinstance(eid, str) or eid in ids:
            raise ExtractionError(f"bad or duplicate id {eid!r}")
        ids[eid] = e
        if e["role"] == "parent" and e["document"] not in parent_docs:
            raise ExtractionError(f"{eid}: parent entry from non-parent document {e['document']!r}")
        if e["role"] == "derived" and e["document"] not in derived_docs:
            raise ExtractionError(f"{eid}: derived entry from non-derived document {e['document']!r}")
        if e["role"] not in ("parent", "derived"):
            raise ExtractionError(f"{eid}: bad role")
        if not isinstance(e["span"], str) or not _norm(e["span"]) or _norm(e["span"]) not in _norm(documents[e["document"]]):
            raise ExtractionError(f"{eid}: span is not quoted from {e['document']}")
        if not isinstance(e["fields"], dict):
            raise ExtractionError(f"{eid}: fields must be an object")
        for prop, field in e["fields"].items():
            if prop not in PROPERTIES:
                raise ExtractionError(f"{eid}: unknown property {prop!r}")
            _check_field(eid, prop, field)
        for owner_key in ("semantics_owner", "adopted_by"):
            owner = e.get(owner_key)
            if owner is not None:
                if not isinstance(owner, dict) or owner.get("status") not in STATUSES or not isinstance(owner.get("names", []), list):
                    raise ExtractionError(f"{eid}: bad {owner_key}")
        if e["role"] == "derived":
            if e.get("asserted_as") not in ASSERTED_AS:
                raise ExtractionError(f"{eid}: bad asserted_as")
            if e.get("scope") not in ("partial", "complete"):
                raise ExtractionError(f"{eid}: bad scope")
            refs = e.get("references_made")
            if not isinstance(refs, list):
                raise ExtractionError(f"{eid}: references_made must be a list")
            for r in refs:
                if not isinstance(r, dict) or r.get("kind") not in ("specific", "general") or not str(r.get("text", "")).strip():
                    raise ExtractionError(f"{eid}: bad reference")
            cands = e.get("support_candidates")
            if not isinstance(cands, dict):
                raise ExtractionError(f"{eid}: support_candidates must be an object")
    for e in entries:
        if e["role"] != "derived":
            continue
        for prop, cand in e["support_candidates"].items():
            if prop not in PROPERTIES or not isinstance(cand, list):
                raise ExtractionError(f"{e['id']}: bad support_candidates for {prop!r}")
            for pid in cand:
                if pid not in ids or ids[pid]["role"] != "parent":
                    raise ExtractionError(f"{e['id']}: support candidate {pid!r} is not a parent entry")
    return ids


def _combine(prop, fields):
    kind = PROPERTIES[prop]["value_kind"]
    binding = [set(f["value"]) for f in fields if f["status"] == "binding"]
    if not binding:
        if any(f["status"] == "ambiguous" for f in fields):
            return "parent_ambiguous", None
        return "parent_unspecified", None
    if kind in ITEM_KINDS:
        return "ok", set().union(*binding)
    if kind == "independence":
        return ("ok", binding[0]) if all(b == binding[0] for b in binding) else ("parents_conflict", None)
    sets = [b for b in binding if "any" not in b]
    if not sets:
        return "ok", {"any"}
    combined = set.intersection(*sets)
    return ("ok", combined) if combined else ("parents_conflict", None)


def _relation(prop, d, v):
    """Return set of components among: equal, stronger, weaker, contradictory, changed, not_comparable."""
    kind = PROPERTIES[prop]["value_kind"]
    if "other" in d or "other" in v:
        return {"not_comparable"}
    if kind in ITEM_KINDS:
        rel = set()
        if d - v:
            rel.add("stronger")
        if v - d:
            rel.add("weaker")
        return rel or {"equal"}
    if kind == "independence":
        if d == v:
            return {"equal"}
        return {"stronger"} if d == {"required"} else {"weaker"}
    if "any" in v:
        return {"equal"} if "any" in d else {"stronger"}
    if "any" in d:
        return {"weaker"}
    if d == v:
        return {"equal"}
    if d < v:
        return {"stronger"}
    if d > v:
        return {"weaker"}
    if not d & v:
        return {"contradictory"}
    return {"changed"}


def _realization_conflict(prop, d, v, complete):
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
        return "contradictory" if not d & v and "any" not in d else "outside_permitted"
    return None


def _owner_status(entry, cands_by_prop, ids):
    owners = set()
    unknown = False
    for cand in cands_by_prop.values():
        for pid in cand:
            so = ids[pid].get("semantics_owner")
            if so and so.get("status") == "binding" and _names(so):
                owners |= _names(so)
            else:
                unknown = True
    adopter = entry.get("adopted_by")
    adopter_names = _names(adopter) if adopter and adopter.get("status") == "binding" else set()
    return owners, adopter_names, unknown


def derived_records(ids):
    flags, records = [], []
    for e in ids.values():
        if e["role"] != "derived":
            continue
        complete = e["scope"] == "complete" or e["asserted_as"] == "unchanged"
        cands = e["support_candidates"]
        if e["asserted_as"] == "realization_choice":
            records.append(_realization_record(e, ids, complete, flags))
            continue
        props = {p for p, f in e["fields"].items() if f["status"] == "binding"}
        if complete:
            for p, cand in cands.items():
                if any(ids[pid]["fields"].get(p, {}).get("status") == "binding" for pid in cand):
                    props.add(p)
        entry_flags = []
        prop_records = {}
        for p in sorted(props):
            parent_fields = [ids[pid]["fields"][p] for pid in cands.get(p, []) if p in ids[pid]["fields"]]
            state, v = _combine(p, parent_fields)
            df = e["fields"].get(p, {"status": "unspecified", "value": []})
            rec = {"parent_state": state, "derived_status": df["status"],
                   "candidates": cands.get(p, [])}
            if df["status"] not in ("binding", "selected_realization"):
                if state == "ok":
                    rel = {"dropped"} if df["status"] == "unspecified" and complete else {"not_assessed"}
                else:
                    rel = {"not_assessed"}
            elif state != "ok":
                rel = {state}
            else:
                rel = _relation(p, set(df["value"]), v)
                if not complete:
                    rel -= {"weaker"}
                    rel = rel or {"equal"}
            rec["relation"] = sorted(rel)
            prop_records[p] = rec
            entry_flags += [(p, r) for r in rel if r != "equal"]
        refs = e["references_made"]
        deficient = not refs or all(r["kind"] == "general" for r in refs)
        record = {"entry": e["id"], "document": e["document"], "asserted_as": e["asserted_as"],
                  "scope": e["scope"], "span": e["span"], "properties": prop_records,
                  "references_made": refs, "deficient_provenance": deficient}
        changes = {r for _, r in entry_flags} & {"stronger", "weaker", "dropped", "contradictory", "changed"}
        if e["asserted_as"] == "proposed":
            record["standing"] = "proposal"
        else:
            for p, r in entry_flags:
                flags.append({"entry": e["id"], "property": p, "flag": r})
            if deficient and props:
                flags.append({"entry": e["id"], "flag": "deficient_provenance"})
            if e["asserted_as"] == "unchanged" and changes:
                flags.append({"entry": e["id"], "flag": "unchanged_claim_false", "detail": sorted(changes)})
            if changes:
                owners, adopter, unknown = _owner_status(e, {p: cands.get(p, []) for p in props}, ids)
                if unknown or not owners or not adopter or not (owners & adopter):
                    flags.append({"entry": e["id"], "flag": "adopted_without_owner",
                                  "owners": sorted(owners), "adopter": sorted(adopter), "owner_unknown": unknown})
            record["standing"] = "supported" if not entry_flags and not (deficient and props) else "flagged"
        records.append(record)
    return flags, records


def _realization_record(e, ids, complete, flags):
    cands = e["support_candidates"]
    props = {p for p, f in e["fields"].items() if f["status"] in ("binding", "selected_realization")}
    if complete:
        for p, cand in cands.items():
            if any(ids[pid]["fields"].get(p, {}).get("status") == "binding" for pid in cand):
                props.add(p)
    prop_records, conflicts = {}, []
    for p in sorted(props):
        parent_fields = [ids[pid]["fields"][p] for pid in cands.get(p, []) if p in ids[pid]["fields"]]
        state, v = _combine(p, parent_fields)
        df = e["fields"].get(p, {"status": "unspecified", "value": []})
        if state != "ok":
            result = None if state == "parent_unspecified" else state
        elif df["status"] not in ("binding", "selected_realization"):
            result = "dropped" if complete else None
        else:
            result = _realization_conflict(p, set(df["value"]), v, complete)
        prop_records[p] = {"parent_state": state, "derived_status": df["status"],
                           "candidates": cands.get(p, []), "conflict": result}
        if result:
            conflicts.append((p, result))
    for p, r in conflicts:
        flags.append({"entry": e["id"], "property": p, "flag": "realization_conflicts", "detail": r})
    return {"entry": e["id"], "document": e["document"], "asserted_as": e["asserted_as"],
            "scope": e["scope"], "span": e["span"], "properties": prop_records,
            "references_made": e["references_made"],
            "standing": "realization_conflicts" if conflicts else "realization"}


def plan_flags(ids):
    flags, evaluated = [], 0
    for e in ids.values():
        if e["role"] != "parent":
            continue
        evaluated += 1
        f = e["fields"]
        status = lambda p: f.get(p, {"status": "unspecified"})["status"]
        evidence_required = status("acceptance_evidence") == "binding"
        if status("outcome_owner") == "ambiguous":
            flags.append({"entry": e["id"], "flag": "owner_ambiguous", "span": e["span"]})
        if evidence_required and status("verifier") in ("unspecified", "ambiguous"):
            flags.append({"entry": e["id"], "flag": "verifier_unassigned", "span": e["span"]})
        if evidence_required and status("verifier") == "binding" and status("verifier_independence") in ("unspecified", "ambiguous"):
            flags.append({"entry": e["id"], "flag": "independence_unstated", "span": e["span"]})
    return flags, evaluated


def fallback_targets(extraction):
    """Fixed list of ambiguous property fields, computed from the original extraction."""
    return [{"entry": e["id"], "property": p, "readings": f["readings"], "span": e["span"]}
            for e in extraction["entries"] for p, f in e["fields"].items() if f["status"] == "ambiguous"]


def apply_fallback(extraction, targets, responses):
    """Apply exactly one response per fixed target. Returns (new extraction, log).

    A response may bind or select one of the original readings, or stay ambiguous
    (readings preserved). Any other value is an invention: rejected and logged.
    """
    out = copy.deepcopy(extraction)
    by_id = {e["id"]: e for e in out["entries"]}
    keyed = {}
    log = []
    for r in responses:
        key = (r.get("entry"), r.get("property"))
        if key in keyed:
            log.append({"response": r, "applied": False, "reason": "duplicate response for target"})
            continue
        keyed[key] = r
    for t in targets:
        key = (t["entry"], t["property"])
        r = keyed.pop(key, None)
        if r is None:
            log.append({"target": key, "applied": False, "reason": "no response"})
            continue
        field = by_id[t["entry"]]["fields"][t["property"]]
        status, value, reason = r.get("status"), r.get("value", []), str(r.get("reason", "")).strip()
        if not reason:
            log.append({"target": key, "applied": False, "reason": "missing reason"})
        elif status == "ambiguous":
            log.append({"target": key, "applied": True, "result": "abstained"})
        elif status in ("binding", "selected_realization") and value in t["readings"]:
            field["status"], field["value"] = status, value
            field.pop("readings", None)
            log.append({"target": key, "applied": True, "result": status, "value": value})
        else:
            log.append({"target": key, "applied": False, "reason": "invented or invalid value", "invented": True, "response": r})
    for key, r in keyed.items():
        log.append({"response": r, "applied": False, "reason": "not a fixed target"})
    return out, log


def report(extraction, documents, parent_docs, derived_docs, task_kind, fallback=None):
    """task_kind: 'plan' or 'review'. fallback: (targets, responses) for M3."""
    ids = validate(extraction, documents, parent_docs, derived_docs)
    result = {"schema": SCHEMA["schema"], "task_kind": task_kind}
    result["pre"] = _evaluate(ids, task_kind)
    if fallback is not None:
        post_extraction, log = apply_fallback(extraction, *fallback)
        post_ids = validate(post_extraction, documents, parent_docs, derived_docs)
        result["post"] = _evaluate(post_ids, task_kind)
        result["fallback_log"] = log
    return result


def _evaluate(ids, task_kind):
    out = {"status_counts": {s: 0 for s in STATUSES}}
    for e in ids.values():
        for f in e["fields"].values():
            out["status_counts"][f["status"]] += 1
    if task_kind == "plan":
        out["plan_flags"], out["plan_entries_evaluated"] = plan_flags(ids)
    flags, records = derived_records(ids)
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
                     spec["task_kind"], (fb["targets"], fb["responses"]) if fb else None)
    except (ExtractionError, ValueError, KeyError, TypeError) as exc:
        print(json.dumps({"error": f"{type(exc).__name__}: {exc}"}))
        return 21
    print(json.dumps(rep, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
