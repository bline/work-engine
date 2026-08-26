#!/usr/bin/env python3
"""Generate and validate the frozen 48-trial behavioral-pilot schedule."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from itertools import product
from pathlib import Path
from typing import Any


EXPERIMENT_ID = "linguistic-register-behavioral-pilot-contract-v1-2026-08-26"
SEED = "sha256:linguistic-register-behavioral-pilot-contract-v1:balanced-order:v1"
CONDITIONS = ("C0", "C1", "C2")
TASKS = ("A1", "A2", "B1", "B2")
REPLICAS = (1, 2)
REPETITIONS = (1, 2)
CALIBRATION = (
    ("C0", 1, "A1", 1),
    ("C1", 2, "A2", 1),
    ("C2", 1, "B1", 1),
    ("C0", 2, "B2", 1),
    ("C1", 1, "B1", 2),
    ("C2", 2, "A1", 2),
    ("C0", 1, "A2", 2),
    ("C1", 2, "B2", 2),
)


class ScheduleError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ScheduleError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def descriptor_key(row: tuple[str, int, str, int]) -> str:
    condition, replica, task, repetition = row
    return f"{condition}-R{replica}|{task}|P{repetition}"


def hashed_order(row: tuple[str, int, str, int]) -> str:
    return hashlib.sha256(f"{SEED}|{descriptor_key(row)}".encode()).hexdigest()


def ordered_descriptors() -> list[tuple[str, int, str, int]]:
    universe = list(product(CONDITIONS, REPLICAS, TASKS, REPETITIONS))
    require(len(universe) == 48 and len(set(universe)) == 48, "invalid schedule universe")
    remainder = sorted((row for row in universe if row not in CALIBRATION), key=hashed_order)
    return list(CALIBRATION) + remainder


def build_schedule(task_manifest: Path) -> dict[str, Any]:
    manifest = json.loads(task_manifest.read_text())
    rows = {row["task_id"]: row for row in manifest["task_rows"]}
    require(set(rows) == set(TASKS), "task manifest must contain A1/A2/B1/B2 exactly")
    trials = []
    for index, (condition, replica, task_id, repetition) in enumerate(ordered_descriptors(), 1):
        row = rows[task_id]
        trials.append({
            "trial_id": f"T{index:03d}",
            "order": index,
            "phase": "budget_calibration" if index <= 8 else "main_batch_pending_budget_gate",
            "condition": condition,
            "artifact_id": f"{condition}-R{replica}",
            "artifact_replica": replica,
            "task_id": task_id,
            "task_family": task_id[0],
            "polarity": row["polarity"],
            "repetition": repetition,
            "task_sha256": row["task_sha256"],
            "evidence_sha256": row["evidence_sha256"],
            "fresh_context_required": True,
            "rerun_permitted": False,
        })
    return {
        "artifact_type": "linguistic_register_execution_schedule_v1",
        "schema_version": 1,
        "experiment_id": EXPERIMENT_ID,
        "generation": {
            "algorithm": "explicit frozen eight-trial calibration prefix; remaining unique descriptors sorted by SHA-256 of seed and descriptor",
            "seed": SEED,
            "task_manifest_sha256": sha256_file(task_manifest),
        },
        "trials": trials,
    }


def validate_schedule(schedule: dict[str, Any], task_manifest: Path) -> dict[str, Any]:
    expected = build_schedule(task_manifest)
    require(schedule == expected, "schedule is not the exact deterministic schedule")
    trials = schedule["trials"]
    require(len(trials) == 48, "expected 48 trials")
    require(len({row["trial_id"] for row in trials}) == 48, "trial IDs must be unique")
    require(len({(row["condition"], row["artifact_replica"], row["task_id"], row["repetition"]) for row in trials}) == 48,
            "trial descriptors must be unique")
    counts = {
        "condition": Counter(row["condition"] for row in trials),
        "artifact": Counter(row["artifact_id"] for row in trials),
        "task": Counter(row["task_id"] for row in trials),
        "family": Counter(row["task_family"] for row in trials),
        "repetition": Counter(row["repetition"] for row in trials),
    }
    require(set(counts["condition"].values()) == {16}, "conditions must have 16 trials each")
    require(set(counts["artifact"].values()) == {8}, "artifacts must have 8 trials each")
    require(set(counts["task"].values()) == {12}, "tasks must have 12 trials each")
    require(set(counts["family"].values()) == {24}, "families must have 24 trials each")
    require(set(counts["repetition"].values()) == {24}, "repetitions must have 24 trials each")
    calibration = trials[:8]
    require({row["condition"] for row in calibration} == set(CONDITIONS), "calibration must cover all conditions")
    require({row["task_id"] for row in calibration} == set(TASKS), "calibration must cover all tasks")
    require({row["artifact_replica"] for row in calibration} == set(REPLICAS), "calibration must cover both replicas")
    return {name: dict(sorted(counter.items())) for name, counter in counts.items()}


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    generate = sub.add_parser("generate")
    generate.add_argument("--task-manifest", type=Path, required=True)
    generate.add_argument("--output", type=Path, required=True)
    validate = sub.add_parser("validate")
    validate.add_argument("--task-manifest", type=Path, required=True)
    validate.add_argument("schedule", type=Path)
    args = parser.parse_args()
    if args.command == "generate":
        require(not args.output.exists(), f"refusing to overwrite: {args.output}")
        value = build_schedule(args.task_manifest)
        validate_schedule(value, args.task_manifest)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(canonical(value))
        print(sha256_file(args.output))
    else:
        value = json.loads(args.schedule.read_text())
        print(json.dumps(validate_schedule(value, args.task_manifest), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
