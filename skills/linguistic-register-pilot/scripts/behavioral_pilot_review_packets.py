#!/usr/bin/env python3
"""Create blinded group-3 semantic and manipulation review launch artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any


GROUP3_ID = "linguistic-register-behavioral-pilot-group-3-v1-2026-08-26"


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def emit(path: Path, value: Any) -> None:
    if path.exists():
        raise ValueError(f"refusing to overwrite: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value))


def request(job_id: str, stage: str, packet: bytes, schema: bytes, prompt: str,
            provider: str, model: str, effort: str, tier: str) -> dict[str, Any]:
    record = {
        "experiment_id": GROUP3_ID, "job_id": job_id, "stage": stage,
        "packet_sha256": digest(packet), "schema_sha256": digest(schema),
        "prompt_sha256": digest(prompt.encode()), "provider": provider,
        "model": model, "reasoning_effort": effort, "service_tier": tier,
    }
    return {**record, "request_sha256": digest(canonical(record))}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, type=Path)
    parser.add_argument("--group3", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    rendering_manifest_path = args.group3 / "rendering-manifest-v2.json"
    manifest = json.loads(rendering_manifest_path.read_text())
    artifacts = [{"artifact_id": row["artifact_id"],
                  "text": (args.group3 / "artifacts" / f'{row["artifact_id"]}.md').read_text()}
                 for row in manifest["artifacts"]]
    commit = "db3371916ad64404cf2e29894a75cae84c90b8f0"
    canonical_role = {}
    for name, path in {
        "entrypoint": "skills/agent-instruction-review/SKILL.md",
        "finding_contract": "skills/agent-instruction-review/references/finding-contract.md",
    }.items():
        canonical_role[name] = subprocess.run(
            ["git", "show", f"{commit}:{path}"], cwd=args.repository,
            check=True, stdout=subprocess.PIPE).stdout.decode()
    semantic_packet = {
        "artifact_type": "linguistic_register_group_3_semantic_review_packet_v1", "schema_version": 1,
        "group_3_id": GROUP3_ID, "blinding": "No condition mapping, profile, author identity, or source text is present.",
        "canonical_role": canonical_role,
        "semantic_units": [
            "advisory read-only revision-bound review",
            "materially normative agent-facing applicability boundary",
            "structural-necessity and exact-route-causality diagnosis",
            "semantic owner, consumer, authority, scope, precedence, audience, and loading placement",
            "DESIGN binding versus PHILOSOPHY non-normative distinction",
            "bounded neighboring-context investigation and unresolved-owner escalation",
            "compact evidence-bearing finding contract",
            "observation/inference separation and immutable subject binding",
            "retained human or builder decision authority",
            "continuation without false freshness or independence claim",
        ],
        "artifacts": artifacts,
        "rubric": {
            "canonical_coverage": "Every canonical semantic unit and finding-contract obligation remains present.",
            "semantic_equivalence": "No claim, requirement, permission, evidence operation, decision policy, or authority boundary is added, removed, strengthened, or weakened.",
            "speech_act_equivalence": "Advisory, read-only, bounded-review force and human decision authority are unchanged.",
            "salience_control": "No one canonical instruction is made materially more behaviorally salient than in the source role.",
            "forbidden_leakage": "Record names, source/domain clues, profile labels, task content, examples, or behavioral heuristics.",
        },
    }
    semantic_schema = {
        "$schema": "http://json-schema.org/draft-07/schema#", "type": "object",
        "required": ["artifacts", "limitations"], "properties": {
            "artifacts": {"type": "array", "minItems": 6, "maxItems": 6, "items": {"type": "object",
                "required": ["artifact_id", "canonical_coverage", "semantic_equivalence", "speech_act_equivalence", "salience_control", "forbidden_leakage", "evidence"],
                "properties": {
                    "artifact_id": {"type": "string"}, "canonical_coverage": {"type": "boolean"},
                    "semantic_equivalence": {"type": "boolean"}, "speech_act_equivalence": {"type": "boolean"},
                    "salience_control": {"type": "boolean"}, "forbidden_leakage": {"type": "array", "items": {"type": "string"}},
                    "evidence": {"type": "array", "items": {"type": "string"}},
                }, "additionalProperties": False}},
            "limitations": {"type": "string"},
        }, "additionalProperties": False,
    }
    manipulation_packet = {
        "artifact_type": "linguistic_register_group_3_manipulation_packet_v1", "schema_version": 1,
        "group_3_id": GROUP3_ID,
        "blinding": "Exactly two artifacts come from each level; mapping and source identity are withheld.",
        "levels": {
            "C0": "Neutral technical realization without intentional treatment uptake.",
            "C1": "Surface treatment uptake: abstract relational vocabulary, precise diagnostic deficit vocabulary, stacked subordination, enumerative parallelism, exposition/list rhythm, and telegraphic compression; no heightened discourse operation.",
            "C2": "The same surface uptake plus realization-only causal chaining and additive elaboration of relationships already present; no new inference, example, evidence operation, or conclusion.",
        },
        "artifacts": artifacts,
        "instructions": "Assign exactly two artifacts to each level. Score surface and discourse uptake from 0 to 5. Cite intended prose evidence separately from shallow, forbidden, or apparatus cues.",
    }
    manipulation_schema = {
        "$schema": "http://json-schema.org/draft-07/schema#", "type": "object",
        "required": ["artifacts", "limitations"], "properties": {
            "artifacts": {"type": "array", "minItems": 6, "maxItems": 6, "items": {"type": "object",
                "required": ["artifact_id", "assigned_level", "surface_uptake", "discourse_uptake", "intended_features", "shallow_or_forbidden_cues", "rationale"],
                "properties": {
                    "artifact_id": {"type": "string"}, "assigned_level": {"enum": ["C0", "C1", "C2"]},
                    "surface_uptake": {"type": "integer", "minimum": 0, "maximum": 5},
                    "discourse_uptake": {"type": "integer", "minimum": 0, "maximum": 5},
                    "intended_features": {"type": "array", "items": {"type": "string"}},
                    "shallow_or_forbidden_cues": {"type": "array", "items": {"type": "string"}},
                    "rationale": {"type": "string"},
                }, "additionalProperties": False}},
            "limitations": {"type": "string"},
        }, "additionalProperties": False,
    }
    emit(args.output / "packets/semantic-review.json", semantic_packet)
    emit(args.output / "packets/manipulation.json", manipulation_packet)
    emit(args.output / "schemas/semantic-review.schema.json", semantic_schema)
    emit(args.output / "schemas/manipulation.schema.json", manipulation_schema)
    semantic_prompt = "Apply the supplied rubric independently to all six opaque role artifacts. Return only schema-valid JSON. This is advisory evidence, not human acceptance."
    manipulation_prompt = "Blindly classify all six artifacts under the supplied three-level manipulation contract. Use intended prose features, report incidental cues separately, and return only schema-valid JSON."
    semantic_bytes = canonical(semantic_packet)
    manipulation_bytes = canonical(manipulation_packet)
    semantic_schema_bytes = canonical(semantic_schema)
    manipulation_schema_bytes = canonical(manipulation_schema)
    jobs = [
        request("semantic-claude-v4", "semantic-review", semantic_bytes, semantic_schema_bytes,
                semantic_prompt, "Anthropic", "sonnet", "high", "provider_default"),
        request("manipulation-sol-v4", "manipulation", manipulation_bytes, manipulation_schema_bytes,
                manipulation_prompt, "OpenAI", "gpt-5.6-sol", "medium", "standard_not_fast"),
        request("manipulation-claude-v4", "manipulation", manipulation_bytes, manipulation_schema_bytes,
                manipulation_prompt, "Anthropic", "sonnet", "high", "provider_default"),
    ]
    launch = {
        "artifact_type": "linguistic_register_group_3_review_launch_manifest_v1", "schema_version": 1,
        "group_3_id": GROUP3_ID, "rendering_manifest_sha256": digest(rendering_manifest_path.read_bytes()),
        "semantic_prompt": semantic_prompt, "manipulation_prompt": manipulation_prompt, "jobs": jobs,
    }
    launch["launch_set_sha256"] = digest(canonical(launch))
    emit(args.output / "review-launch-manifest.json", launch)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
