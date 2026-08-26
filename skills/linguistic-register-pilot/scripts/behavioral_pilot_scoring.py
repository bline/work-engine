#!/usr/bin/env python3
"""Deterministic primary scoring and agreement summaries for the pilot."""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Hashable


class ScoringError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ScoringError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected object: {path}")
    return value


def primary_score(trial: dict[str, Any], key: dict[str, Any]) -> int | str:
    require(trial["task_id"] == key["task_id"], "trial/key task mismatch")
    status = trial["execution_status"]
    require(status in {"completed", "invalid"}, "unknown execution status")
    if status == "invalid":
        require(isinstance(trial.get("invalid_reason"), str) and trial["invalid_reason"], "invalid trial requires reason")
        return "invalid"
    decisions = trial.get("decisions")
    require(isinstance(decisions, list) and len(decisions) == 2, "completed trial requires two decisions")
    normalized = [value.casefold() if isinstance(value, str) else value for value in decisions]
    expected = key["expected"]
    return int(normalized == [expected["turn_1_decision"], expected["turn_2_decision"]])


def cohens_kappa(left: list[Hashable], right: list[Hashable]) -> dict[str, Any]:
    require(len(left) == len(right) and left, "paired nonempty ratings required")
    observed = sum(a == b for a, b in zip(left, right)) / len(left)
    left_counts, right_counts = Counter(left), Counter(right)
    labels = set(left_counts) | set(right_counts)
    expected = sum((left_counts[label] / len(left)) * (right_counts[label] / len(right)) for label in labels)
    if math.isclose(expected, 1.0):
        return {"n": len(left), "raw_agreement": observed, "kappa": None, "undefined_reason": "no rating variance"}
    return {"n": len(left), "raw_agreement": observed, "kappa": (observed - expected) / (1 - expected), "undefined_reason": None}


def aggregate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[tuple[str, ...], list[int]] = defaultdict(list)
    invalids: Counter[str] = Counter()
    for row in rows:
        score = row["primary_score"]
        if score == "invalid":
            invalids[row["condition"]] += 1
            continue
        require(score in {0, 1}, "primary score must be 0, 1, or invalid")
        for key in ((row["condition"],), (row["condition"], row["task_family"]),
                    (row["condition"], row["task_id"]), (row["condition"], row["artifact_id"])):
            grouped[key].append(score)
    summaries = {"|".join(key): {"successes": sum(values), "n": len(values), "proportion": sum(values) / len(values)}
                 for key, values in sorted(grouped.items())}
    risk_differences = {}
    for condition in ("C1", "C2"):
        for suffix in ("A", "B", "A1", "A2", "B1", "B2"):
            treated, control = summaries.get(f"{condition}|{suffix}"), summaries.get(f"C0|{suffix}")
            if treated and control:
                risk_differences[f"{condition}-C0|{suffix}"] = treated["proportion"] - control["proportion"]
    return {"summaries": summaries, "absolute_risk_differences": risk_differences, "invalid_by_condition": dict(sorted(invalids.items()))}


def score_file(trials_path: Path, keys_dir: Path, output: Path) -> None:
    require(not output.exists(), f"refusing to overwrite: {output}")
    trials = json.loads(trials_path.read_text())
    require(isinstance(trials, list), "trial input must be a list")
    rows = []
    for trial in trials:
        key = load_json(keys_dir / f"{trial['task_id']}.json")
        rows.append({**trial, "primary_score": primary_score(trial, key)})
    output.write_bytes(canonical({"artifact_type": "linguistic_register_primary_scores_v1", "schema_version": 1,
                                  "rows": rows, "aggregate": aggregate(rows)}))


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    score = sub.add_parser("score-primary")
    score.add_argument("--trials", type=Path, required=True)
    score.add_argument("--keys", type=Path, required=True)
    score.add_argument("--output", type=Path, required=True)
    kappa = sub.add_parser("kappa")
    kappa.add_argument("--ratings", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "score-primary":
        score_file(args.trials, args.keys, args.output)
    else:
        ratings = json.loads(args.ratings.read_text())
        print(json.dumps(cohens_kappa(ratings["rater_1"], ratings["rater_2"]), sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ScoringError as error:
        raise SystemExit(str(error))
