#!/usr/bin/env python3
"""P7 run 1 comparator prototype. Research instrumentation only.

Reads one extraction (schema p7-obligation-extract/v1) and computes, without
any model call:

- plan-time flags on parent entries whose required outcome has no binding
  outcome owner or verifier (bundled or unowned obligations);
- a disposition for every derived entry, comparing each binding field with
  the cited parent field.

It never types prose. Every judgment about what the documents say comes from
the extraction, which is what P7 measures. This is not the parked production
comparator: it has no owner authentication or ownership rules.

Usage: python3 -I comparator.py EXTRACTION.json [--fallback RESOLUTIONS.json]
Prints the report as JSON.
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(HERE, "schema", "p7-obligation-extract-v1.json")) as f:
    SCHEMA = json.load(f)

PROPERTIES = SCHEMA["properties"]
STATUSES = set(SCHEMA["statuses"])
ACTORS = set(SCHEMA["actors"])
PATH_ITEMS = set(SCHEMA["path_items"])
EVIDENCE_ITEMS = set(SCHEMA["evidence_items"])
ASSERTED_AS = {"inherited", "proposed", "new", "realization_choice", "unchanged"}

# Worst first. Anything but "supported" on a derived entry treated as binding is a flag.
SEVERITY = ["contradictory", "requires_owner_decision", "ambiguous_parent", "not_established", "supported"]


class ExtractionError(ValueError):
    pass


def _allowed_values(prop):
    if prop == "path_fact":
        return PATH_ITEMS
    if prop == "acceptance_evidence":
        return EVIDENCE_ITEMS
    return ACTORS


def validate(extraction):
    if extraction.get("schema") != SCHEMA["schema"]:
        raise ExtractionError("wrong or missing schema id")
    ids = set()
    for e in extraction.get("entries", []):
        for key in ("id", "document", "role", "span", "fields"):
            if key not in e:
                raise ExtractionError(f"entry missing {key}: {e.get('id')}")
        if e["id"] in ids:
            raise ExtractionError(f"duplicate id {e['id']}")
        ids.add(e["id"])
        if e["role"] not in ("parent", "derived"):
            raise ExtractionError(f"{e['id']}: bad role")
        for prop, field in e["fields"].items():
            if prop not in PROPERTIES:
                raise ExtractionError(f"{e['id']}: unknown property {prop}")
            if field.get("status") not in STATUSES:
                raise ExtractionError(f"{e['id']}.{prop}: bad status")
            bad = set(field.get("value", [])) - _allowed_values(prop)
            for reading in field.get("readings", []):
                bad |= set(reading) - _allowed_values(prop)
            if bad:
                raise ExtractionError(f"{e['id']}.{prop}: values outside enumeration {sorted(bad)}")
        if e["role"] == "derived":
            if e.get("asserted_as") not in ASSERTED_AS:
                raise ExtractionError(f"{e['id']}: bad asserted_as")
            cites = e.get("cites") or {}
            if cites.get("kind") not in ("specific", "general", "none"):
                raise ExtractionError(f"{e['id']}: bad cites")


def apply_fallback(extraction, resolutions):
    """M3: replace ambiguous fields with bounded fallback resolutions.

    resolutions: list of {entry, property, status, value}. A resolution may only
    target a field whose extracted status is 'ambiguous'; it may itself stay
    'ambiguous'. Returns (new extraction, log).
    """
    out = json.loads(json.dumps(extraction))
    by_id = {e["id"]: e for e in out["entries"]}
    log = []
    for r in resolutions:
        entry = by_id.get(r["entry"])
        field = entry and entry["fields"].get(r["property"])
        if not field or field.get("status") != "ambiguous":
            log.append({"resolution": r, "applied": False, "reason": "target is not an ambiguous field"})
            continue
        if r.get("status") not in STATUSES:
            log.append({"resolution": r, "applied": False, "reason": "bad status"})
            continue
        field["status"] = r["status"]
        field["value"] = r.get("value", [])
        field.pop("readings", None)
        log.append({"resolution": r, "applied": True})
    validate(out)
    return out, log


def plan_flags(extraction):
    flags = []
    for e in extraction["entries"]:
        if e["role"] != "parent":
            continue
        ev = e["fields"].get("acceptance_evidence", {})
        if ev.get("status") != "binding" or not ev.get("value"):
            continue
        missing = [p for p in ("outcome_owner", "verifier")
                   if e["fields"].get(p, {"status": "unspecified"})["status"] in ("unspecified", "ambiguous")]
        if missing:
            flags.append({"entry": e["id"], "document": e["document"], "flag": "unowned_outcome",
                          "properties": missing, "span": e["span"]})
    return flags


def _compare(prop, derived, parent, replaces):
    kind = PROPERTIES[prop]["value_kind"]
    d, p = set(derived), set(parent)
    if "other" in d or "other" in p:
        return "not_established", "an 'other' value cannot be compared"
    if kind == "permitted_actors":
        if "any" in p:
            return ("supported", "parent permits any") if "any" in d else ("requires_owner_decision", "restricts an unrestricted parent")
        if "any" in d or d >= p:
            return "supported", "permits no fewer actors than the parent"
        if not d & p:
            return "contradictory", "disjoint from the parent's permitted actors"
        return "requires_owner_decision", "permits fewer actors than the parent"
    # required_items: larger is stronger
    if not d <= p:
        return "requires_owner_decision", f"requires items the parent does not: {sorted(d - p)}"
    if replaces and d < p:
        return "contradictory", f"replaces the parent but drops {sorted(p - d)}"
    return "supported", "requires nothing beyond the parent"


def _worst(results):
    return min(results, key=lambda r: SEVERITY.index(r[0]))


def derived_dispositions(extraction):
    by_id = {e["id"]: e for e in extraction["entries"]}
    out = []
    for e in extraction["entries"]:
        if e["role"] != "derived":
            continue
        rec = {"entry": e["id"], "document": e["document"], "asserted_as": e["asserted_as"], "span": e["span"]}
        kind = e["cites"]["kind"]
        if e["asserted_as"] == "realization_choice":
            rec.update(disposition="realization", reasons=["author's revisable choice; not a requirement"])
        elif e["asserted_as"] == "proposed":
            rec.update(disposition="proposal", reasons=["offered for decision; never binding"])
        elif kind == "general":
            rec.update(disposition="requires_owner_decision", reasons=["general citation names no specific parent requirement"])
        elif kind == "none":
            rec.update(disposition="requires_owner_decision", reasons=["binding requirement with no parent"])
        else:
            parent = by_id.get(e["cites"].get("parent_entry"))
            if not parent or parent["role"] != "parent":
                rec.update(disposition="not_established", reasons=["cited parent entry not found"])
            else:
                results, skipped = [], []
                for prop, field in e["fields"].items():
                    if field["status"] != "binding":
                        skipped.append(f"{prop}: derived status {field['status']}")
                        continue
                    pf = parent["fields"].get(prop, {"status": "unspecified", "value": []})
                    if pf["status"] in ("unspecified", "selected_realization"):
                        results.append(("requires_owner_decision", f"{prop}: parent is {pf['status']}"))
                    elif pf["status"] == "ambiguous":
                        results.append(("ambiguous_parent", f"{prop}: parent is ambiguous"))
                    else:
                        status, why = _compare(prop, field.get("value", []), pf.get("value", []),
                                               bool(e.get("replaces_parent")))
                        results.append((status, f"{prop}: {why}"))
                if results:
                    worst = _worst(results)[0]
                    rec.update(disposition=worst, reasons=[r[1] for r in results], skipped=skipped)
                else:
                    rec.update(disposition="supported", reasons=["no binding fields asserted"], skipped=skipped)
        if e["asserted_as"] == "unchanged" and rec["disposition"] != "supported":
            rec["reasons"].append("presented as unchanged but not supported by the cited parent")
        rec["flag"] = rec["disposition"] not in ("supported", "realization", "proposal")
        out.append(rec)
    return out


def extraction_quality(extraction):
    counts = {s: 0 for s in STATUSES}
    for e in extraction["entries"]:
        for field in e["fields"].values():
            counts[field["status"]] += 1
    return counts


def report(extraction, resolutions=None):
    validate(extraction)
    log = None
    if resolutions is not None:
        extraction, log = apply_fallback(extraction, resolutions)
    return {"schema": SCHEMA["schema"],
            "plan_flags": plan_flags(extraction),
            "derived": derived_dispositions(extraction),
            "status_counts": extraction_quality(extraction),
            "fallback_log": log}


def main(argv):
    if len(argv) < 2:
        sys.stderr.write(__doc__)
        return 2
    with open(argv[1]) as f:
        extraction = json.load(f)
    resolutions = None
    if "--fallback" in argv:
        with open(argv[argv.index("--fallback") + 1]) as f:
            resolutions = json.load(f)
    try:
        print(json.dumps(report(extraction, resolutions), indent=2))
    except ExtractionError as exc:
        print(json.dumps({"error": str(exc)}))
        return 21
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
