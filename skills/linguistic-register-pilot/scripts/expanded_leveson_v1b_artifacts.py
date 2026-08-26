#!/usr/bin/env python3
"""Build the fresh v1b expanded-Leveson treatment-validation artifacts."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import Any

import yaml


ROOT = Path("skills/linguistic-register-pilot")
RUN_V1 = ROOT / "pilot/behavioral-pilot-construction/leveson-expanded-v1"
RUN_V1B = ROOT / "pilot/behavioral-pilot-construction/leveson-expanded-v1b"
TARGETS = {
    "lexicon-abstract-relational",
    "lexicon-diagnostic-deficit",
    "syntax-stacked-subordination",
    "rhythm-telegraphic-prompt-response",
}
TARGET = "leveson-system-safety-expanded-v1"
PRACTICE_TO_CANDIDATE = {
    "Bayesian model criticism": "gelman-model-criticism",
    "system-safety review and causal-boundary analysis": TARGET,
    "software-engineering judgment": "shaw-engineering-judgment",
    "neutral professional editing": "neutral-editorial-defaults",
}
WORD_RE = re.compile(r"[\w]+(?:['’][\w]+)?", re.UNICODE)


class V1BError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise V1BError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tokens(value: str) -> list[str]:
    return [word.casefold().replace("’", "'") for word in WORD_RE.findall(value)]


def load_yaml(path: Path) -> dict[str, Any]:
    value = yaml.safe_load(path.read_text())
    require(isinstance(value, dict), f"expected object: {path}")
    return value


def prereg(subject: Path) -> dict[str, Any]:
    return load_yaml(subject / RUN_V1B / "preregistration.yaml")


def write_new(path: Path, value: Any) -> None:
    require(not path.exists(), f"refusing to overwrite: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value))


def prepare_evidence(subject: Path, prior_packet: Path, output: Path) -> None:
    registration = prereg(subject)
    manifest = json.loads((subject / RUN_V1 / "corpus-manifest.json").read_text())
    prior_profile = json.loads((subject / RUN_V1 / "profile.json").read_text())
    require(sha256_file(prior_packet) == manifest["extraction_packet_sha256"], "prior extraction packet digest mismatch")
    frozen = {row["id"]: row for row in prior_profile["features"] if row["id"] in TARGETS}
    require(set(frozen) == TARGETS, "target feature set mismatch")
    packet = json.loads(prior_packet.read_text())
    require(len(packet["sources"]) == 8, "expected bound eight-source packet")
    write_new(output, {
        "artifact_type": "linguistic_register_expanded_feature_evidence_packet_v1b",
        "schema_version": 1,
        "experiment_id": registration["experiment_id"],
        "preregistration_sha256": sha256_file(subject / RUN_V1B / "preregistration.yaml"),
        "features": [{key: row[key] for key in ("id", "layer", "category", "description", "distinctiveness_weight", "evidence")}
                     for row in sorted(frozen.values(), key=lambda item: item["id"])],
        "sources": packet["sources"],
        "decision_contract": "Add only concrete third-document evidence found in these excerpts; otherwise return drop. Do not infer support from topic alone.",
        "task_exposure": {"behavioral_tasks": False, "behavioral_keys": False, "behavioral_outcomes": False},
    })


def recover_profile(subject: Path, packet: Path, result: Path, output: Path, ledger_output: Path) -> None:
    registration = prereg(subject)
    prior = json.loads((subject / RUN_V1 / "profile.json").read_text())
    packet_value = json.loads(packet.read_text())
    result_value = json.loads(result.read_text())
    require(result_value["packet_sha256"] == sha256_file(packet), "evidence result packet binding mismatch")
    decisions = {row["feature_id"]: row for row in result_value["features"]}
    require(set(decisions) == TARGETS and len(result_value["features"]) == 4, "evidence decision feature set mismatch")
    source_meta = {row["source_id"]: row for row in packet_value["sources"]}
    features = []
    ledger = []
    for original in prior["features"]:
        row = dict(original)
        row["treatment_disposition"] = "retained" if row["disposition"] == "realization_only" else "excluded_semantically"
        if row["id"] in TARGETS:
            decision = decisions[row["id"]]
            existing_ids = {evidence["source_id"] for evidence in row["evidence"]}
            additions = []
            for evidence in decision["evidence"]:
                require(evidence["source_id"] in source_meta, f"unknown source in evidence: {evidence['source_id']}")
                require(evidence["source_id"] not in existing_ids, f"evidence does not add a distinct document: {row['id']}")
                require(evidence["excerpt_segment"] in {entry["segment"] for entry in source_meta[evidence["source_id"]]["excerpts"]}, "unknown excerpt segment")
                additions.append(evidence)
            additions_by_source = {entry["source_id"]: entry for entry in additions}
            require(len(additions_by_source) == len(additions), f"duplicate added source evidence: {row['id']}")
            if decision["decision"] == "retain":
                row["evidence"] = row["evidence"] + additions
                require(len({entry["source_id"] for entry in row["evidence"]}) >= 3, f"retained feature lacks three documents: {row['id']}")
                row["cross_family"] = len({source_meta[entry["source_id"]]["genre_family"] for entry in row["evidence"]}) >= 2
                row["treatment_disposition"] = "retained"
            else:
                require(not additions, f"dropped feature supplied evidence: {row['id']}")
                row["treatment_disposition"] = "dropped_insufficient_recurrence"
            ledger.append({
                "feature_id": row["id"], "decision": decision["decision"],
                "prior_document_count": len(existing_ids),
                "added_source_ids": sorted(additions_by_source),
                "final_document_count": len({entry["source_id"] for entry in row["evidence"]}),
                "treatment_disposition": row["treatment_disposition"],
                "rationale": decision["rationale"],
            })
        features.append(row)
    profile = dict(prior)
    profile.update({
        "artifact_type": "linguistic_register_expanded_profile_v1b",
        "schema_version": 1,
        "experiment_id": registration["experiment_id"],
        "bindings": dict(prior["bindings"], preregistration_sha256=sha256_file(subject / RUN_V1B / "preregistration.yaml"),
                         evidence_packet_sha256=sha256_file(packet), evidence_result_sha256=sha256_file(result),
                         prior_profile_sha256=sha256_file(subject / RUN_V1 / "profile.json")),
        "features": features,
        "limitations": prior.get("limitations", []) + result_value.get("limitations", []),
    })
    write_new(output, profile)
    write_new(ledger_output, {
        "artifact_type": "linguistic_register_expanded_feature_disposition_ledger_v1b",
        "schema_version": 1, "experiment_id": registration["experiment_id"],
        "bindings": {"preregistration_sha256": sha256_file(subject / RUN_V1B / "preregistration.yaml"),
                     "evidence_packet_sha256": sha256_file(packet), "evidence_result_sha256": sha256_file(result),
                     "profile_sha256": sha256_file(output)},
        "features": ledger,
    })


CARD_ROWS = {
    "gelman-model-criticism": [
        ("symmetric-opposition", "surface", "contrast-patterns", "Frame disagreement as two opposing concerns before addressing either position in detail."),
        ("first-person-epistemic-positioning", "surface", "claim-posture", "State the speaker's practice, uncertainty, or limitation directly instead of making every conclusion impersonal."),
        ("numbered-response-structure", "surface", "paragraph-rhythm", "Organize a disagreement into numbered sections and compact pointwise replies that preserve its distinctions."),
        ("informal-analogy-and-aside", "surface", "rhythm", "Break formal analysis with a brief analogy, conversational aside, or lightly humorous clarification without changing the claim."),
        ("partial-agreement-before-boundary", "discourse", "qualification-patterns", "Acknowledge the valid part of an objection before identifying the narrower point where disagreement remains."),
    ],
    "shaw-engineering-judgment": [
        ("parallel-guiding-questions", "surface", "paragraph-rhythm", "Introduce evaluation through a parallel series of guiding questions that exposes the dimensions under inspection."),
        ("neutral-progression-connectives", "surface", "connective-patterns", "Move from a general observation through examples and bounded contrast to an implication using restrained connectives."),
        ("compact-parallel-categories", "surface", "syntax", "Present supplied categories in compact parallel grammatical forms so their differences remain comparable."),
        ("consequence-first-conditionals", "discourse", "conclusion-patterns", "State a condition's practical consequence directly, then explain the supplied evidence or mechanism behind it."),
        ("restrained-evaluative-calibration", "discourse", "qualification-patterns", "Mark evidential maturity, distinguishing observations, tendencies, and established results without changing the assessment."),
    ],
    "neutral-editorial-defaults": [
        ("focused-decision", "surface", "paragraph-rhythm", "Keep the response centered on the supplied review decision without adding secondary framing."),
        ("ordinary-transitions", "surface", "connective-patterns", "Use ordinary transitions between supplied propositions so the explanation reads continuously without special emphasis."),
        ("direct-syntax", "surface", "syntax", "Prefer direct sentences with explicit subjects and actions, preserving the supplied claims and stated relationships."),
        ("concise-conclusion", "surface", "conclusion-patterns", "End with a concise restatement of the decision without adding an implication, recommendation, or claim."),
        ("single-limitation", "discourse", "qualification-patterns", "State one supplied evidence limitation once, proportionally to its scope, without additional evaluation or inference."),
    ],
    TARGET: [
        ("syntax-enumerative-parallelism", "surface", "syntax", "Present distinctions through numbered taxonomies, paired alternatives, and repeated frames that make categories ordered sets."),
        ("rhythm-exposition-list-bursts", "surface", "rhythm", "Alternate explanation with labels, lists, and parallel fragments, creating bursts of compression and emphasis."),
        ("syntax-stacked-subordination", "surface", "syntax", "Build sentences with conditional and causal clauses so claim, rationale, exception, and consequence share one unit."),
        ("connective-causal-chain", "discourse", "connective", "Use causal connectives to link conditions to mechanism, consequence, and response in a chain."),
        ("connective-additive-exemplification", "discourse", "connective", "Develop the claim cumulatively through examples, restatements, and considerations instead of topic changes."),
    ],
}


def build_cards(subject: Path, profile_path: Path, output: Path, key_output: Path) -> None:
    registration = prereg(subject)
    profile = json.loads(profile_path.read_text())
    available = {row["id"] for row in profile["features"] if row["treatment_disposition"] == "retained"}
    require({row[0] for row in CARD_ROWS[TARGET]} <= available, "Leveson card references unavailable retained feature")
    mapping = {"B01": "neutral-editorial-defaults", "B02": TARGET, "B03": "gelman-model-criticism", "B04": "shaw-engineering-judgment"}
    normalized_weights = [4, 4, 3, 3, 3]
    cards = []
    for card_id, candidate in mapping.items():
        rows = CARD_ROWS[candidate]
        features = [{"source_feature_id": row[0], "layer": row[1], "category": row[2], "description": row[3], "salience_weight": weight}
                    for row, weight in zip(rows, normalized_weights)]
        word_count = len(tokens(" ".join(row["description"] for row in features)))
        require(68 <= word_count <= 72, f"card word count outside frozen band: {candidate}={word_count}")
        cards.append({"anonymous_card_id": card_id, "features": features})
    packet = {
        "artifact_type": "linguistic_register_expanded_recognizability_packet_v1b", "schema_version": 1,
        "experiment_id": registration["experiment_id"], "preregistration_sha256": sha256_file(subject / RUN_V1B / "preregistration.yaml"),
        "profile_sha256": sha256_file(profile_path),
        "candidate_practices": list(PRACTICE_TO_CANDIDATE), "cards": cards,
        "task": "Assign each anonymous card to exactly one candidate practice and explain the profile cues used.",
    }
    write_new(output, packet)
    write_new(key_output, {"artifact_type": "linguistic_register_expanded_recognizability_key_v1b", "schema_version": 1,
                           "packet_sha256": sha256_file(output), "mapping": mapping})


def shallow_metrics(features: list[dict[str, Any]]) -> dict[str, int]:
    text = " ".join(row["description"] for row in features)
    weights = [row["salience_weight"] for row in features]
    return {
        "normalized_word_count": len(tokens(text)), "sentence_count": len(re.findall(r"[.!?]+", text)),
        "question_mark_count": text.count("?"), "colon_count": text.count(":"), "semicolon_count": text.count(";"),
        "dash_count": len(re.findall(r"[-–—]", text)), "parenthesis_count": text.count("(") + text.count(")"),
        "feature_count": len(features), "surface_layer_count": sum(row["layer"] == "surface" for row in features),
        "discourse_layer_count": sum(row["layer"] == "discourse" for row in features),
        "salience_weight_sum": sum(weights), "salience_weight_minimum": min(weights), "salience_weight_maximum": max(weights),
    }


def cue_baseline(packet_path: Path, key_path: Path, output: Path) -> None:
    packet = json.loads(packet_path.read_text())
    key = json.loads(key_path.read_text())
    rows = []
    for card in packet["cards"]:
        for replica in range(3):
            rotated = card["features"][replica:] + card["features"][:replica]
            rows.append({"candidate_id": key["mapping"][card["anonymous_card_id"]], "replica": replica + 1,
                         "metrics": shallow_metrics(rotated)})
    perfect = []
    for name in sorted(rows[0]["metrics"]):
        values = sorted({row["metrics"][name] for row in rows})
        thresholds = values + [(left + right) / 2 for left, right in zip(values, values[1:])]
        for operator in ("equal", "greater_than", "less_than_or_equal"):
            for threshold in thresholds:
                correct = []
                for row in rows:
                    value = row["metrics"][name]
                    predicted = value == threshold if operator == "equal" else value > threshold if operator == "greater_than" else value <= threshold
                    correct.append(predicted == (row["candidate_id"] == TARGET))
                if all(correct):
                    perfect.append({"metric": name, "operator": operator, "threshold": threshold})
    write_new(output, {
        "artifact_type": "linguistic_register_expanded_cue_baseline_v1b", "schema_version": 1,
        "packet_sha256": sha256_file(packet_path), "replicas": rows,
        "cross_replica_perfect_leveson_rules": perfect, "gate": not perfect,
        "limitation": "Feature-order replicas test preregistered shallow card statistics, not semantic cue sufficiency or rendered prose.",
    })


def score(packet_path: Path, key_path: Path, result_path: Path, cue_path: Path, output: Path) -> None:
    packet, key, result, cue = (json.loads(path.read_text()) for path in (packet_path, key_path, result_path, cue_path))
    require(result["packet_sha256"] == key["packet_sha256"] == sha256_file(packet_path), "recognition binding mismatch")
    assignments = result["assignments"]
    require({row["anonymous_card_id"] for row in assignments} == set(key["mapping"]), "recognition card set mismatch")
    require(len({row["candidate_practice"] for row in assignments}) == 4, "recognition assignments are not bijective")
    rows = []
    for row in assignments:
        expected = key["mapping"][row["anonymous_card_id"]]
        predicted = PRACTICE_TO_CANDIDATE[row["candidate_practice"]]
        rows.append(dict(row, expected_candidate_id=expected, predicted_candidate_id=predicted, correct=expected == predicted))
    target_row = next(row for row in rows if row["expected_candidate_id"] == TARGET)
    write_new(output, {
        "artifact_type": "linguistic_register_expanded_recognizability_report_v1b", "schema_version": 1,
        "bindings": {"packet_sha256": sha256_file(packet_path), "key_sha256": sha256_file(key_path),
                     "result_sha256": sha256_file(result_path), "cue_baseline_sha256": sha256_file(cue_path)},
        "assignments": rows, "total_correct": sum(row["correct"] for row in rows),
        "expanded_leveson_assignment_correct": target_row["correct"],
        "recognizability_gate": target_row["correct"], "cue_concentration_gate": cue["gate"],
        "gate": target_row["correct"] and cue["gate"],
        "interpretation": "same-family length-matched, count-matched, salience-normalized profile-card traceability only",
        "limitations": result.get("limitations", []),
    })


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prepare = sub.add_parser("prepare-evidence")
    prepare.add_argument("--subject", type=Path, default=Path.cwd()); prepare.add_argument("--prior-packet", type=Path, required=True); prepare.add_argument("--output", type=Path, required=True)
    recover = sub.add_parser("recover-profile")
    recover.add_argument("--subject", type=Path, default=Path.cwd()); recover.add_argument("--packet", type=Path, required=True); recover.add_argument("--result", type=Path, required=True); recover.add_argument("--output", type=Path, required=True); recover.add_argument("--ledger-output", type=Path, required=True)
    cards = sub.add_parser("build-cards")
    cards.add_argument("--subject", type=Path, default=Path.cwd()); cards.add_argument("--profile", type=Path, required=True); cards.add_argument("--output", type=Path, required=True); cards.add_argument("--key-output", type=Path, required=True)
    cue = sub.add_parser("cue-baseline")
    cue.add_argument("--packet", type=Path, required=True); cue.add_argument("--key", type=Path, required=True); cue.add_argument("--output", type=Path, required=True)
    scoring = sub.add_parser("score")
    scoring.add_argument("--packet", type=Path, required=True); scoring.add_argument("--key", type=Path, required=True); scoring.add_argument("--result", type=Path, required=True); scoring.add_argument("--cue", type=Path, required=True); scoring.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "prepare-evidence": prepare_evidence(args.subject.resolve(), args.prior_packet.resolve(), args.output.resolve())
    elif args.command == "recover-profile": recover_profile(args.subject.resolve(), args.packet.resolve(), args.result.resolve(), args.output.resolve(), args.ledger_output.resolve())
    elif args.command == "build-cards": build_cards(args.subject.resolve(), args.profile.resolve(), args.output.resolve(), args.key_output.resolve())
    elif args.command == "cue-baseline": cue_baseline(args.packet.resolve(), args.key.resolve(), args.output.resolve())
    else: score(args.packet.resolve(), args.key.resolve(), args.result.resolve(), args.cue.resolve(), args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (V1BError, OSError, json.JSONDecodeError) as error:
        print(f"expanded_leveson_v1b_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
