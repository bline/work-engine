#!/usr/bin/env python3
"""Close v2c quotation-survival and cue-concentration questions without source redistribution."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import re
import statistics
import subprocess
import unicodedata
from collections import Counter, defaultdict
from html.parser import HTMLParser
from pathlib import Path
from typing import Any, Iterable

import yaml

LEAK_PATH = Path(__file__).with_name("micro_render_v2c_leakage_audit.py")
LEAK_SPEC = importlib.util.spec_from_file_location("micro_render_v2c_leakage_audit_for_closure", LEAK_PATH)
if LEAK_SPEC is None or LEAK_SPEC.loader is None:
    raise RuntimeError(f"cannot load leakage-audit dependency: {LEAK_PATH}")
LEAK = importlib.util.module_from_spec(LEAK_SPEC)
LEAK_SPEC.loader.exec_module(LEAK)


ROOT = Path("skills/linguistic-register-pilot")
RUN_REL = ROOT / "pilot/micro-render-v2c/runs"
EXPECTED_CORPUS_DIGESTS = {
    "gelman-model-criticism": "18e3db1829bdac0945622dc7999fbb74d076398643d51bec2f6042016b7b0dc3",
    "leveson-system-safety": "bf7bfd3b4f38387814a9d9ea920466b97e3f6bcd4e7f7e6421f2a94a5e708541",
    "shaw-engineering-judgment": "c2d35df5253001fbb4d0e95f4932b3b60bcf31e7d9efd70ba8dfd6f89aac9bd5",
}
FEATURE_ORDER = (
    "word_count", "sentence_count", "paragraph_count", "heading_count", "list_marker_count",
    "period_count", "comma_count", "semicolon_count", "colon_count", "question_mark_count",
    "exclamation_count", "dash_count", "parenthesis_count",
)
OPERATOR_ORDER = {"eq": 0, "gt": 1, "le": 2}
CONDITIONS = LEAK.CONDITIONS
WORD_RE = re.compile(r"[\w]+(?:['’][\w]+)?", re.UNICODE)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def tokens(value: str) -> list[str]:
    normalized = unicodedata.normalize("NFKC", value).casefold().replace("’", "'")
    return [word for word in WORD_RE.findall(normalized) if any(character.isalnum() for character in word)]


class VisibleHTML(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.hidden_depth = 0
        self.parts: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.casefold() in {"script", "style", "noscript", "svg"}:
            self.hidden_depth += 1

    def handle_endtag(self, tag: str) -> None:
        if tag.casefold() in {"script", "style", "noscript", "svg"} and self.hidden_depth:
            self.hidden_depth -= 1

    def handle_data(self, data: str) -> None:
        if not self.hidden_depth and data.strip():
            self.parts.append(data)


def extract_source(path: Path) -> tuple[str, dict[str, Any]]:
    if path.suffix.casefold() == ".pdf":
        completed = subprocess.run(
            ["pdftotext", "-enc", "UTF-8", str(path), "-"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )
        if completed.returncode:
            raise ValueError(f"pdftotext failed for {path}: {completed.stderr.decode(errors='replace')}")
        text = completed.stdout.decode("utf-8", errors="replace")
        return text, {"extractor": "pdftotext", "arguments": ["-enc", "UTF-8"]}
    parser = VisibleHTML()
    parser.feed(path.read_text(errors="replace"))
    return "\n".join(parser.parts), {"extractor": "python.html.parser.HTMLParser", "hidden_elements": ["script", "style", "noscript", "svg"]}


def parse_named_paths(values: list[str], label: str) -> dict[str, Path]:
    result = {}
    for value in values:
        name, separator, path = value.partition("=")
        if not separator or not name or not path or name in result:
            raise ValueError(f"invalid {label}: {value}")
        result[name] = Path(path).resolve()
    return result


def load_sources(manifest_paths: dict[str, Path], source_paths: dict[str, Path]) -> tuple[dict[str, Any], dict[str, list[str]]]:
    retained_manifest = {"schema_version": 1, "artifact_type": "linguistic_register_content_addressed_source_manifest", "candidates": []}
    extracted: dict[str, list[str]] = {}
    observed_ids = set()
    for candidate in CONDITIONS:
        if candidate == "neutral-editorial-defaults":
            continue
        manifest_path = manifest_paths[candidate]
        manifest_digest = sha256_file(manifest_path)
        if manifest_digest != EXPECTED_CORPUS_DIGESTS[candidate]:
            raise ValueError(f"corpus manifest digest mismatch for {candidate}")
        manifest = yaml.safe_load(manifest_path.read_text())
        candidate_record = {
            "candidate_id": candidate,
            "corpus_artifact_sha256": manifest_digest,
            "sources": [],
        }
        for source in manifest["sources"]:
            source_id = source["id"]
            path = source_paths[source_id]
            observed_ids.add(source_id)
            content_digest = sha256_file(path)
            if content_digest != source["content_sha256"]:
                raise ValueError(f"source digest mismatch for {source_id}")
            text, extractor = extract_source(path)
            source_tokens = tokens(text)
            extracted[source_id] = source_tokens
            candidate_record["sources"].append({
                "source_id": source_id,
                "source_url": source["public_url"],
                "retrieval_class": "recovered_existing_temporary_cache_exact_digest_match",
                "byte_length": path.stat().st_size,
                "sha256": content_digest,
                "media_type": "application/pdf" if path.suffix.casefold() == ".pdf" else "text/html",
                "license_or_rights_status": source["rights_status"],
                "redistribution_allowed": False,
                "retained_location_class": "external_cache_not_committed",
                "extraction": {
                    **extractor,
                    "extracted_utf8_sha256": sha256_bytes(text.encode()),
                    "normalized_token_count": len(source_tokens),
                },
            })
        retained_manifest["candidates"].append(candidate_record)
    if observed_ids != set(source_paths):
        raise ValueError("source path mapping does not exactly match recovered manifests")
    return retained_manifest, extracted


def positions_by_token(source: list[str]) -> dict[str, list[int]]:
    result: dict[str, list[int]] = defaultdict(list)
    for index, token in enumerate(source):
        result[token].append(index)
    return result


def longest_exact(render: list[str], source: list[str], positions: dict[str, list[int]]) -> dict[str, Any]:
    best = (0, 0, 0)
    for render_start, token in enumerate(render):
        for source_start in positions.get(token, []):
            length = 0
            while render_start + length < len(render) and source_start + length < len(source) and render[render_start + length] == source[source_start + length]:
                length += 1
            if length > best[0]:
                best = (length, render_start, source_start)
    length, render_start, source_start = best
    sequence = " ".join(render[render_start:render_start + length])
    return {
        "length_tokens": length,
        "render_token_offset": render_start,
        "source_token_offset": source_start,
        "sequence_sha256": sha256_bytes(sequence.encode()),
    }


def exact_matches(render: list[str], source: list[str], minimum: int = 5, maximum: int = 12) -> list[dict[str, Any]]:
    matches = []
    for size in range(minimum, min(maximum, len(render)) + 1):
        source_index: dict[tuple[str, ...], list[int]] = defaultdict(list)
        for source_start in range(len(source) - size + 1):
            source_index[tuple(source[source_start:source_start + size])].append(source_start)
        for render_start in range(len(render) - size + 1):
            gram = tuple(render[render_start:render_start + size])
            for source_start in source_index.get(gram, []):
                matches.append({
                    "length_tokens": size,
                    "render_token_offset": render_start,
                    "source_token_offset": source_start,
                    "sequence_sha256": sha256_bytes(" ".join(gram).encode()),
                })
    return matches


def near_verbatim_matches(render: list[str], source: list[str], minimum: int = 12, maximum: int = 30, threshold: float = 0.9) -> list[dict[str, Any]]:
    source_positions = positions_by_token(source)
    candidates = {}
    for size in range(minimum, min(maximum, len(render)) + 1):
        allowed_mismatches = math.floor((1.0 - threshold) * size + 1e-9)
        anchors_needed = allowed_mismatches + 1
        for render_start in range(len(render) - size + 1):
            window = render[render_start:render_start + size]
            ranked = sorted(enumerate(window), key=lambda pair: (len(source_positions.get(pair[1], [])), pair[0]))
            source_starts = {
                source_position - offset
                for offset, token in ranked[:anchors_needed]
                for source_position in source_positions.get(token, [])
                if 0 <= source_position - offset <= len(source) - size
            }
            for source_start in source_starts:
                equal = sum(left == right for left, right in zip(window, source[source_start:source_start + size]))
                ratio = equal / size
                if ratio >= threshold and equal >= 10:
                    key = (render_start, source_start, size)
                    candidates[key] = {
                        "length_tokens": size,
                        "identical_positions": equal,
                        "similarity": ratio,
                        "render_token_offset": render_start,
                        "source_token_offset": source_start,
                        "render_window_sha256": sha256_bytes(" ".join(window).encode()),
                        "source_window_sha256": sha256_bytes(" ".join(source[source_start:source_start + size]).encode()),
                    }
    ordered = sorted(candidates.values(), key=lambda row: (-row["length_tokens"], -row["similarity"], row["render_token_offset"], row["source_token_offset"]))
    maximal = []
    for row in ordered:
        contained = any(
            kept["render_token_offset"] <= row["render_token_offset"]
            and kept["render_token_offset"] + kept["length_tokens"] >= row["render_token_offset"] + row["length_tokens"]
            and kept["source_token_offset"] <= row["source_token_offset"]
            and kept["source_token_offset"] + kept["length_tokens"] >= row["source_token_offset"] + row["length_tokens"]
            for kept in maximal
        )
        if not contained:
            maximal.append(row)
    return maximal


def common_mediated(render: list[str], profile: list[str], source: list[str], minimum: int = 5, maximum: int = 12) -> list[dict[str, Any]]:
    rows = []
    for size in range(minimum, min(maximum, len(render), len(profile), len(source)) + 1):
        profile_index: dict[tuple[str, ...], list[int]] = defaultdict(list)
        source_index: dict[tuple[str, ...], list[int]] = defaultdict(list)
        for index in range(len(profile) - size + 1):
            profile_index[tuple(profile[index:index + size])].append(index)
        for index in range(len(source) - size + 1):
            source_index[tuple(source[index:index + size])].append(index)
        for render_start in range(len(render) - size + 1):
            gram = tuple(render[render_start:render_start + size])
            if gram not in profile_index or gram not in source_index:
                continue
            for profile_start in profile_index[gram]:
                for source_start in source_index[gram]:
                    rows.append({
                        "length_tokens": size,
                        "render_token_offset": render_start,
                        "profile_token_offset": profile_start,
                        "source_token_offset": source_start,
                        "sequence_sha256": sha256_bytes(" ".join(gram).encode()),
                    })
    return rows


def profile_texts(subject: Path) -> dict[str, list[str]]:
    result = {}
    matching_packet = json.loads((subject / RUN_REL / "matching/pass-01/packet.json").read_text())
    matching_key = json.loads((subject / RUN_REL / "matching/pass-01/key.json").read_text())
    reference_by_id = {item["reference_id"]: item for item in matching_packet["anonymous_references"]}
    card_by_condition = {
        condition: reference_by_id[reference_id]
        for reference_id, condition in matching_key["reference_mapping"].items()
    }
    for condition, relative in LEAK.PROFILE_PATHS.items():
        profile = yaml.safe_load((subject / relative).read_text())
        profile_value = json.dumps(profile, ensure_ascii=False)
        card_value = json.dumps(card_by_condition[condition], ensure_ascii=False)
        result[condition] = tokens(profile_value + "\n" + card_value)
    neutral = card_by_condition["neutral-editorial-defaults"]
    result["neutral-editorial-defaults"] = tokens(json.dumps(neutral, ensure_ascii=False))
    return result


def quotation_audit(subject: Path, source_manifest: dict[str, Any], source_tokens: dict[str, list[str]]) -> dict[str, Any]:
    runs = subject / RUN_REL
    unblinding = json.loads((runs / "sealed-unblinding.json").read_text())["samples"]
    profiles = profile_texts(subject)
    source_candidate = {
        source["source_id"]: candidate["candidate_id"]
        for candidate in source_manifest["candidates"] for source in candidate["sources"]
    }
    comparisons = []
    material_exact = []
    material_near = []
    mediated_hits = []
    for sample_id, mapping in sorted(unblinding.items()):
        render_text = json.loads((runs / "samples" / sample_id / "render.json").read_text())["text"]
        render = tokens(render_text)
        for source_id, source in sorted(source_tokens.items()):
            positions = positions_by_token(source)
            exact = exact_matches(render, source)
            near = near_verbatim_matches(render, source)
            mediation = common_mediated(render, profiles[source_candidate[source_id]], source)
            row = {
                "sample_id": sample_id,
                "sample_condition_id": mapping["condition_id"],
                "source_id": source_id,
                "source_candidate_id": source_candidate[source_id],
                "longest_exact": longest_exact(render, source, positions),
                "exact_matches_5_to_12": exact,
                "near_verbatim_matches": near,
                "source_profile_render_mediated_matches": mediation,
            }
            comparisons.append(row)
            material_exact.extend({**match, "sample_id": sample_id, "source_id": source_id} for match in exact if match["length_tokens"] >= 8)
            material_near.extend({**match, "sample_id": sample_id, "source_id": source_id} for match in near)
            mediated_hits.extend({**match, "sample_id": sample_id, "source_id": source_id} for match in mediation)
    return {
        "comparison_count": len(comparisons),
        "comparisons": comparisons,
        "material_exact_matches": material_exact,
        "qualifying_near_verbatim_matches": material_near,
        "source_profile_render_mediated_matches": mediated_hits,
        "gate": "material_overlap" if material_exact or material_near or mediated_hits else "no_material_overlap",
    }


def scalar_features(text: str) -> dict[str, int]:
    metrics = LEAK.sample_metrics(text)
    punctuation = metrics["punctuation_counts"]
    return {
        "word_count": len(tokens(text)),
        "sentence_count": metrics["sentence_count"],
        "paragraph_count": metrics["paragraph_count"],
        "heading_count": metrics["heading_count"],
        "list_marker_count": metrics["list_marker_count"],
        "period_count": punctuation["."],
        "comma_count": punctuation[","],
        "semicolon_count": punctuation[";"],
        "colon_count": punctuation[":"],
        "question_mark_count": punctuation["?"],
        "exclamation_count": punctuation["!"],
        "dash_count": punctuation["—"] + punctuation["-"],
        "parenthesis_count": punctuation["("] + punctuation[")"],
    }


def rule_predict(value: int, operator: str, threshold: float) -> bool:
    return value == threshold if operator == "eq" else (value > threshold if operator == "gt" else value <= threshold)


def metrics_for_predictions(rows: list[dict[str, Any]], condition: str, rule: dict[str, Any]) -> dict[str, Any]:
    tp = fp = tn = fn = 0
    for row in rows:
        predicted = rule_predict(row["features"][rule["feature"]], rule["operator"], rule["threshold"])
        actual = row["condition_id"] == condition
        tp += predicted and actual
        fp += predicted and not actual
        tn += not predicted and not actual
        fn += not predicted and actual
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    specificity = tn / (tn + fp) if tn + fp else 0.0
    return {"precision": precision, "recall": recall, "f1": f1, "specificity": specificity, "tp": tp, "fp": fp, "tn": tn, "fn": fn}


def enumerate_rules(rows: list[dict[str, Any]], condition: str) -> list[dict[str, Any]]:
    results = []
    for feature in FEATURE_ORDER:
        values = sorted({row["features"][feature] for row in rows})
        candidates = [("eq", float(value)) for value in values]
        candidates += [(operator, (left + right) / 2) for left, right in zip(values, values[1:]) for operator in ("gt", "le")]
        for operator, threshold in candidates:
            rule = {"feature": feature, "operator": operator, "threshold": threshold}
            results.append({**rule, **metrics_for_predictions(rows, condition, rule)})
    return results


def best_rule(rules: list[dict[str, Any]]) -> dict[str, Any]:
    return sorted(rules, key=lambda row: (
        -row["f1"], -row["recall"], -row["precision"], FEATURE_ORDER.index(row["feature"]),
        OPERATOR_ORDER[row["operator"]], row["threshold"],
    ))[0]


def rationale_coding(subject: Path) -> dict[str, Any]:
    matching = subject / RUN_REL / "matching"
    rows = []
    for number in range(1, 4):
        directory = matching / f"pass-{number:02d}"
        key = json.loads((directory / "key.json").read_text())
        result = json.loads((directory / "result.json").read_text())
        shaw_reference = next(reference for reference, condition in key["reference_mapping"].items() if condition == "shaw-engineering-judgment")
        for assignment in result["assignments"]:
            if assignment["reference_id"] != shaw_reference:
                continue
            rationale = assignment["rationale"]
            folded = rationale.casefold()
            question = "question" in folded or "interrogative" in folded
            families = []
            if re.search(r"decision first|practical decision|bounded decision|recommendation before|states the recommendation|gives the recommendation|leads with|opens with|begins with", folded):
                families.append("decision_first")
            if re.search(r"compact parallel forms|parallel grammatical", folded):
                families.append("parallel_non_interrogative_forms")
            if re.search(r"promising.but.insufficient|evidential posture|evidential maturity", folded):
                families.append("evidential_maturity")
            if re.search(r"restrained connective|connective progression|resulting contrast", folded):
                families.append("connective_or_contrast_progression")
            if not question:
                category = "absent"
            elif not families:
                category = "sole"
            elif re.search(r"through (?:a |an )?(?:sustained |short |tightly )?parallel|question sequence that organizes|introduces? the review through|develops? .* through .*questions|follows? it with .*questions|uses? a parallel series .*questions to (?:organize|move)", folded):
                category = "primary"
            else:
                category = "secondary"
            rows.append({
                "pass_number": number,
                "text_id": assignment["text_id"],
                "sample_id": key["text_mapping"][assignment["text_id"]],
                "rationale": rationale,
                "interrogative_category": category,
                "other_intended_feature_families": families,
                "other_intended_feature_count": len(families),
            })
    counts = Counter(row["interrogative_category"] for row in rows)
    return {
        "denominator": len(rows),
        "expected_denominator": 12,
        "denominator_verified": len(rows) == 12,
        "category_counts": {category: counts[category] for category in ("sole", "primary", "secondary", "absent")},
        "median_other_intended_feature_count": statistics.median(row["other_intended_feature_count"] for row in rows),
        "rows": rows,
        "limitation": "Rationale coding records reported cue use and is not a mechanistic trace.",
    }


def cue_audit(subject: Path) -> dict[str, Any]:
    runs = subject / RUN_REL
    unblinding = json.loads((runs / "sealed-unblinding.json").read_text())["samples"]
    rows = []
    for sample_id, mapping in sorted(unblinding.items()):
        text = json.loads((runs / "samples" / sample_id / "render.json").read_text())["text"]
        rows.append({"sample_id": sample_id, "condition_id": mapping["condition_id"], "brief_id": mapping["brief_id"], "features": scalar_features(text)})
    assignment_counts = Counter()
    for number in range(1, 4):
        directory = runs / "matching" / f"pass-{number:02d}"
        key = json.loads((directory / "key.json").read_text())
        result = json.loads((directory / "result.json").read_text())
        for assignment in result["assignments"]:
            assignment_counts[key["reference_mapping"][assignment["reference_id"]]] += 1
    baselines = {}
    for condition in CONDITIONS:
        full_rules = enumerate_rules(rows, condition)
        full_best = best_rule(full_rules)
        brief_rows = {
            brief: [row for row in rows if row["brief_id"] == brief]
            for brief in ("alias-removal", "cache-canary")
        }
        cross_brief_perfect_rules = []
        for rule in full_rules:
            definition = {key: rule[key] for key in ("feature", "operator", "threshold")}
            evaluations = {brief: metrics_for_predictions(selected, condition, definition) for brief, selected in brief_rows.items()}
            if all(metrics["precision"] == 1.0 and metrics["recall"] == 1.0 for metrics in evaluations.values()):
                cross_brief_perfect_rules.append({"rule": definition, "by_brief": evaluations})
        transfers = []
        for train_brief, test_brief in (("alias-removal", "cache-canary"), ("cache-canary", "alias-removal")):
            train = [row for row in rows if row["brief_id"] == train_brief]
            test = [row for row in rows if row["brief_id"] == test_brief]
            trained = best_rule(enumerate_rules(train, condition))
            transfers.append({
                "train_brief": train_brief, "test_brief": test_brief,
                "selected_rule": {key: trained[key] for key in ("feature", "operator", "threshold")},
                "train_metrics": {key: trained[key] for key in ("precision", "recall", "f1", "specificity", "tp", "fp", "tn", "fn")},
                "test_metrics": metrics_for_predictions(test, condition, trained),
            })
        baselines[condition] = {
            "full_sample_best": full_best,
            "cross_brief": transfers,
            "selected_training_rules_transfer_perfectly": all(item["test_metrics"]["precision"] == 1.0 and item["test_metrics"]["recall"] == 1.0 for item in transfers),
            "cross_brief_perfect_rules": cross_brief_perfect_rules,
            "cross_brief_perfect": bool(cross_brief_perfect_rules),
            "all_full_sample_rules": full_rules,
        }
    question_rule = {"feature": "question_mark_count", "operator": "gt", "threshold": 0.0}
    shaw_question = metrics_for_predictions(rows, "shaw-engineering-judgment", question_rule)
    shaw_question_by_brief = {
        brief: metrics_for_predictions(selected, "shaw-engineering-judgment", question_rule)
        for brief, selected in brief_rows.items()
    }
    rationales = rationale_coding(subject)
    suitability = {}
    for condition in CONDITIONS:
        if baselines[condition]["cross_brief_perfect"]:
            suitability[condition] = {"decision": "downgrade_behavioral_candidate", "reason": "cross_brief_perfect_one_feature_trivial_rule"}
        elif condition == "shaw-engineering-judgment" and rationales["category_counts"]["sole"] + rationales["category_counts"]["primary"] >= 6 and rationales["median_other_intended_feature_count"] < 2:
            suitability[condition] = {"decision": "downgrade_behavioral_candidate", "reason": "interrogative_cue_concentration"}
        else:
            suitability[condition] = {"decision": "inconclusive", "reason": "no_trivial_downgrade_but_rationale_feature-bundle_retention_not_established_for_this_condition"}
    return {
        "sample_rows": rows,
        "assignment_denominators": dict(sorted(assignment_counts.items())),
        "assignment_denominators_verified_12_each": all(assignment_counts[condition] == 12 for condition in CONDITIONS),
        "condition_baselines": baselines,
        "shaw_question_rule": {
            "rule": question_rule, **shaw_question,
            "by_brief": shaw_question_by_brief,
            "cross_brief_perfect": all(value["precision"] == 1.0 and value["recall"] == 1.0 for value in shaw_question_by_brief.values()),
        },
        "shaw_rationale_coding": rationales,
        "candidate_suitability": suitability,
        "limitation": "One-feature baselines establish descriptive sufficiency on the observed samples; they do not establish causal dependence or behavioral generalization.",
    }


def build_report(subject: Path, manifest_paths: dict[str, Path], source_paths: dict[str, Path]) -> dict[str, Any]:
    source_manifest, source_tokens = load_sources(manifest_paths, source_paths)
    quotation = quotation_audit(subject, source_manifest, source_tokens)
    cue = cue_audit(subject)
    any_downgrade = any(value["decision"] == "downgrade_behavioral_candidate" for value in cue["candidate_suitability"].values())
    if quotation["gate"] == "material_overlap":
        interpretation = "affected_samples_qualified"
    elif any_downgrade:
        interpretation = "traceability_with_candidate_downgrade"
    else:
        interpretation = "strong_retention"
    return {
        "schema_version": 1,
        "artifact_type": "linguistic_register_v2c_forensic_closure",
        "closure_id": "semantic-licensed-micro-render-v2c-forensic-closure-2026-08-25",
        "subject": {
            "v2c_checkpoint_commit_oid": LEAK.SUBJECT_COMMIT,
            "v2c_checkpoint_tree_oid": LEAK.SUBJECT_TREE,
            "source_subject_root_class": "exact_checkpoint_export",
        },
        "content_addressed_source_manifest": source_manifest,
        "quotation_analysis": quotation,
        "cue_concentration": cue,
        "interpretation_gate": interpretation,
        "authority": "Final v2c forensic closure only; no behavioral, independent-recognizability, corpus-selection, or production authority.",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--subject-root", type=Path, required=True)
    parser.add_argument("--corpus-manifest", action="append", default=[])
    parser.add_argument("--source", action="append", default=[])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    manifests = parse_named_paths(args.corpus_manifest, "corpus manifest")
    sources = parse_named_paths(args.source, "source")
    if set(manifests) != set(EXPECTED_CORPUS_DIGESTS):
        raise ValueError("corpus manifests must exactly cover the three candidates")
    report = build_report(args.subject_root.resolve(), manifests, sources)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
