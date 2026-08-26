#!/usr/bin/env python3
"""Build and validate the expanded Leveson pre-outcome treatment without retaining source text."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import subprocess
import sys
import unicodedata
from collections import defaultdict
from html.parser import HTMLParser
from pathlib import Path
from typing import Any

import yaml


ROOT = Path("skills/linguistic-register-pilot")
RUN = ROOT / "pilot/behavioral-pilot-construction/leveson-expanded-v1"
ROLE = ROOT / "pilot/role/candidate-manifest.yaml"
OLD_PROFILES = {
    "gelman-model-criticism": ROOT / "pilot/candidates/gelman-model-criticism/profile.yaml",
    "shaw-engineering-judgment": ROOT / "pilot/candidates/shaw-engineering-judgment/profile.yaml",
}
DISPOSITIONS = {"realization_only", "semantic_duplicate", "semantic_addition", "uncertain"}
WORD_RE = re.compile(r"[\w]+(?:['’][\w]+)?", re.UNICODE)


class ArtifactError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ArtifactError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def normalized_tokens(value: str) -> list[str]:
    value = unicodedata.normalize("NFKC", value).casefold().replace("’", "'")
    return [token for token in WORD_RE.findall(value) if any(character.isalnum() for character in token)]


class TestimonyHTML(HTMLParser):
    """Capture the unique MIT article panel while excluding page chrome and executable content."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.capture_depth = 0
        self.hidden_depth = 0
        self.parts: list[str] = []
        self.matches = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        classes = set((values.get("class") or "").split())
        if not self.capture_depth and tag.casefold() == "div" and {"panel", "panel--main", "panel--small"} <= classes:
            self.capture_depth = 1
            self.matches += 1
        elif self.capture_depth:
            self.capture_depth += 1
        if self.capture_depth and tag.casefold() in {"script", "style", "noscript", "svg"}:
            self.hidden_depth += 1

    def handle_endtag(self, tag: str) -> None:
        if self.capture_depth and tag.casefold() in {"script", "style", "noscript", "svg"} and self.hidden_depth:
            self.hidden_depth -= 1
        if self.capture_depth:
            self.capture_depth -= 1

    def handle_data(self, data: str) -> None:
        if self.capture_depth and not self.hidden_depth and data.strip():
            self.parts.append(data.strip())


def extract(path: Path) -> tuple[str, dict[str, Any]]:
    if path.suffix.casefold() == ".pdf":
        completed = subprocess.run(
            ["pdftotext", "-enc", "UTF-8", str(path), "-"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )
        require(completed.returncode == 0, f"pdftotext failed for {path}")
        text = completed.stdout.decode("utf-8", errors="replace")
        return text, {"tool": "pdftotext", "arguments": ["-enc", "UTF-8"]}
    parser = TestimonyHTML()
    parser.feed(path.read_text(encoding="utf-8", errors="replace"))
    require(parser.matches == 1, f"expected one testimony panel in {path}, found {parser.matches}")
    return "\n".join(parser.parts), {
        "tool": "python.html.parser.HTMLParser",
        "selector_contract": "div.panel.panel--main.panel--small",
        "matched_elements": parser.matches,
    }


def load_yaml(path: Path) -> dict[str, Any]:
    value = yaml.safe_load(path.read_text())
    require(isinstance(value, dict), f"expected object in {path}")
    return value


def load_selection(subject: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    selection_path = subject / RUN / "source-selection.yaml"
    plan_path = subject / RUN / "construction-plan.yaml"
    selection = load_yaml(selection_path)
    plan = load_yaml(plan_path)
    require(selection["candidate_id"] == plan["candidate_id"], "selection/plan candidate mismatch")
    sources = selection.get("sources", [])
    policy = plan["source_policy"]
    require(policy["minimum_documents"] <= len(sources) <= policy["maximum_documents"], "source count outside plan")
    require(len({row["genre_class"] for row in sources}) >= policy["minimum_genre_classes"], "genre-class threshold failed")
    family_counts = {family: sum(row["genre_family"] == family for row in sources) for family in policy["required_genre_families"]}
    require(all(value >= policy["minimum_documents_per_family"] for value in family_counts.values()), "genre-family balance failed")
    require(all(row["authors"] in (["Nancy Leveson"], ["Nancy G. Leveson"]) for row in sources), "non-solo-authored source")
    return selection, plan


def segment_ranges(size: int, cap: int) -> list[tuple[str, int, int]]:
    if size <= cap:
        third = max(1, math.ceil(size / 3))
        return [
            ("beginning", 0, min(third, size)),
            ("middle", min(third, size), min(2 * third, size)),
            ("end", min(2 * third, size), size),
        ]
    width = cap // 3
    starts = [0, max(0, size // 2 - width // 2), size - width]
    return [(label, start, start + width) for label, start in zip(("beginning", "middle", "end"), starts)]


def prepare(subject: Path, source_dir: Path, packet_output: Path, manifest_output: Path) -> None:
    selection, plan = load_selection(subject)
    require(not packet_output.exists() and not manifest_output.exists(), "refusing to overwrite output")
    packet_sources = []
    manifest_sources = []
    total_tokens = 0
    for row in selection["sources"]:
        path = source_dir / row["filename"]
        require(path.is_file(), f"missing exact source: {path}")
        digest = sha256_file(path)
        require(digest == row["content_sha256"], f"source digest mismatch: {row['id']}")
        text, extractor = extract(path)
        tokens = normalized_tokens(text)
        total_tokens += len(tokens)
        ranges = segment_ranges(len(tokens), plan["profile_extraction"]["balanced_excerpt_tokens_per_document"])
        excerpts = []
        for label, start, end in ranges:
            excerpt = " ".join(tokens[start:end])
            excerpts.append({
                "segment": label,
                "source_token_start": start,
                "source_token_end_exclusive": end,
                "text": excerpt,
                "text_sha256": sha256_bytes(excerpt.encode()),
            })
        packet_sources.append({
            "source_id": row["id"],
            "genre_class": row["genre_class"],
            "genre_family": row["genre_family"],
            "excerpts": excerpts,
        })
        manifest_sources.append({
            "source_id": row["id"],
            "title": row["title"],
            "authors": row["authors"],
            "authorship_basis": row["authorship_basis"],
            "genre_class": row["genre_class"],
            "genre_family": row["genre_family"],
            "public_url": row["public_url"],
            "authority_basis": row["authority_basis"],
            "rights_status": row["rights_status"],
            "content_sha256": digest,
            "byte_length": path.stat().st_size,
            "media_type": "application/pdf" if path.suffix.casefold() == ".pdf" else "text/html",
            "normalized_token_count": len(tokens),
            "extracted_utf8_sha256": sha256_bytes(text.encode()),
            "extractor": extractor,
            "retention": "external_source_bytes_and_extracted_text_not_committed",
            "excerpt_ranges": [{key: value for key, value in excerpt.items() if key != "text"} for excerpt in excerpts],
        })
    minimum, maximum = (plan["source_policy"]["target_normalized_token_range"][key] for key in ("minimum", "maximum"))
    require(minimum <= total_tokens <= maximum, "corpus token total outside frozen range")
    packet = {
        "artifact_type": "linguistic_register_balanced_extraction_packet_v1",
        "schema_version": 1,
        "candidate_id": plan["candidate_id"],
        "task_exposure": "source excerpts only; no role, behavioral task, key, or outcome",
        "sources": packet_sources,
    }
    packet_output.parent.mkdir(parents=True, exist_ok=True)
    packet_output.write_bytes(canonical(packet))
    manifest = {
        "artifact_type": "linguistic_register_expanded_corpus_manifest_v1",
        "schema_version": 1,
        "candidate_id": plan["candidate_id"],
        "source_selection_sha256": sha256_file(subject / RUN / "source-selection.yaml"),
        "construction_plan_sha256": sha256_file(subject / RUN / "construction-plan.yaml"),
        "source_count": len(manifest_sources),
        "normalized_token_total": total_tokens,
        "genre_class_counts": {genre: sum(row["genre_class"] == genre for row in manifest_sources) for genre in sorted({row["genre_class"] for row in manifest_sources})},
        "genre_family_counts": {family: sum(row["genre_family"] == family for row in manifest_sources) for family in sorted({row["genre_family"] for row in manifest_sources})},
        "all_solo_authored": True,
        "sources": manifest_sources,
        "extraction_packet_sha256": sha256_file(packet_output),
        "rights_limitation": "Public readability and institutional hosting do not establish redistribution permission; exact bytes remain outside the repository.",
    }
    manifest_output.parent.mkdir(parents=True, exist_ok=True)
    manifest_output.write_bytes(canonical(manifest))


def semantic_packet(subject: Path, extraction_result: Path, output: Path) -> None:
    require(not output.exists(), "refusing to overwrite output")
    result = json.loads(extraction_result.read_text())
    require(result["packet_sha256"], "missing extraction packet binding")
    role = load_yaml(subject / ROLE)
    packet = {
        "artifact_type": "linguistic_register_semantic_classification_packet_v1",
        "schema_version": 1,
        "feature_ledger": result,
        "frozen_role": role,
        "task_exposure": "feature ledger and frozen role only; no source text, behavioral task, key, or outcome",
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(packet))


def assemble(subject: Path, corpus: Path, extraction_result: Path, semantic_result: Path, output: Path) -> None:
    require(not output.exists(), "refusing to overwrite output")
    plan = load_yaml(subject / RUN / "construction-plan.yaml")
    role_path = subject / ROLE
    extraction = json.loads(extraction_result.read_text())
    semantic = json.loads(semantic_result.read_text())
    classification = {row["feature_id"]: row for row in semantic["classifications"]}
    features = []
    for row in extraction["features"]:
        require(row["id"] in classification, f"missing classification for {row['id']}")
        item = dict(row)
        item["disposition"] = classification[row["id"]]["disposition"]
        item["disposition_rationale"] = classification[row["id"]]["rationale"]
        features.append(item)
    require(set(classification) == {row["id"] for row in features}, "classification feature set mismatch")
    profile = {
        "artifact_type": "linguistic_register_expanded_profile_v1",
        "schema_version": 1,
        "candidate_id": plan["candidate_id"],
        "role_id": load_yaml(role_path)["role_id"],
        "bindings": {
            "construction_plan_sha256": sha256_file(subject / RUN / "construction-plan.yaml"),
            "corpus_manifest_sha256": sha256_file(corpus),
            "role_manifest_sha256": sha256_file(role_path),
            "extraction_result_sha256": sha256_file(extraction_result),
            "semantic_result_sha256": sha256_file(semantic_result),
        },
        "features": features,
        "content_screening": semantic["content_screening"],
        "thresholds": plan["retention_gates"],
        "judgment_provenance": {
            "extractor_model": plan["profile_extraction"]["model"],
            "semantic_classifier_model": plan["semantic_classification"]["model"],
            "weights_assigned_before_semantic_classification": True,
            "performed_before_behavioral_outcomes": True,
            "future_task_or_key_exposure": False,
        },
        "limitations": extraction.get("limitations", []) + semantic.get("limitations", []),
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(profile))


def longest_exact(left: list[str], right: list[str]) -> dict[str, Any]:
    positions: dict[str, list[int]] = defaultdict(list)
    for index, token in enumerate(right):
        positions[token].append(index)
    best = (0, 0, 0)
    for left_start, token in enumerate(left):
        for right_start in positions.get(token, []):
            length = 0
            while left_start + length < len(left) and right_start + length < len(right) and left[left_start + length] == right[right_start + length]:
                length += 1
            if length > best[0]:
                best = (length, left_start, right_start)
    length, left_start, right_start = best
    return {"length_tokens": length, "profile_token_offset": left_start, "source_token_offset": right_start,
            "sequence_sha256": sha256_bytes(" ".join(left[left_start:left_start + length]).encode())}


def near_matches(left: list[str], right: list[str], minimum: int, maximum: int, threshold: float, identical_minimum: int) -> list[dict[str, Any]]:
    positions: dict[str, list[int]] = defaultdict(list)
    for index, token in enumerate(right):
        positions[token].append(index)
    rows = []
    for size in range(minimum, min(maximum, len(left), len(right)) + 1):
        mismatches = math.floor((1 - threshold) * size + 1e-9)
        for left_start in range(len(left) - size + 1):
            window = left[left_start:left_start + size]
            anchors = sorted(enumerate(window), key=lambda pair: (len(positions.get(pair[1], [])), pair[0]))[:mismatches + 1]
            starts = {source_index - offset for offset, token in anchors for source_index in positions.get(token, []) if 0 <= source_index - offset <= len(right) - size}
            for right_start in starts:
                identical = sum(a == b for a, b in zip(window, right[right_start:right_start + size]))
                if identical >= identical_minimum and identical / size >= threshold:
                    rows.append({"length_tokens": size, "identical_positions": identical, "profile_token_offset": left_start,
                                 "source_token_offset": right_start, "profile_window_sha256": sha256_bytes(" ".join(window).encode()),
                                 "source_window_sha256": sha256_bytes(" ".join(right[right_start:right_start + size]).encode())})
    return rows


def audit(subject: Path, source_dir: Path, corpus_path: Path, profile_path: Path, output: Path) -> None:
    require(not output.exists(), "refusing to overwrite output")
    selection, plan = load_selection(subject)
    corpus = json.loads(corpus_path.read_text())
    profile = json.loads(profile_path.read_text())
    # Audit every model-authored feature field that is retained in Git, not just
    # treatment descriptions. Evidence observations and classification
    # rationales can also accidentally preserve source wording.
    profile_text = "\n".join(
        row["description"] + "\n"
        + "\n".join(item["observation"] for item in row["evidence"]) + "\n"
        + row["disposition_rationale"]
        for row in profile["features"]
    )
    profile_tokens = normalized_tokens(profile_text)
    rows = []
    material_exact = 0
    near_total = 0
    for source in selection["sources"]:
        path = source_dir / source["filename"]
        require(sha256_file(path) == source["content_sha256"], f"source digest mismatch: {source['id']}")
        text, _ = extract(path)
        source_tokens = normalized_tokens(text)
        exact = longest_exact(profile_tokens, source_tokens)
        near = near_matches(profile_tokens, source_tokens,
                            plan["quotation_gate"]["near_verbatim_minimum_tokens"],
                            plan["quotation_gate"]["near_verbatim_maximum_tokens"],
                            plan["quotation_gate"]["near_verbatim_minimum_positional_identity"],
                            plan["quotation_gate"]["near_verbatim_minimum_identical_positions"])
        if exact["length_tokens"] >= plan["quotation_gate"]["material_exact_minimum_tokens"]:
            material_exact += 1
        near_total += len(near)
        rows.append({"source_id": source["id"], "source_sha256": source["content_sha256"], "longest_exact": exact, "near_verbatim_matches": near})
    dispositions = {value: [row for row in profile["features"] if row["disposition"] == value] for value in sorted(DISPOSITIONS)}
    total_weight = sum(row["distinctiveness_weight"] for row in profile["features"])
    retained = dispositions["realization_only"]
    retained_weight = sum(row["distinctiveness_weight"] for row in retained)
    cross = sum(row["cross_family"] for row in retained)
    evidence_minimum = min((len({item["source_id"] for item in row["evidence"]}) for row in retained), default=0)
    gates = {
        "retained_feature_count": len(retained) >= plan["retention_gates"]["minimum_retained_features"],
        "weighted_retention": retained_weight / total_weight >= plan["retention_gates"]["minimum_weighted_retention"] if total_weight else False,
        "cross_family_retained_features": cross >= plan["retention_gates"]["minimum_cross_family_retained_features"],
        "evidence_documents_per_retained_feature": evidence_minimum >= plan["retention_gates"]["minimum_evidence_documents_per_retained_feature"],
        "quotation_survival": material_exact == 0 and near_total == 0,
    }
    report = {
        "artifact_type": "linguistic_register_expanded_profile_audit_v1", "schema_version": 1,
        "bindings": {"corpus_manifest_sha256": sha256_file(corpus_path), "profile_sha256": sha256_file(profile_path)},
        "corpus_normalized_token_total": corpus["normalized_token_total"],
        "metrics": {"feature_count": len(profile["features"]), "counts_by_disposition": {key: len(value) for key, value in dispositions.items()},
                    "weighted_retention": round(retained_weight / total_weight, 10), "cross_family_retained_features": cross,
                    "minimum_retained_evidence_document_count": evidence_minimum,
                    "material_exact_source_matches": material_exact, "near_verbatim_source_matches": near_total},
        "gates": gates, "overall_gate": all(gates.values()), "quotation_comparisons": rows,
        "audited_profile_fields": ["features[].description", "features[].evidence[].observation", "features[].disposition_rationale"],
        "limitations": ["Deterministic quotation checks cannot prove absence of every possible paraphrase or memorized influence."],
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(report))


def card_features(profile: dict[str, Any], count: int) -> list[dict[str, Any]]:
    retained = [row for row in profile["features"] if row["disposition"] == "realization_only"]
    retained.sort(key=lambda row: (-row["distinctiveness_weight"], row["id"]))
    require(len(retained) >= count, "profile has too few retained features for equalized card")
    return [{"layer": row["layer"], "category": row["category"], "description": row["description"],
             "salience_weight": row["distinctiveness_weight"]} for row in retained[:count]]


def recognition_packet(subject: Path, expanded_profile: Path, output: Path, key_output: Path) -> None:
    require(not output.exists() and not key_output.exists(), "refusing to overwrite output")
    plan = load_yaml(subject / RUN / "construction-plan.yaml")
    count = plan["recognizability_precheck"]["equalized_feature_count"]
    profiles = {"leveson-system-safety-expanded-v1": json.loads(expanded_profile.read_text())}
    profiles.update({candidate: load_yaml(subject / path) for candidate, path in OLD_PROFILES.items()})
    neutral = [
        {"layer": "surface", "category": "paragraph-rhythm", "description": "Keep the response focused on the single review decision.", "salience_weight": 3},
        {"layer": "surface", "category": "connective-patterns", "description": "Use ordinary transitions between supplied propositions.", "salience_weight": 3},
        {"layer": "surface", "category": "syntax", "description": "Prefer direct sentences with clear subjects and actions.", "salience_weight": 3},
        {"layer": "discourse", "category": "qualification-patterns", "description": "State a supplied evidence limitation once without additional evaluation.", "salience_weight": 3},
        {"layer": "surface", "category": "conclusion-patterns", "description": "End with a concise restatement of the supplied decision.", "salience_weight": 3},
    ]
    mapping = {"R01": "shaw-engineering-judgment", "R02": "neutral-editorial-defaults", "R03": "leveson-system-safety-expanded-v1", "R04": "gelman-model-criticism"}
    cards = []
    for card_id, candidate in mapping.items():
        features = neutral if candidate == "neutral-editorial-defaults" else card_features(profiles[candidate], count)
        cards.append({"anonymous_card_id": card_id, "features": features})
    packet = {"artifact_type": "linguistic_register_expanded_recognizability_packet_v1", "schema_version": 1,
              "candidate_practices": ["Bayesian model criticism", "system-safety review and causal-boundary analysis", "software-engineering judgment", "neutral professional editing"],
              "cards": cards, "task": "Assign each anonymous card to exactly one candidate practice and explain the profile cues used."}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(packet))
    key_output.write_bytes(canonical({"packet_sha256": sha256_file(output), "mapping": mapping}))


def shallow_metrics(features: list[dict[str, Any]]) -> dict[str, int]:
    text = " ".join(row["description"] for row in features)
    return {
        "word_count": len(normalized_tokens(text)), "sentence_count": len(re.findall(r"[.!?]+", text)),
        "question_mark_count": text.count("?"), "colon_count": text.count(":"), "semicolon_count": text.count(";"),
        "dash_count": len(re.findall(r"[-–—]", text)), "parenthesis_count": text.count("(") + text.count(")"),
        "layer_count_surface": sum(row["layer"] == "surface" for row in features),
        "layer_count_discourse": sum(row["layer"] == "discourse" for row in features),
    }


def cue_baseline(packet_path: Path, key_path: Path, output: Path) -> None:
    require(not output.exists(), "refusing to overwrite output")
    packet = json.loads(packet_path.read_text())
    key = json.loads(key_path.read_text())
    rows = []
    for card in packet["cards"]:
        candidate = key["mapping"][card["anonymous_card_id"]]
        features = card["features"]
        for replica in range(3):
            rotated = features[replica:] + features[:replica]
            rows.append({"candidate_id": candidate, "replica": replica + 1, "metrics": shallow_metrics(rotated)})
    target = "leveson-system-safety-expanded-v1"
    perfect = []
    metric_names = sorted(rows[0]["metrics"])
    for name in metric_names:
        values = sorted({row["metrics"][name] for row in rows})
        thresholds = values + [(a + b) / 2 for a, b in zip(values, values[1:])]
        for operator in ("eq", "gt", "le"):
            for threshold in thresholds:
                predictions = []
                for row in rows:
                    value = row["metrics"][name]
                    predicted = value == threshold if operator == "eq" else value > threshold if operator == "gt" else value <= threshold
                    predictions.append(predicted == (row["candidate_id"] == target))
                if all(predictions):
                    perfect.append({"metric": name, "operator": operator, "threshold": threshold})
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical({"artifact_type": "linguistic_register_expanded_cue_baseline_v1", "schema_version": 1,
                                  "packet_sha256": sha256_file(packet_path), "replicas": rows,
                                  "cross_replica_perfect_leveson_rules": perfect, "flag": bool(perfect),
                                  "limitation": "Feature-order replicas test shallow card statistics, not rendered prose or semantic cue sufficiency."}))


def score_recognition(packet_path: Path, key_path: Path, result_path: Path, output: Path) -> None:
    require(not output.exists(), "refusing to overwrite output")
    packet = json.loads(packet_path.read_text())
    key = json.loads(key_path.read_text())
    result = json.loads(result_path.read_text())
    require(result["packet_sha256"] == sha256_file(packet_path) == key["packet_sha256"], "recognition packet binding mismatch")
    practice_to_candidate = {
        "Bayesian model criticism": "gelman-model-criticism",
        "system-safety review and causal-boundary analysis": "leveson-system-safety-expanded-v1",
        "software-engineering judgment": "shaw-engineering-judgment",
        "neutral professional editing": "neutral-editorial-defaults",
    }
    assignments = result["assignments"]
    require({row["anonymous_card_id"] for row in assignments} == set(key["mapping"]), "recognition card set mismatch")
    require(len({row["candidate_practice"] for row in assignments}) == 4, "recognition assignments are not bijective")
    rows = []
    for row in assignments:
        predicted = practice_to_candidate[row["candidate_practice"]]
        expected = key["mapping"][row["anonymous_card_id"]]
        rows.append({"anonymous_card_id": row["anonymous_card_id"], "predicted_candidate_id": predicted,
                     "expected_candidate_id": expected, "correct": predicted == expected,
                     "confidence": row["confidence"], "rationale": row["rationale"]})
    leveson = next(row for row in rows if row["expected_candidate_id"] == "leveson-system-safety-expanded-v1")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical({
        "artifact_type": "linguistic_register_expanded_recognizability_report_v1", "schema_version": 1,
        "bindings": {"packet_sha256": sha256_file(packet_path), "key_sha256": sha256_file(key_path), "result_sha256": sha256_file(result_path)},
        "assignments": rows, "total_correct": sum(row["correct"] for row in rows),
        "expanded_leveson_assignment_correct": leveson["correct"],
        "gate": leveson["correct"],
        "interpretation": "same-family equal-feature-count profile-card traceability only",
        "limitations": result.get("limitations", []) + ["The shallow baseline independently found card length sufficient to identify expanded Leveson."],
    }))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prep = sub.add_parser("prepare")
    prep.add_argument("--subject", type=Path, default=Path.cwd())
    prep.add_argument("--source-dir", type=Path, required=True)
    prep.add_argument("--packet-output", type=Path, required=True)
    prep.add_argument("--manifest-output", type=Path, required=True)
    sem = sub.add_parser("semantic-packet")
    sem.add_argument("--subject", type=Path, default=Path.cwd())
    sem.add_argument("--extraction-result", type=Path, required=True)
    sem.add_argument("--output", type=Path, required=True)
    ass = sub.add_parser("assemble")
    ass.add_argument("--subject", type=Path, default=Path.cwd())
    ass.add_argument("--corpus", type=Path, required=True)
    ass.add_argument("--extraction-result", type=Path, required=True)
    ass.add_argument("--semantic-result", type=Path, required=True)
    ass.add_argument("--output", type=Path, required=True)
    aud = sub.add_parser("audit")
    aud.add_argument("--subject", type=Path, default=Path.cwd())
    aud.add_argument("--source-dir", type=Path, required=True)
    aud.add_argument("--corpus", type=Path, required=True)
    aud.add_argument("--profile", type=Path, required=True)
    aud.add_argument("--output", type=Path, required=True)
    rec = sub.add_parser("recognition-packet")
    rec.add_argument("--subject", type=Path, default=Path.cwd())
    rec.add_argument("--expanded-profile", type=Path, required=True)
    rec.add_argument("--output", type=Path, required=True)
    rec.add_argument("--key-output", type=Path, required=True)
    cue = sub.add_parser("cue-baseline")
    cue.add_argument("--packet", type=Path, required=True)
    cue.add_argument("--key", type=Path, required=True)
    cue.add_argument("--output", type=Path, required=True)
    score = sub.add_parser("score-recognition")
    score.add_argument("--packet", type=Path, required=True)
    score.add_argument("--key", type=Path, required=True)
    score.add_argument("--result", type=Path, required=True)
    score.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.subject.resolve(), args.source_dir.resolve(), args.packet_output.resolve(), args.manifest_output.resolve())
    elif args.command == "semantic-packet":
        semantic_packet(args.subject.resolve(), args.extraction_result.resolve(), args.output.resolve())
    elif args.command == "assemble":
        assemble(args.subject.resolve(), args.corpus.resolve(), args.extraction_result.resolve(), args.semantic_result.resolve(), args.output.resolve())
    elif args.command == "audit":
        audit(args.subject.resolve(), args.source_dir.resolve(), args.corpus.resolve(), args.profile.resolve(), args.output.resolve())
    elif args.command == "recognition-packet":
        recognition_packet(args.subject.resolve(), args.expanded_profile.resolve(), args.output.resolve(), args.key_output.resolve())
    elif args.command == "cue-baseline":
        cue_baseline(args.packet.resolve(), args.key.resolve(), args.output.resolve())
    else:
        score_recognition(args.packet.resolve(), args.key.resolve(), args.result.resolve(), args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ArtifactError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"expanded_leveson_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
