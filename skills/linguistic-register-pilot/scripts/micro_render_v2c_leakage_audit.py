#!/usr/bin/env python3
"""Audit a checkpoint export of micro-render v2c without altering subject evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable

import yaml


SUBJECT_COMMIT = "62cc7f7039cfc6238de8cc7d16a030ebe1892bf2"
SUBJECT_TREE = "3b4a3eb525636dbe19eaafaffaece8bb4c6153bf"
EXPERIMENT_ID = "semantic-licensed-micro-render-v2c-2026-08-25"
ROOT = Path("skills/linguistic-register-pilot")
RUN_REL = ROOT / "pilot/micro-render-v2c/runs"
PROFILE_PATHS = {
    "gelman-model-criticism": ROOT / "pilot/candidates/gelman-model-criticism/profile.yaml",
    "leveson-system-safety": ROOT / "pilot/candidates/leveson/profile.yaml",
    "shaw-engineering-judgment": ROOT / "pilot/candidates/shaw-engineering-judgment/profile.yaml",
}
CONDITIONS = (
    "gelman-model-criticism",
    "leveson-system-safety",
    "neutral-editorial-defaults",
    "shaw-engineering-judgment",
)
FORBIDDEN_LITERALS = (
    "Andrew Gelman", "Gelman", "Nancy Leveson", "Leveson", "Mary Shaw", "Shaw",
    *CONDITIONS,
)
TRANSPORT_TERMS = (
    "wrapper", "unwrapped", "normalization", "transport", "raw_response",
    "extracted_prose", "condition_id", "answer_key", "source_feature_id",
)
SOURCE_DOMAIN_TERMS = {
    "gelman-model-criticism": ("bayesian", "posterior", "prior distribution", "model checking"),
    "leveson-system-safety": ("accident", "hazard", "oil and gas", "safety assurance", "control problem"),
    "shaw-engineering-judgment": ("software engineering research", "research paper", "research method"),
}
WORD_RE = re.compile(r"[A-Za-z0-9]+(?:['’][A-Za-z0-9]+)?")
SAMPLE_RE = re.compile(r"\bX[0-9A-F]{12}\b")
LIST_RE = re.compile(r"(?m)^\s*(?:[-*+] |\d+[.)] )")
HEADING_RE = re.compile(r"(?m)^\s*#{1,6}\s+\S")
STOPWORDS = {
    "a", "an", "and", "as", "at", "be", "but", "by", "for", "from", "in", "is",
    "it", "of", "on", "or", "that", "the", "this", "to", "with",
}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git_blob_oid(value: bytes) -> str:
    header = f"blob {len(value)}\0".encode()
    return hashlib.sha1(header + value).hexdigest()


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def normalize_words(value: str) -> list[str]:
    return [word.casefold().replace("’", "'") for word in WORD_RE.findall(value)]


def ngrams(value: str, minimum: int = 3, maximum: int = 8) -> set[str]:
    words = normalize_words(value)
    result: set[str] = set()
    for size in range(minimum, min(maximum, len(words)) + 1):
        for index in range(len(words) - size + 1):
            gram_words = words[index:index + size]
            if any(word not in STOPWORDS for word in gram_words):
                result.add(" ".join(gram_words))
    return result


def literal_hits(value: str, literals: Iterable[str]) -> list[str]:
    folded = value.casefold()
    return sorted({
        literal for literal in literals
        if re.search(r"(?<!\w)" + re.escape(literal.casefold()) + r"(?!\w)", folded)
    })


def verify_inventory(subject: Path, inventory_path: Path) -> dict[str, Any]:
    inventory = load_json(inventory_path)
    expected = {entry["path"]: entry for entry in inventory["entries"]}
    observed_paths = sorted(
        str(path.relative_to(subject)) for path in subject.rglob("*") if path.is_file()
    )
    missing = sorted(set(expected) - set(observed_paths))
    extra = sorted(set(observed_paths) - set(expected))
    mismatches = []
    for relative in sorted(set(expected) & set(observed_paths)):
        path = subject / relative
        observed_oid = git_blob_oid(path.read_bytes())
        if observed_oid != expected[relative]["oid"]:
            mismatches.append({"path": relative, "expected": expected[relative]["oid"], "observed": observed_oid})
    return {
        "checkpoint_commit_oid": inventory["checkpoint_commit_oid"],
        "checkpoint_tree_oid": inventory["checkpoint_tree_oid"],
        "expected_entries": len(expected),
        "observed_files": len(observed_paths),
        "missing": missing,
        "extra": extra,
        "blob_oid_mismatches": mismatches,
        "verified": (
            inventory["checkpoint_commit_oid"] == SUBJECT_COMMIT
            and inventory["checkpoint_tree_oid"] == SUBJECT_TREE
            and not missing and not extra and not mismatches
        ),
    }


def recursive_strings(value: Any, prefix: str = "$") -> Iterable[tuple[str, str]]:
    if isinstance(value, dict):
        for key, child in value.items():
            yield f"{prefix}.<key>", str(key)
            yield from recursive_strings(child, f"{prefix}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from recursive_strings(child, f"{prefix}[{index}]")
    elif isinstance(value, str):
        yield prefix, value


def matcher_packet_audit(root: Path) -> dict[str, Any]:
    matching = root / RUN_REL / "matching"
    passes = []
    all_direct_cue_hits = []
    reference_permutations = []
    text_permutations = []
    for number in range(1, 4):
        directory = matching / f"pass-{number:02d}"
        packet_path = directory / "packet.json"
        packet_bytes = packet_path.read_bytes()
        packet = json.loads(packet_bytes)
        key = load_json(directory / "key.json")
        result = load_json(directory / "result.json")
        receipt = load_json(directory / "execution-receipt.json")
        marker = load_json(directory / "attempt-marker.json")
        manifest = load_json(matching / "launch-manifest.json")
        packet_sha = sha256_bytes(packet_bytes)
        descriptor = next(job for job in manifest["launch_set"]["jobs"] if job["job_id"] == f"matching-pass-{number:02d}")
        digest_checks = {
            "key": key["packet_artifact_sha256"] == packet_sha,
            "result": result["packet_artifact_sha256"] == packet_sha,
            "receipt": receipt["packet_artifact_sha256"] == packet_sha,
            "marker": marker["packet_artifact_sha256"] == packet_sha,
            "manifest": descriptor["packet_artifact_sha256"] == packet_sha,
        }
        packet_text = packet_bytes.decode()
        forbidden = literal_hits(packet_text, FORBIDDEN_LITERALS)
        transport = literal_hits(packet_text, TRANSPORT_TERMS)
        opaque_ids = sorted(set(SAMPLE_RE.findall(packet_text)))
        field_shapes = {
            "anonymous_text_fields": sorted({tuple(sorted(item)) for item in packet["anonymous_texts"]}),
            "anonymous_reference_fields": sorted({tuple(sorted(item)) for item in packet["anonymous_references"]}),
        }
        reconstructed = []
        for anonymous in packet["anonymous_texts"]:
            source_id = key["text_mapping"][anonymous["text_id"]]
            source_text = load_json(root / RUN_REL / "samples" / source_id / "render.json")["text"]
            reconstructed.append({
                "text_id": anonymous["text_id"],
                "source_sample_id": source_id,
                "exact_text_match": anonymous["text"] == source_text,
            })
        event_text = (directory / "events.jsonl").read_text()
        event_key_hits = literal_hits(event_text, (*CONDITIONS, "condition_by_sample", "reference_mapping", "text_mapping"))
        cue_record = {
            "pass_number": number,
            "packet_sha256": packet_sha,
            "digest_checks": digest_checks,
            "field_shapes": field_shapes,
            "forbidden_literal_hits": forbidden,
            "transport_or_ingestion_term_hits": transport,
            "opaque_source_sample_id_hits": opaque_ids,
            "event_stream_key_or_condition_hits": event_key_hits,
            "all_normalized_texts_reconstructed_exactly": all(row["exact_text_match"] for row in reconstructed),
            "reconstruction": reconstructed,
        }
        if forbidden or transport or opaque_ids or event_key_hits:
            all_direct_cue_hits.append(cue_record)
        passes.append(cue_record)
        reference_permutations.append(list(key["reference_mapping"].values()))
        text_permutations.append(list(key["text_mapping"].values()))
    return {
        "passes": passes,
        "all_packet_digests_bound": all(all(row["digest_checks"].values()) for row in passes),
        "all_texts_reconstruct_exactly": all(row["all_normalized_texts_reconstructed_exactly"] for row in passes),
        "direct_cue_hits": all_direct_cue_hits,
        "wrapper_metadata_absent_from_retained_matcher_packets": not all_direct_cue_hits,
        "distinct_reference_permutations": len({tuple(row) for row in reference_permutations}),
        "distinct_text_permutations": len({tuple(row) for row in text_permutations}),
        "interpretation": "Retained matcher packet bytes contain normalized prose and randomized anonymous IDs only; wrapper disposition is mechanically unavailable as a direct cue." if not all_direct_cue_hits else "A potential direct cue requires adjudication.",
    }


def sample_metrics(text: str) -> dict[str, Any]:
    punctuation = {symbol: text.count(symbol) for symbol in (".", ",", ";", ":", "?", "!", "—", "-", "(", ")")}
    sentences = [part for part in re.split(r"(?<=[.!?])\s+", text.strip()) if part]
    paragraphs = [part for part in re.split(r"\n\s*\n", text.strip()) if part]
    return {
        "word_count": len(normalize_words(text)),
        "sentence_count": len(sentences),
        "paragraph_count": len(paragraphs),
        "heading_count": len(HEADING_RE.findall(text)),
        "list_marker_count": len(LIST_RE.findall(text)),
        "punctuation_counts": punctuation,
    }


def flatten_metrics(metrics: dict[str, Any]) -> dict[str, int]:
    flat = {key: value for key, value in metrics.items() if isinstance(value, int)}
    flat.update({f"punctuation_{key}": value for key, value in metrics["punctuation_counts"].items()})
    return flat


def metric_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    by_condition: dict[str, list[dict[str, int]]] = defaultdict(list)
    for row in rows:
        by_condition[row["condition_id"]].append(flatten_metrics(row["metrics"]))
    names = sorted(next(iter(by_condition.values()))[0])
    summaries = {}
    disjoint = {}
    one_vs_rest = {}
    for name in names:
        summaries[name] = {}
        value_sets = {}
        for condition in CONDITIONS:
            values = [row[name] for row in by_condition[condition]]
            summaries[name][condition] = {
                "values": values,
                "min": min(values), "max": max(values), "mean": statistics.fmean(values),
            }
            value_sets[condition] = set(values)
        disjoint[name] = all(
            value_sets[left].isdisjoint(value_sets[right])
            for index, left in enumerate(CONDITIONS)
            for right in CONDITIONS[index + 1:]
        )
        one_vs_rest[name] = {
            condition: value_sets[condition].isdisjoint(set().union(*(
                value_sets[other] for other in CONDITIONS if other != condition
            )))
            for condition in CONDITIONS
        }
    return {
        "by_metric_and_condition": summaries,
        "pairwise_disjoint_for_all_conditions": disjoint,
        "condition_disjoint_from_all_others": one_vs_rest,
    }


def wrapper_and_feature_audit(root: Path) -> dict[str, Any]:
    runs = root / RUN_REL
    unblinding = load_json(runs / "sealed-unblinding.json")["samples"]
    wrapper_counts = {condition: Counter() for condition in CONDITIONS}
    rows = []
    forbidden_hits = []
    domain_hits = []
    for sample_id, mapping in sorted(unblinding.items()):
        render = load_json(runs / "samples" / sample_id / "render.json")
        raw = load_json(runs / "samples" / sample_id / "raw-render.json")
        condition = mapping["condition_id"]
        disposition = render["normalization"]
        wrapper_counts[condition][disposition] += 1
        wrapper_shape_valid = (
            (disposition == "direct" and isinstance(raw, str))
            or (disposition == "unwrapped_once" and isinstance(raw, dict) and set(raw) == {"text"} and raw["text"] == render["text"])
        )
        literal = literal_hits(render["text"], FORBIDDEN_LITERALS)
        if literal:
            forbidden_hits.append({"sample_id": sample_id, "condition_id": condition, "hits": literal})
        for source_condition, terms in SOURCE_DOMAIN_TERMS.items():
            hits = literal_hits(render["text"], terms)
            if hits:
                domain_hits.append({
                    "sample_id": sample_id, "condition_id": condition,
                    "source_condition": source_condition, "hits": hits,
                })
        rows.append({
            "sample_id": sample_id,
            "condition_id": condition,
            "brief_id": mapping["brief_id"],
            "normalization": disposition,
            "exact_wrapper_shape_valid": wrapper_shape_valid,
            "metrics": sample_metrics(render["text"]),
        })
    report_counts = load_json(runs / "transport-gate-report.json")["normalization_by_condition"]
    recomputed_counts = {
        condition: {
            "direct": wrapper_counts[condition]["direct"],
            "unwrapped_once": wrapper_counts[condition]["unwrapped_once"],
            "rejected": wrapper_counts[condition]["rejected"],
        } for condition in CONDITIONS
    }
    return {
        "wrapper_distribution_by_condition": recomputed_counts,
        "matches_retained_transport_report": recomputed_counts == report_counts,
        "all_raw_shapes_match_disposition": all(row["exact_wrapper_shape_valid"] for row in rows),
        "forbidden_literal_hits_in_renders": forbidden_hits,
        "source_domain_term_hits_in_renders": domain_hits,
        "samples": rows,
        "metric_summary": metric_summary(rows),
        "metric_limitation": "Observed feature distributions are descriptive. Punctuation and sentence structure can be licensed intended manipulation signals; no univariate difference is treated as mechanistic proof.",
    }


def profile_and_quotation_audit(root: Path) -> dict[str, Any]:
    profiles = {}
    for condition, relative in PROFILE_PATHS.items():
        profile = yaml.safe_load((root / relative).read_text())
        retained = [feature for feature in profile["features"] if feature["disposition"] == "realization_only"]
        profiles[condition] = {
            "path": str(relative),
            "content_screening": profile["content_screening"],
            "retained_feature_ids": [feature["id"] for feature in retained],
            "retained_text": " ".join(
                str(feature[field]) for feature in retained for field in ("id", "category", "description")
            ),
        }
    runs = root / RUN_REL
    unblinding = load_json(runs / "sealed-unblinding.json")["samples"]
    matching_packet = load_json(runs / "matching/pass-01/packet.json")
    matching_key = load_json(runs / "matching/pass-01/key.json")
    reference_by_id = {item["reference_id"]: item for item in matching_packet["anonymous_references"]}
    style_card_by_condition = {
        condition: reference_by_id[reference_id]
        for reference_id, condition in matching_key["reference_mapping"].items()
    }
    overlap_rows = []
    style_card_overlap_rows = []
    for sample_id, mapping in sorted(unblinding.items()):
        condition = mapping["condition_id"]
        text = load_json(runs / "samples" / sample_id / "render.json")["text"]
        style_card = style_card_by_condition[condition]
        style_card_text = " ".join(
            str(value) for feature in style_card["licensed_style_features"] for value in feature.values()
        )
        style_overlap = sorted(ngrams(text) & ngrams(style_card_text), key=lambda value: (-len(value.split()), value))
        style_card_overlap_rows.append({
            "sample_id": sample_id, "condition_id": condition, "overlap": style_overlap,
        })
        if condition == "neutral-editorial-defaults":
            continue
        overlap = sorted(ngrams(text) & ngrams(profiles[condition]["retained_text"]), key=lambda value: (-len(value.split()), value))
        overlap_rows.append({"sample_id": sample_id, "condition_id": condition, "overlap": overlap})
    corpus_candidates = [
        path for path in (root / ROOT / "pilot").rglob("*")
        if path.is_file() and "corpus" in path.name.casefold()
    ]
    return {
        "profiles": profiles,
        "render_profile_ngram_overlap": overlap_rows,
        "render_matcher_style_card_ngram_overlap": style_card_overlap_rows,
        "corpus_artifact_paths_in_bound_subject": [str(path.relative_to(root)) for path in corpus_candidates],
        "exact_corpus_quotation_coverage": "available" if corpus_candidates else "unresolved_source_corpora_absent_from_bound_subject",
        "limitation": "Profile content-screening claims names, copied language, and domain terms were removed, but the bound checkpoint contains no source corpus bytes from which to independently recompute exact quotation absence." if not corpus_candidates else None,
    }


def extract_skill_content(output: str) -> tuple[str | None, str]:
    marker = "---\nname:"
    index = output.find(marker)
    if index < 0:
        return None, output
    return output[index:], output[:index]


def global_skill_audit(root: Path) -> dict[str, Any]:
    runs = root / RUN_REL
    unblinding = load_json(runs / "sealed-unblinding.json")["samples"]
    exposures = []
    invocation_rows = []
    for sample_id, mapping in sorted(unblinding.items()):
        event_path = runs / "samples" / sample_id / "render-events.jsonl"
        sample_exposures = []
        for line_number, line in enumerate(event_path.read_text().splitlines(), 1):
            event = json.loads(line)
            item = event.get("item", {})
            if event.get("type") != "item.completed" or item.get("type") != "command_execution":
                continue
            command = item.get("command", "")
            if "/skills/" not in command or "SKILL.md" not in command:
                continue
            match = re.search(r"(/[^\"' ]+/skills/[^\"' ]+/SKILL\.md)", command)
            path = match.group(1) if match else "unresolved"
            content, prefix = extract_skill_content(item.get("aggregated_output", ""))
            record = {
                "sample_id": sample_id,
                "condition_id": mapping["condition_id"],
                "event_path": str(event_path.relative_to(root)),
                "event_line": line_number,
                "external_path": path,
                "command": command,
                "content_recovered_from_event": content is not None,
                "event_recovered_content_sha256": sha256_bytes(content.encode()) if content is not None else None,
                "event_recovered_content": content,
                "diagnostic_prefix": prefix,
                "forbidden_literal_hits": literal_hits(content or "", FORBIDDEN_LITERALS),
                "source_domain_term_hits": {
                    condition: literal_hits(content or "", terms)
                    for condition, terms in SOURCE_DOMAIN_TERMS.items()
                    if literal_hits(content or "", terms)
                },
                "formatting_language_hits": literal_hits(content or "", ("format", "paragraph", "syntax", "rhythm", "style")),
                "epistemic_language_hits": literal_hits(content or "", ("evidence", "claim", "inference", "confidence", "uncertainty")),
                "visibility_claim": "The retained invocation event records completed command output. The historical bytes returned in that event are established; mechanistic use or attention remains unresolved.",
            }
            exposures.append(record)
            sample_exposures.append({"external_path": path, "event_line": line_number})
        invocation_rows.append({
            "sample_id": sample_id,
            "condition_id": mapping["condition_id"],
            "external_skill_reads": sample_exposures,
        })
    exposure_conditions = Counter(record["condition_id"] for record in exposures)
    return {
        "render_invocations_audited": len(invocation_rows),
        "invocations": invocation_rows,
        "exposures": exposures,
        "exposure_count": len(exposures),
        "exposure_by_condition": dict(sorted(exposure_conditions.items())),
        "all_invocations_saw_same_skill_content": bool(invocation_rows) and all(
            row["external_skill_reads"] == invocation_rows[0]["external_skill_reads"] for row in invocation_rows
        ),
        "condition_adjacent_content_found": any(
            row["forbidden_literal_hits"] or row["source_domain_term_hits"] for row in exposures
        ),
        "classification": "unequal_condition-neutral exposure" if exposures and not any(row["forbidden_literal_hits"] or row["source_domain_term_hits"] for row in exposures) else ("no external skill exposure" if not exposures else "potential contamination"),
    }


def rationale_audit(root: Path) -> dict[str, Any]:
    matching = root / RUN_REL / "matching"
    rows = []
    counts = Counter()
    for number in range(1, 4):
        directory = matching / f"pass-{number:02d}"
        packet = load_json(directory / "packet.json")
        key = load_json(directory / "key.json")
        result = load_json(directory / "result.json")
        references = {item["reference_id"]: item for item in packet["anonymous_references"]}
        texts = {item["text_id"]: item["text"] for item in packet["anonymous_texts"]}
        for assignment in result["assignments"]:
            rationale = assignment["rationale"]
            reference = references[assignment["reference_id"]]
            reference_text = " ".join(
                str(value) for feature in reference["licensed_style_features"] for value in feature.values()
            )
            overlap = sorted(ngrams(rationale, 3, 6) & ngrams(reference_text, 3, 6), key=lambda value: (-len(value.split()), value))
            intended_terms = literal_hits(rationale, (
                "syntax", "question", "contrast", "declarative", "transition", "transitions", "connective",
                "qualification", "limitation", "evidence limit", "boundary", "recommendation", "first-person",
                "epistemic", "parallel", "claim", "conclusion", "sentence", "exposition", "sequencing", "posture", "restates",
            ))
            domain_author = literal_hits(rationale, (*FORBIDDEN_LITERALS, *(term for terms in SOURCE_DOMAIN_TERMS.values() for term in terms)))
            length_format = literal_hits(rationale, ("short", "compact", "concise", "length", "paragraph", "heading", "bullet", "numbered", "format"))
            structural = literal_hits(rationale, ("text_id", "reference_id", "T01", "R01", "wrapper", "order", "position", "metadata", "transport", "normalization"))
            categories = {
                "intended_register_features": bool(intended_terms or overlap),
                "explicit_lexical_overlap_with_profile_or_style_card": bool(overlap),
                "domain_or_author_clues": bool(domain_author),
                "length_or_formatting": bool(length_format),
                "structural_artifacts": bool(structural),
                "unsupported_confidence": assignment["confidence"] == 5 and not (intended_terms or overlap or domain_author or length_format or structural),
            }
            for category, present in categories.items():
                if present:
                    counts[category] += 1
            rows.append({
                "pass_number": number,
                "text_id": assignment["text_id"],
                "reference_id": assignment["reference_id"],
                "source_sample_id": key["text_mapping"][assignment["text_id"]],
                "assigned_condition": key["reference_mapping"][assignment["reference_id"]],
                "confidence": assignment["confidence"],
                "rationale": rationale,
                "categories": categories,
                "evidence": {
                    "intended_terms": intended_terms,
                    "reference_card_ngram_overlap": overlap,
                    "domain_or_author_hits": domain_author,
                    "length_or_format_hits": length_format,
                    "structural_hits": structural,
                    "render_text_sha256": sha256_bytes(texts[assignment["text_id"]].encode()),
                },
            })
    return {
        "assignments_audited": len(rows),
        "category_counts": {category: counts[category] for category in (
            "intended_register_features", "explicit_lexical_overlap_with_profile_or_style_card",
            "domain_or_author_clues", "length_or_formatting", "structural_artifacts", "unsupported_confidence",
        )},
        "confidence_values": sorted({row["confidence"] for row in rows}),
        "confidence_metric_status": "failed_secondary_metric_uncalibrated_and_without_variance",
        "rows": rows,
        "limitation": "Rationales are observable reports, not faithful mechanistic traces. Category absence cannot prove a cue was unused.",
    }


def build_report(subject: Path, inventory_path: Path) -> dict[str, Any]:
    inventory = verify_inventory(subject, inventory_path)
    if not inventory["verified"]:
        raise ValueError("subject export does not match the frozen inventory")
    matcher = matcher_packet_audit(subject)
    wrapper_features = wrapper_and_feature_audit(subject)
    profiles = profile_and_quotation_audit(subject)
    skills = global_skill_audit(subject)
    rationales = rationale_audit(subject)
    direct_contamination = bool(
        matcher["direct_cue_hits"]
        or wrapper_features["forbidden_literal_hits_in_renders"]
        or skills["condition_adjacent_content_found"]
        or rationales["category_counts"]["domain_or_author_clues"]
        or rationales["category_counts"]["structural_artifacts"]
    )
    unresolved_quotation = profiles["exact_corpus_quotation_coverage"].startswith("unresolved")
    if direct_contamination:
        gate = "contaminated"
    elif unresolved_quotation:
        gate = "unresolved_exact_corpus_quotation_coverage"
    elif skills["classification"] == "unequal_condition-neutral exposure":
        gate = "condition_neutral_or_incidental"
    else:
        gate = "clean"
    return {
        "schema_version": 1,
        "artifact_type": "linguistic_register_v2c_read_only_leakage_audit",
        "audit_id": "semantic-licensed-micro-render-v2c-leakage-audit-2026-08-25",
        "subject": inventory,
        "matcher_packet_reconstruction": matcher,
        "wrapper_and_incidental_features": wrapper_features,
        "profile_and_quotation_audit": profiles,
        "global_skill_exposure": skills,
        "rationale_audit": rationales,
        "interpretation_gate": gate,
        "earned_statement_status": "qualified_by_unresolved_quotation_coverage" if gate == "unresolved_exact_corpus_quotation_coverage" else ("earned" if gate in {"clean", "condition_neutral_or_incidental"} else "not_earned"),
        "earned_statement": "Under a same-family Sol apparatus, semantically accepted outputs retained sufficient intended profile signal for perfect blinded profile traceability across the observed sample set.",
        "authority": "Read-only audit interpretation only; no rerendering, recognizability, behavioral, corpus-selection, or production authority.",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--subject-root", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = build_report(args.subject_root.resolve(), args.inventory.resolve())
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
