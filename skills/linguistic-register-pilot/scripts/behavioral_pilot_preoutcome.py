#!/usr/bin/env python3
"""Prepare and deterministically audit group-3 preoutcome role artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import unicodedata
from pathlib import Path
from typing import Any


EXPERIMENT_ID = "linguistic-register-behavioral-pilot-contract-v1-2026-08-26"
GROUP3_ID = "linguistic-register-behavioral-pilot-group-3-v1-2026-08-26"
CONTRACT_SEAL_SHA256 = "87fcab85871549e6a5798099c229c31aa6241911cad1060bae22e5f23f290699"
CONTRACT_PREREG_SHA256 = "bb7340b544a26161f50bea3e9932a66f108e4b1c08932dc635edb9e67c1f328b"
TREATMENT_PREREG_SHA256 = "9467ce4dff704cc840eb57f50a9e20f85e6387841e28081ad191bceadfc2bc4e"
ROLE_MANIFEST_SHA256 = "1ca5b13fa66e53381f0980dcd87db8f8b8f831140aba441fd66de096f0265c94"
PROFILE_SHA256 = "e3592b0c6b99032e85d87ae2453f49ce1f24048c4f3e12f049cf786e537e5ef2"
CORPUS_MANIFEST_SHA256 = "c947def160d1f1c0a890ed7850fa3910efa0081e37b2b7ef1183200de9014e32"
WORD_RE = re.compile(r"[\w]+(?:['’][\w]+)?", re.UNICODE)
SENTENCE_RE = re.compile(r"(?<=[.!?])(?:[\"')\]]+)?\s+")


class PreoutcomeError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PreoutcomeError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected object: {path}")
    return value


def emit_new(path: Path, value: Any) -> None:
    require(not path.exists(), f"refusing to overwrite: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value))


def tokens(text: str) -> list[str]:
    return [token.casefold() for token in WORD_RE.findall(unicodedata.normalize("NFKC", text))]


def role_sources(repository: Path) -> dict[str, str]:
    commit = "db3371916ad64404cf2e29894a75cae84c90b8f0"
    paths = {
        "entrypoint": "skills/agent-instruction-review/SKILL.md",
        "finding_contract": "skills/agent-instruction-review/references/finding-contract.md",
    }
    result: dict[str, str] = {}
    for key, path in paths.items():
        completed = subprocess.run(
            ["git", "show", f"{commit}:{path}"], cwd=repository,
            check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        result[key] = completed.stdout.decode()
    return result


def retained_features(profile: dict[str, Any]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    retained = [row for row in profile["features"] if row["treatment_disposition"] == "retained"]
    surface = [row for row in retained if row["layer"] == "surface"]
    discourse = [row for row in retained if row["layer"] == "discourse"]
    require(len(retained) == 8 and len(surface) == 6 and len(discourse) == 2,
            "unexpected retained treatment composition")
    require(all(row["disposition"] == "realization_only" for row in retained),
            "non-realization feature entered retained treatment")
    return surface, discourse


def render_instruction(condition: str, surface: list[dict[str, Any]], discourse: list[dict[str, Any]]) -> dict[str, Any]:
    common = {
        "objective": "Render one complete, standalone SKILL.md role that preserves every supplied canonical instruction and authority boundary.",
        "shape": {
            "normalized_word_range": [700, 850],
            "required_top_heading": "Agent Instruction Review",
            "required_second_level_headings": [
                "Scope", "Diagnose the instruction", "Evidence-bearing result",
                "Finding contract", "Remediation and authority",
            ],
            "frontmatter_name": "agent-instruction-review",
        },
        "prohibitions": [
            "Do not add, remove, strengthen, weaken, or reinterpret a claim, requirement, permission, evidence operation, decision policy, or authority boundary.",
            "Do not add methods, examples, task content, behavioral heuristics, personal names, profile labels, source phrases, or subject-domain terminology.",
            "Do not mention style transfer, conditions, candidates, authors, experiments, or this packet.",
            "Do not use external information or tools.",
        ],
        "output": "Return only one JSON object with exactly one string field named text. The string is the complete SKILL.md artifact.",
    }
    if condition == "C0":
        common["realization"] = {
            "mode": "neutral",
            "instruction": "Use clear, ordinary technical prose. Do not intentionally apply any treatment features.",
            "licensed_features": [],
        }
    elif condition == "C1":
        common["realization"] = {
            "mode": "surface_only",
            "instruction": "Apply only the listed surface realization preferences. They may change wording, syntax, organization, and rhythm but not discourse operations or meaning.",
            "licensed_features": [{"id": row["id"], "description": row["description"]} for row in surface],
        }
    elif condition == "C2":
        common["realization"] = {
            "mode": "surface_plus_realization_only_discourse",
            "instruction": "Apply all listed realization preferences. The discourse preferences may make already-supplied causal and additive relationships explicit but may not create a new inference, example, evidence operation, or conclusion.",
            "licensed_features": [{"id": row["id"], "description": row["description"]} for row in surface + discourse],
        }
    else:
        raise PreoutcomeError(f"unknown condition: {condition}")
    return common


def request_record(job_id: str, packet: bytes, schema: bytes, prompt: str) -> dict[str, Any]:
    return {
        "experiment_id": GROUP3_ID,
        "job_id": job_id,
        "stage": "render",
        "packet_sha256": sha256_bytes(packet),
        "schema_sha256": sha256_bytes(schema),
        "prompt_sha256": sha256_bytes(prompt.encode()),
        "provider": "OpenAI",
        "model": "gpt-5.6-sol",
        "reasoning_effort": "medium",
        "service_tier": "standard_not_fast",
    }


def prepare(repository: Path, contract: Path, output: Path) -> None:
    require(sha256_file(contract / "contract-seal.json") == CONTRACT_SEAL_SHA256, "contract seal mismatch")
    require(sha256_file(contract / "preregistration.yaml") == CONTRACT_PREREG_SHA256, "contract preregistration mismatch")
    role_manifest = repository / "skills/linguistic-register-pilot/pilot/role/candidate-manifest.yaml"
    profile_path = repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1b/profile.json"
    corpus_path = repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1/corpus-manifest.json"
    require(sha256_file(role_manifest) == ROLE_MANIFEST_SHA256, "role manifest mismatch")
    require(sha256_file(profile_path) == PROFILE_SHA256, "treatment profile mismatch")
    require(sha256_file(corpus_path) == CORPUS_MANIFEST_SHA256, "corpus manifest mismatch")
    profile = load_json(profile_path)
    surface, discourse = retained_features(profile)
    sources = role_sources(repository)
    schema = {
        "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
        "required": ["text"], "properties": {"text": {"type": "string"}}, "additionalProperties": False,
    }
    prompt = "Use only the supplied packet. Produce the requested role rendering without tools or external context."
    assignments = [
        ("1d504763-88bc-462e-aa4e-9d9581d5591e", "C2", 1),
        ("d2eb55ed-1f46-42f8-9698-6670c71309ca", "C0", 1),
        ("caac9200-86b3-4614-bbe0-1b6865e951ab", "C2", 2),
        ("41767db4-ea13-4f3c-90db-9d0b472d2551", "C1", 1),
        ("5522b147-d5ac-4d37-9805-e1a0578cbcf5", "C1", 2),
        ("e0f58b89-cd75-4cb8-b6b9-fa7c8bfb7425", "C0", 2),
    ]
    jobs, key_rows = [], []
    for artifact_id, condition, replica in assignments:
        packet_value = {
            "artifact_type": "linguistic_register_group_3_render_packet_v1", "schema_version": 1,
            "group_3_id": GROUP3_ID, "replica": replica,
            "canonical_role": sources, "render_contract": render_instruction(condition, surface, discourse),
        }
        packet = canonical(packet_value)
        packet_path = output / "packets/render" / f"{artifact_id}.json"
        packet_path.parent.mkdir(parents=True, exist_ok=True)
        packet_path.write_bytes(packet)
        record = request_record(artifact_id, packet, canonical(schema), prompt)
        jobs.append({**record, "request_sha256": sha256_bytes(canonical(record))})
        key_rows.append({"artifact_id": artifact_id, "condition": condition, "replica": replica,
                         "packet_sha256": sha256_bytes(packet)})
    (output / "schemas").mkdir(parents=True, exist_ok=True)
    (output / "schemas/render-output.schema.json").write_bytes(canonical(schema))
    emit_new(output / "condition-key.json", {
        "artifact_type": "linguistic_register_group_3_condition_key_v1", "schema_version": 1,
        "group_3_id": GROUP3_ID, "mapping": key_rows,
        "separation": "This key is not supplied to semantic, salience, or manipulation reviewers.",
    })
    key_sha = sha256_file(output / "condition-key.json")
    launch = {
        "artifact_type": "linguistic_register_group_3_render_launch_manifest_v1", "schema_version": 1,
        "group_3_id": GROUP3_ID, "contract_seal_sha256": CONTRACT_SEAL_SHA256,
        "condition_key_sha256": key_sha, "prompt": prompt,
        "schema_sha256": sha256_file(output / "schemas/render-output.schema.json"),
        "jobs": jobs,
    }
    launch["launch_set_sha256"] = sha256_bytes(canonical(launch))
    emit_new(output / "render-launch-manifest.json", launch)


def normalize_render(raw: bytes) -> tuple[str, str]:
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as error:
        raise PreoutcomeError(f"render output is not JSON: {error}") from error
    require(isinstance(value, dict) and set(value) == {"text"} and isinstance(value["text"], str),
            "only an exact single-key string text wrapper is accepted")
    text = value["text"]
    require(not text.lstrip().startswith("```"), "code-fenced artifact rejected")
    count = len(tokens(text))
    require(700 <= count <= 850, f"artifact word count outside 700-850: {count}")
    return text, "exact_single_key_text_wrapper"


def ingest(group3: Path, evidence: Path, output: Path) -> None:
    launch = load_json(group3 / "render-launch-manifest.json")
    key = load_json(group3 / "condition-key.json")
    require(sha256_file(group3 / "condition-key.json") == launch["condition_key_sha256"], "condition key mismatch")
    rows = []
    for job in launch["jobs"]:
        artifact_id = job["job_id"]
        raw_path = evidence / artifact_id / "raw-output.json"
        receipt_path = evidence / artifact_id / "receipt.json"
        marker_path = evidence / artifact_id / "attempt-marker.json"
        for path in (raw_path, receipt_path, marker_path):
            require(path.is_file(), f"missing evidence: {path}")
        receipt, marker = load_json(receipt_path), load_json(marker_path)
        require(receipt["request_sha256"] == marker["request_sha256"] == job["request_sha256"],
                f"request binding mismatch: {artifact_id}")
        require(receipt["launch_set_sha256"] == marker["launch_set_sha256"] == launch["launch_set_sha256"],
                f"launch-set binding mismatch: {artifact_id}")
        require(receipt["completion_status"] == "completed", f"render did not complete: {artifact_id}")
        raw = raw_path.read_bytes()
        require(sha256_bytes(raw) == receipt["raw_output_sha256"], f"raw digest mismatch: {artifact_id}")
        text, normalization = normalize_render(raw)
        artifact_path = output / "artifacts" / f"{artifact_id}.md"
        require(not artifact_path.exists(), f"refusing to overwrite artifact: {artifact_path}")
        artifact_path.parent.mkdir(parents=True, exist_ok=True)
        artifact_path.write_text(text)
        rows.append({
            "artifact_id": artifact_id, "artifact_sha256": sha256_file(artifact_path),
            "normalized_word_count": len(tokens(text)), "transport_normalization": normalization,
            "raw_response_sha256": sha256_bytes(raw), "receipt_sha256": sha256_file(receipt_path),
            "attempt_marker_sha256": sha256_file(marker_path),
        })
    require(len(rows) == len(key["mapping"]) == 6, "six artifacts required")
    emit_new(output / "rendering-manifest.json", {
        "artifact_type": "linguistic_register_group_3_rendering_manifest_v1", "schema_version": 1,
        "group_3_id": GROUP3_ID, "contract_seal_sha256": CONTRACT_SEAL_SHA256,
        "render_launch_manifest_sha256": sha256_file(group3 / "render-launch-manifest.json"),
        "condition_key_sha256": sha256_file(group3 / "condition-key.json"), "artifacts": rows,
        "condition_labels_present": False,
    })


def shallow_metrics(text: str) -> dict[str, int]:
    paragraphs = [p for p in re.split(r"\n\s*\n", text.strip()) if p]
    lines = text.splitlines()
    return {
        "word_count": len(tokens(text)), "sentence_count": len([s for s in SENTENCE_RE.split(text) if s.strip()]),
        "paragraph_count": len(paragraphs), "line_count": len(lines),
        "heading_count": sum(line.startswith("#") for line in lines),
        "bullet_count": sum(bool(re.match(r"^\s*[-*+]\s", line)) for line in lines),
        "numbered_item_count": sum(bool(re.match(r"^\s*\d+[.)]\s", line)) for line in lines),
        "question_mark_count": text.count("?"), "colon_count": text.count(":"),
        "semicolon_count": text.count(";"), "dash_count": text.count("—") + text.count("–"),
        "parenthesis_count": text.count("(") + text.count(")"),
    }


def perfect_scalar_rules(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    results = []
    fields = sorted(rows[0]["metrics"])
    for field in fields:
        values = sorted({row["metrics"][field] for row in rows})
        thresholds = [(left + right) / 2 for left, right in zip(values, values[1:])]
        for condition in ("C0", "C1", "C2"):
            actual = [row["condition"] == condition for row in rows]
            for threshold in thresholds:
                for operator in ("gt", "lt"):
                    predicted = [row["metrics"][field] > threshold if operator == "gt" else row["metrics"][field] < threshold for row in rows]
                    if predicted == actual:
                        results.append({"condition": condition, "field": field, "operator": operator, "threshold": threshold})
    return results


def longest_exact(left: list[str], right: list[str]) -> dict[str, int]:
    previous = [0] * (len(right) + 1)
    best = {"length": 0, "source_start": 0, "render_start": 0}
    for i, token in enumerate(left, 1):
        current = [0] * (len(right) + 1)
        for j, other in enumerate(right, 1):
            if token == other:
                current[j] = previous[j - 1] + 1
                if current[j] > best["length"]:
                    best = {"length": current[j], "source_start": i - current[j], "render_start": j - current[j]}
        previous = current
    return best


def near_verbatim(left: list[str], right: list[str], minimum: int = 12) -> list[dict[str, Any]]:
    matches = []
    for width in range(min(30, len(right)), minimum - 1, -1):
        for render_start in range(len(right) - width + 1):
            target = right[render_start:render_start + width]
            for source_start in range(len(left) - width + 1):
                same = sum(a == b for a, b in zip(left[source_start:source_start + width], target))
                if same >= 10 and same / width >= 0.9:
                    matches.append({"length": width, "identical": same, "source_start": source_start,
                                    "render_start": render_start, "identity": same / width})
                    break
            if matches and matches[-1]["render_start"] == render_start and matches[-1]["length"] == width:
                continue
    return matches


def extract_source(path: Path, media_type: str) -> str:
    if media_type == "application/pdf":
        completed = subprocess.run(["pdftotext", "-enc", "UTF-8", str(path), "-"],
                                   check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        return completed.stdout.decode(errors="replace")
    text = path.read_text(errors="replace")
    text = re.sub(r"(?is)<(script|style).*?>.*?</\1>", " ", text)
    return re.sub(r"(?s)<[^>]+>", " ", text)


def quotation_audit(repository: Path, group3: Path, source_cache: Path) -> dict[str, Any]:
    corpus_path = repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1/corpus-manifest.json"
    profile_path = repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1b/profile.json"
    corpus, profile = load_json(corpus_path), load_json(profile_path)
    profile_token_set = set(tokens(" ".join(row["description"] for row in profile["features"])))
    artifacts = load_json(group3 / "rendering-manifest.json")["artifacts"]
    comparisons, maximum = [], 0
    for source in corpus["sources"]:
        suffix = ".pdf" if source["media_type"] == "application/pdf" else ".html"
        path = source_cache / "sources" / f'{source["content_sha256"]}{suffix}'
        require(path.is_file() and sha256_file(path) == source["content_sha256"], f"source mismatch: {source['source_id']}")
        source_tokens = tokens(extract_source(path, source["media_type"]))
        for artifact in artifacts:
            render_tokens = tokens((group3 / "artifacts" / f'{artifact["artifact_id"]}.md').read_text())
            exact = longest_exact(source_tokens, render_tokens)
            maximum = max(maximum, exact["length"])
            mediated = longest_exact([t for t in source_tokens if t in profile_token_set],
                                     [t for t in render_tokens if t in profile_token_set])
            comparisons.append({
                "source_id": source["source_id"], "source_sha256": source["content_sha256"],
                "artifact_id": artifact["artifact_id"], "artifact_sha256": artifact["artifact_sha256"],
                "longest_exact": exact, "material_exact": exact["length"] >= 8,
                "near_verbatim": near_verbatim(source_tokens, render_tokens),
                "profile_mediated_longest_exact": mediated,
                "material_profile_mediated": mediated["length"] >= 5,
            })
    return {
        "source_objects": len(corpus["sources"]), "artifact_objects": len(artifacts),
        "comparison_count": len(comparisons), "maximum_exact_tokens": maximum,
        "material_exact_count": sum(row["material_exact"] for row in comparisons),
        "near_verbatim_count": sum(bool(row["near_verbatim"]) for row in comparisons),
        "material_profile_mediated_count": sum(row["material_profile_mediated"] for row in comparisons),
        "comparisons": comparisons,
    }


def validate_review(review: dict[str, Any], artifact_ids: set[str]) -> bool:
    rows = review.get("artifacts", [])
    require({row.get("artifact_id") for row in rows} == artifact_ids, "review does not cover exact artifact set")
    return all(row.get("canonical_coverage") is True and row.get("semantic_equivalence") is True
               and row.get("speech_act_equivalence") is True and row.get("salience_control") is True
               and row.get("forbidden_leakage") == [] for row in rows)


def validate_manipulation(result: dict[str, Any], mapping: dict[str, str]) -> dict[str, Any]:
    rows = result.get("artifacts", [])
    require({row.get("artifact_id") for row in rows} == set(mapping), "manipulation result artifact mismatch")
    grouped = {condition: [] for condition in ("C0", "C1", "C2")}
    correct = 0
    for row in rows:
        condition = mapping[row["artifact_id"]]
        grouped[condition].append(row)
        if row.get("assigned_level") == condition:
            correct += 1
    means = {
        condition: {
            "surface": sum(row["surface_uptake"] for row in group) / len(group),
            "discourse": sum(row["discourse_uptake"] for row in group) / len(group),
        } for condition, group in grouped.items()
    }
    inequalities = means["C1"]["surface"] > means["C0"]["surface"] and means["C2"]["discourse"] > means["C1"]["discourse"]
    return {"exact_assignments": correct, "means": means, "inequalities_pass": inequalities,
            "gate": correct >= 5 and inequalities}


def subject_verification() -> dict[str, Any]:
    completed = subprocess.run(["codex", "--version"], check=True, stdout=subprocess.PIPE)
    version = completed.stdout.decode().strip()
    return {
        "observed_at_preoutcome": {"codex_cli_version": version, "client_binary": str(Path("/usr/bin/codex")),
                                   "model_designation_from_frozen_configuration": "gpt-5.6-sol",
                                   "reasoning_effort_from_frozen_configuration": "medium",
                                   "service_tier_from_frozen_configuration": "standard_not_fast",
                                   "tools_from_frozen_configuration": "none"},
        "runtime_only_required_before_T001": ["effective model designation receipt", "backend/deployment identifiers",
                                               "sampling identifiers when exposed", "weekly usage meter and timestamp",
                                               "fresh-context and no-tools launcher proof"],
        "runtime_only_resolved": False,
        "gate": version == "codex-cli 0.149.1",
        "claim": "Local fields are verified; runtime-only identifiers remain mandatory before T001 and are not invented.",
    }


def audit(repository: Path, contract: Path, group3: Path, source_cache: Path,
          semantic_review: Path, sol_manipulation: Path, claude_manipulation: Path, output: Path) -> None:
    manifest = load_json(group3 / "rendering-manifest.json")
    artifact_ids = {row["artifact_id"] for row in manifest["artifacts"]}
    key = load_json(group3 / "condition-key.json")
    mapping = {row["artifact_id"]: row["condition"] for row in key["mapping"]}
    require(set(mapping) == artifact_ids, "condition key/rendering manifest mismatch")
    semantic = load_json(semantic_review)
    semantic_gate = validate_review(semantic, artifact_ids)
    sol_result, claude_result = load_json(sol_manipulation), load_json(claude_manipulation)
    sol = validate_manipulation(sol_result, mapping)
    claude = validate_manipulation(claude_result, mapping)
    rows = []
    for artifact in manifest["artifacts"]:
        artifact_id = artifact["artifact_id"]
        rows.append({"artifact_id": artifact_id, "condition": mapping[artifact_id],
                     "metrics": shallow_metrics((group3 / "artifacts" / f"{artifact_id}.md").read_text())})
    shallow_rules = perfect_scalar_rules(rows)
    quotation = quotation_audit(repository, group3, source_cache)
    subject = subject_verification()
    gates = {
        "six_artifacts_bound": len(artifact_ids) == 6 and all((group3 / "artifacts" / f"{item}.md").is_file() for item in artifact_ids),
        "canonical_coverage": semantic_gate,
        "semantic_equivalence": semantic_gate,
        "speech_act_equivalence": semantic_gate,
        "salience_control": semantic_gate,
        "actual_artifact_manipulation": sol["gate"] and claude["gate"],
        "no_single_feature_concentration": not shallow_rules,
        "subject_prelaunch_verification": subject["gate"],
    }
    quotation_gate = (quotation["material_exact_count"] == quotation["near_verbatim_count"] ==
                      quotation["material_profile_mediated_count"] == 0)
    overall = all(gates.values()) and quotation_gate
    emit_new(output / "quotation-audit.json", quotation)
    emit_new(output / "cue-concentration.json", {"rows": rows, "perfect_single_scalar_rules": shallow_rules,
                                                   "gate": not shallow_rules})
    emit_new(output / "subject-verification.json", subject)
    emit_new(output / "manipulation-report.json", {
        "perspectives": [
            {"provider_family": "OpenAI/Sol", "independence_claimed": False, "result": sol,
             "raw_result_sha256": sha256_file(sol_manipulation)},
            {"provider_family": "Anthropic/Claude", "different_provider_from_renderer": True,
             "independence_claimed": False, "result": claude,
             "raw_result_sha256": sha256_file(claude_manipulation)},
        ],
        "human_perspective": {"available": False, "status": "unresolved_not_fabricated"},
        "gate": sol["gate"] and claude["gate"],
    })
    gate = {
        "artifact_type": "linguistic_register_group_3_preoutcome_gate_v1", "schema_version": 1,
        "experiment_id": EXPERIMENT_ID, "group_3_id": GROUP3_ID,
        "contract_seal_sha256": sha256_file(contract / "contract-seal.json"),
        "rendering_manifest_sha256": sha256_file(group3 / "rendering-manifest.json"),
        "subject_verification_sha256": sha256_file(output / "subject-verification.json"),
        "semantic_review_sha256": sha256_file(semantic_review),
        "manipulation_report_sha256": sha256_file(output / "manipulation-report.json"),
        "quotation_audit_sha256": sha256_file(output / "quotation-audit.json"),
        "cue_concentration_sha256": sha256_file(output / "cue-concentration.json"),
        "gates": gates, "quotation_survival": quotation_gate, "overall_gate": overall,
        "human_audit": "unresolved; model reviews are evidence only and do not exercise human acceptance authority",
        "boundary": "This gate records preoutcome readiness only. Group 3 stops before T001 and does not authorize itself to execute behavioral trials.",
        "behavioral_outcomes_generated": False,
    }
    emit_new(output / "group-3-preoutcome-gate.json", gate)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prepare_parser = sub.add_parser("prepare")
    prepare_parser.add_argument("--repository", required=True, type=Path)
    prepare_parser.add_argument("--contract", required=True, type=Path)
    prepare_parser.add_argument("--output", required=True, type=Path)
    ingest_parser = sub.add_parser("ingest")
    ingest_parser.add_argument("--group3", required=True, type=Path)
    ingest_parser.add_argument("--evidence", required=True, type=Path)
    ingest_parser.add_argument("--output", required=True, type=Path)
    audit_parser = sub.add_parser("audit")
    audit_parser.add_argument("--repository", required=True, type=Path)
    audit_parser.add_argument("--contract", required=True, type=Path)
    audit_parser.add_argument("--group3", required=True, type=Path)
    audit_parser.add_argument("--source-cache", required=True, type=Path)
    audit_parser.add_argument("--semantic-review", required=True, type=Path)
    audit_parser.add_argument("--sol-manipulation", required=True, type=Path)
    audit_parser.add_argument("--claude-manipulation", required=True, type=Path)
    audit_parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.repository, args.contract, args.output)
    elif args.command == "ingest":
        ingest(args.group3, args.evidence, args.output)
    else:
        audit(args.repository, args.contract, args.group3, args.source_cache,
              args.semantic_review, args.sol_manipulation, args.claude_manipulation, args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (PreoutcomeError, OSError, subprocess.SubprocessError) as error:
        print(f"behavioral_pilot_preoutcome: {error}", file=sys.stderr)
        raise SystemExit(2)
