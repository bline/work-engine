#!/usr/bin/env python3
"""Finalize a bounded group-3 preoutcome gate without behavioral execution."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import subprocess
import sys
import unicodedata
from html.parser import HTMLParser
from pathlib import Path
from typing import Any


EXPERIMENT_ID = "linguistic-register-behavioral-pilot-contract-v1-2026-08-26"
GROUP3_ID = "linguistic-register-behavioral-pilot-group-3-v1-2026-08-26"
WORD_RE = re.compile(r"[\w]+(?:['’][\w]+)?", re.UNICODE)
SENTENCE_RE = re.compile(r"(?<=[.!?])(?:[\"')\]]+)?\s+")


class FinalizeError(ValueError):
    pass


def require(value: bool, message: str) -> None:
    if not value:
        raise FinalizeError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected object: {path}")
    return value


def emit(path: Path, value: Any) -> None:
    require(not path.exists(), f"refusing to overwrite: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value))


def tokens(text: str) -> list[str]:
    return [token.casefold() for token in WORD_RE.findall(unicodedata.normalize("NFKC", text))]


class TextExtractor(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.parts: list[str] = []
        self.hidden = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag in {"script", "style"}:
            self.hidden += 1

    def handle_endtag(self, tag: str) -> None:
        if tag in {"script", "style"} and self.hidden:
            self.hidden -= 1

    def handle_data(self, data: str) -> None:
        if not self.hidden:
            self.parts.append(data)


def extract(path: Path, media_type: str) -> str:
    if media_type == "application/pdf":
        return subprocess.run(["pdftotext", "-enc", "UTF-8", str(path), "-"], check=True,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.decode(errors="replace")
    parser = TextExtractor()
    parser.feed(path.read_text(errors="replace"))
    return " ".join(parser.parts)


def exact_match(source: list[str], render: list[str]) -> dict[str, int]:
    source_blob = "\x00" + "\x00".join(source) + "\x00"
    best = {"length": 0, "source_start": 0, "render_start": 0}
    maximum = min(30, len(render))
    for width in range(maximum, 0, -1):
        found = False
        for render_start in range(len(render) - width + 1):
            needle = "\x00" + "\x00".join(render[render_start:render_start + width]) + "\x00"
            character = source_blob.find(needle)
            if character >= 0:
                source_start = source_blob[:character].count("\x00")
                best = {"length": width, "source_start": source_start, "render_start": render_start}
                found = True
                break
        if found:
            break
    return best


def near_matches(source: list[str], render: list[str]) -> list[dict[str, Any]]:
    positions: dict[str, list[int]] = collections.defaultdict(list)
    for index, token in enumerate(source):
        positions[token].append(index)
    matches: list[dict[str, Any]] = []
    for width in range(min(30, len(render)), 11, -1):
        allowed_mismatches = width - max(10, int(0.9 * width + 0.999999))
        anchors_needed = allowed_mismatches + 1
        for render_start in range(len(render) - width + 1):
            window = render[render_start:render_start + width]
            ranked = sorted(range(width), key=lambda i: len(positions.get(window[i], [])))[:anchors_needed]
            offsets: set[int] = set()
            for relative in ranked:
                offsets.update(source_position - relative for source_position in positions.get(window[relative], []))
            for source_start in offsets:
                if source_start < 0 or source_start + width > len(source):
                    continue
                same = sum(a == b for a, b in zip(source[source_start:source_start + width], window))
                if same >= 10 and same / width >= 0.9:
                    matches.append({"length": width, "identical": same, "identity": same / width,
                                    "source_start": source_start, "render_start": render_start})
                    break
            if matches and matches[-1]["length"] == width and matches[-1]["render_start"] == render_start:
                break
        if matches:
            break
    return matches


def quotation(repository: Path, group3: Path, cache: Path) -> dict[str, Any]:
    corpus = load(repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1/corpus-manifest.json")
    profile = load(repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1b/profile.json")
    profile_terms = set(tokens(" ".join(row["description"] for row in profile["features"])))
    manifest = load(group3 / "rendering-manifest-v2.json")
    comparisons = []
    for source in corpus["sources"]:
        suffix = ".pdf" if source["media_type"] == "application/pdf" else ".html"
        source_path = cache / "sources" / f'{source["content_sha256"]}{suffix}'
        require(source_path.is_file() and sha256_file(source_path) == source["content_sha256"],
                f"source object mismatch: {source['source_id']}")
        source_tokens = tokens(extract(source_path, source["media_type"]))
        for artifact in manifest["artifacts"]:
            render_tokens = tokens((group3 / "artifacts" / f'{artifact["artifact_id"]}.md').read_text())
            exact = exact_match(source_tokens, render_tokens)
            mediated = exact_match([token for token in source_tokens if token in profile_terms],
                                   [token for token in render_tokens if token in profile_terms])
            comparisons.append({
                "source_id": source["source_id"], "source_sha256": source["content_sha256"],
                "artifact_id": artifact["artifact_id"], "artifact_sha256": artifact["artifact_sha256"],
                "longest_exact": exact, "material_exact": exact["length"] >= 8,
                "near_verbatim": near_matches(source_tokens, render_tokens),
                "profile_mediated_longest_exact": mediated,
                "material_profile_mediated": mediated["length"] >= 5,
            })
    return {
        "artifact_type": "linguistic_register_group_3_quotation_audit_v1", "schema_version": 1,
        "source_count": len(corpus["sources"]), "artifact_count": len(manifest["artifacts"]),
        "comparison_count": len(comparisons),
        "maximum_exact_tokens": max(row["longest_exact"]["length"] for row in comparisons),
        "material_exact_count": sum(row["material_exact"] for row in comparisons),
        "near_verbatim_count": sum(bool(row["near_verbatim"]) for row in comparisons),
        "material_profile_mediated_count": sum(row["material_profile_mediated"] for row in comparisons),
        "comparisons": comparisons,
    }


def metrics(text: str) -> dict[str, int]:
    lines = text.splitlines()
    paragraphs = [row for row in re.split(r"\n\s*\n", text.strip()) if row]
    return {
        "word_count": len(tokens(text)), "sentence_count": len([row for row in SENTENCE_RE.split(text) if row.strip()]),
        "paragraph_count": len(paragraphs), "line_count": len(lines),
        "heading_count": sum(row.startswith("#") for row in lines),
        "bullet_count": sum(bool(re.match(r"^\s*[-*+]\s", row)) for row in lines),
        "numbered_item_count": sum(bool(re.match(r"^\s*\d+[.)]\s", row)) for row in lines),
        "question_mark_count": text.count("?"), "colon_count": text.count(":"),
        "semicolon_count": text.count(";"), "dash_count": text.count("—") + text.count("–"),
        "parenthesis_count": text.count("(") + text.count(")"),
        "trailing_plus_line_count": sum(row.strip() == "+" for row in lines),
        "path_marker_count": text.count("*** Add File:"), "code_fence_count": text.count("```"),
    }


def scalar_rules(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    found = []
    for field in sorted(rows[0]["metrics"]):
        values = sorted({row["metrics"][field] for row in rows})
        for threshold in ((a + b) / 2 for a, b in zip(values, values[1:])):
            for condition in ("C0", "C1", "C2"):
                actual = [row["condition"] == condition for row in rows]
                for operator in ("gt", "lt"):
                    predicted = [row["metrics"][field] > threshold if operator == "gt" else row["metrics"][field] < threshold for row in rows]
                    if predicted == actual:
                        found.append({"condition": condition, "field": field, "operator": operator, "threshold": threshold})
    return found


def manipulation(result: dict[str, Any], mapping: dict[str, str]) -> dict[str, Any]:
    rows = result["artifacts"]
    require({row["artifact_id"] for row in rows} == set(mapping), "manipulation artifact set mismatch")
    groups = {condition: [row for row in rows if mapping[row["artifact_id"]] == condition]
              for condition in ("C0", "C1", "C2")}
    means = {condition: {"surface": sum(row["surface_uptake"] for row in group) / 2,
                         "discourse": sum(row["discourse_uptake"] for row in group) / 2}
             for condition, group in groups.items()}
    exact = sum(row["assigned_level"] == mapping[row["artifact_id"]] for row in rows)
    order = means["C1"]["surface"] > means["C0"]["surface"] and means["C2"]["discourse"] > means["C1"]["discourse"]
    return {"exact_assignments": exact, "means": means, "ordinal_gate": order,
            "gate": exact >= 5 and order}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, type=Path)
    parser.add_argument("--contract", required=True, type=Path)
    parser.add_argument("--group3", required=True, type=Path)
    parser.add_argument("--source-cache", required=True, type=Path)
    parser.add_argument("--semantic", required=True, type=Path)
    parser.add_argument("--sol", required=True, type=Path)
    parser.add_argument("--claude", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    manifest = load(args.group3 / "rendering-manifest-v2.json")
    key = load(args.group3 / "condition-key.json")
    mapping = {row["artifact_id"]: row["condition"] for row in key["mapping"]}
    require(set(mapping) == {row["artifact_id"] for row in manifest["artifacts"]}, "mapping mismatch")
    for artifact in manifest["artifacts"]:
        require(sha256_file(args.group3 / "artifacts" / f'{artifact["artifact_id"]}.md') == artifact["artifact_sha256"],
                f"publication artifact digest mismatch: {artifact['artifact_id']}")
    semantic = load(args.semantic)
    semantic_rows = semantic["artifacts"]
    require({row["artifact_id"] for row in semantic_rows} == set(mapping), "semantic artifact set mismatch")
    coverage = all(row["canonical_coverage"] for row in semantic_rows)
    equivalence = all(row["semantic_equivalence"] for row in semantic_rows)
    speech = all(row["speech_act_equivalence"] for row in semantic_rows)
    salience = all(row["salience_control"] for row in semantic_rows)
    no_leakage = all(not row["forbidden_leakage"] for row in semantic_rows)
    sol, claude = manipulation(load(args.sol), mapping), manipulation(load(args.claude), mapping)
    quotation_result = quotation(args.repository, args.group3, args.source_cache)
    quotation_gate = not any((quotation_result["material_exact_count"], quotation_result["near_verbatim_count"],
                              quotation_result["material_profile_mediated_count"]))
    cue_rows = [{"artifact_id": artifact["artifact_id"], "condition": mapping[artifact["artifact_id"]],
                 "metrics": metrics((args.group3 / "artifacts" / f'{artifact["artifact_id"]}.md').read_text())}
                for artifact in manifest["artifacts"]]
    rules = scalar_rules(cue_rows)
    cue = {"artifact_type": "linguistic_register_group_3_cue_audit_v1", "schema_version": 1,
           "rows": cue_rows, "perfect_single_scalar_rules": rules, "gate": not rules}
    version = subprocess.run(["codex", "--version"], check=True, stdout=subprocess.PIPE).stdout.decode().strip()
    subject = {
        "artifact_type": "linguistic_register_group_3_subject_verification_v1", "schema_version": 1,
        "local": {"codex_cli_version": version, "model_designation_contract": "gpt-5.6-sol",
                  "reasoning_effort_contract": "medium", "service_tier_contract": "standard_not_fast",
                  "tools_contract": "none"},
        "runtime_only_before_T001": ["effective model receipt", "backend/deployment identifiers",
                                       "exposed sampling identifiers", "weekly usage meter and timestamp",
                                       "fresh-context/no-tools launcher proof"],
        "runtime_only_status": "unresolved_required_not_invented", "local_gate": version == "codex-cli 0.149.1",
    }
    emit(args.output / "quotation-audit.json", quotation_result)
    emit(args.output / "cue-concentration.json", cue)
    emit(args.output / "subject-verification.json", subject)
    manipulation_report = {
        "artifact_type": "linguistic_register_group_3_manipulation_report_v1", "schema_version": 1,
        "perspectives": [
            {"provider_family": "OpenAI/Sol", "independence_claimed": False, "result": sol,
             "raw_sha256": sha256_file(args.sol)},
            {"provider_family": "Anthropic/Claude", "different_provider_from_renderer": True,
             "independence_claimed": False, "result": claude, "raw_sha256": sha256_file(args.claude)},
        ], "human_perspective": "unavailable_unresolved_not_fabricated", "gate": sol["gate"] and claude["gate"],
    }
    emit(args.output / "manipulation-report.json", manipulation_report)
    gates = {
        "six_artifacts_bound": True, "canonical_coverage": coverage,
        "semantic_equivalence": equivalence, "speech_act_equivalence": speech,
        "salience_control": salience and no_leakage,
        "actual_artifact_manipulation": manipulation_report["gate"],
        "no_single_feature_concentration": cue["gate"],
        "subject_prelaunch_verification": subject["local_gate"],
    }
    gate = {
        "artifact_type": "linguistic_register_group_3_preoutcome_gate_v1", "schema_version": 1,
        "experiment_id": EXPERIMENT_ID, "group_3_id": GROUP3_ID,
        "contract_seal_sha256": sha256_file(args.contract / "contract-seal.json"),
        "rendering_manifest_sha256": sha256_file(args.group3 / "rendering-manifest-v2.json"),
        "subject_verification_sha256": sha256_file(args.output / "subject-verification.json"),
        "semantic_review_sha256": sha256_file(args.semantic),
        "manipulation_report_sha256": sha256_file(args.output / "manipulation-report.json"),
        "quotation_audit_sha256": sha256_file(args.output / "quotation-audit.json"),
        "cue_concentration_sha256": sha256_file(args.output / "cue-concentration.json"),
        "gates": gates, "quotation_survival": quotation_gate,
        "overall_gate": all(gates.values()) and quotation_gate,
        "human_audit": "unresolved; model reviews are evidence only",
        "behavioral_outcomes_generated": False,
        "boundary": "Terminal preoutcome group-3 result. Do not execute T001 in this slice.",
    }
    emit(args.output / "group-3-preoutcome-gate.json", gate)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (FinalizeError, OSError, subprocess.SubprocessError) as error:
        print(f"behavioral_pilot_group3_finalize: {error}", file=sys.stderr)
        raise SystemExit(2)
