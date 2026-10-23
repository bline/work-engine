#!/usr/bin/env python3
"""Run plan -> battery -> evaluate for one or more readiness profile/subject pairs.

Usage:
  python3 -I integration_readiness.py \
      --binary BIN --binary-sha256 DIGEST --battery evidence_battery.py \
      --out-dir DIR --profile P --subject S [--profile P2 --subject S2 ...] \
      [--evaluated-at YYYY-MM-DDTHH:MM:SSZ]

The script is the procedural binding of the decision-readiness contract
(section 5.2, AC7). In one process it:

  1. verifies the binary's sha256 against --binary-sha256 and runs that binary
     by the explicit path given, never a build. The digest is verified again
     immediately before every execution (`plan` and each `evaluate`) and after
     `plan`, and each evaluation's own `tool.binary_sha256` (the evaluator's
     hash of the executable it ran as) must equal the pin. The pathname is
     still executed, so a replacement between a verification and the exec is
     not prevented; it is detected from the evaluation's record and fails the
     run (exit 5, with the summary still printed to show what ran);
  2. runs `plan` over all pairs into DIR/run-plan.json;
  3. runs the battery as `--plan RP --save DIR/battery-result.json --quiet`,
     where the result path must not exist beforehand;
  4. writes the observation record (decision-readiness-observation-v1) from
     what it ran: DIR/observation.json;
  5. runs `evaluate` once per pair with the same run plan, result and
     observation, into DIR/evaluation-<n>.json (n from 1, argument order).

The tool never reads the clock; this script supplies every time. A battery
exit of 0 or 1 with a saved result is a valid observation (errored checks are
data). The script reads the result file, not only the exit code.

Provenance boundary: `check --kind assessment` validates structure and digests, not
provenance. It does not show that an evaluation was derived from the plan, battery and
subject; an internally consistent forged evaluation passes it. Only `check --kind
evaluation --rederive` catches that, so a decision must cite a rederive result.

Exit codes:
  0  at least one evaluation per pair was written, whatever the statuses
  2  usage or input error, including a refused `plan` and an existing output
  3  `evaluate` refused the inputs (binding error) for at least one pair
  4  no observation: battery exit other than 0 or 1, no result file, or no
     parsable battery digest on its stdout
  5  the binary's sha256 differs from --binary-sha256, at the start or at a
     later re-verification (nothing further is run), or an evaluation records
     a `tool.binary_sha256` other than the pin

Standard library only; run with `python3 -I`.
"""
from __future__ import annotations

import argparse
import datetime
import errno
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

EXIT_OK = 0
EXIT_USAGE = 2
EXIT_BINDING = 3
EXIT_NO_OBSERVATION = 4
EXIT_DIGEST = 5

OBSERVATION_SCHEMA = "decision-readiness-observation-v1"


class Stop(Exception):
    """End the run with an exit code and a message on stderr."""

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def utc_now() -> str:
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_arguments(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="integration_readiness.py", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--binary", required=True, type=Path, help="work-engine-readiness binary, by explicit path")
    parser.add_argument("--binary-sha256", required=True, help="expected sha256 of the binary")
    parser.add_argument("--battery", required=True, type=Path, help="evidence_battery.py to run")
    parser.add_argument("--out-dir", required=True, type=Path, help="directory for the run's files")
    parser.add_argument("--profile", action="append", default=[], type=Path, help="profile file (repeat per pair)")
    parser.add_argument("--subject", action="append", default=[], type=Path, help="subject file (repeat per pair)")
    parser.add_argument("--evaluated-at", help="UTC time to record; default is the clock after the battery")
    return parser.parse_args(argv)


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, capture_output=True, text=True, check=False)


def verify_inputs(arguments: argparse.Namespace) -> None:
    if not arguments.profile or len(arguments.profile) != len(arguments.subject):
        raise Stop(EXIT_USAGE, "need one or more --profile P --subject S pairs, with equal counts")
    expected = arguments.binary_sha256.lower()
    if len(expected) != 64 or any(c not in "0123456789abcdef" for c in expected):
        raise Stop(EXIT_USAGE, "--binary-sha256 must be 64 hexadecimal characters")
    for label, path in (("--binary", arguments.binary), ("--battery", arguments.battery)):
        if not path.is_file():
            raise Stop(EXIT_USAGE, f"{label} {path} is not a file")
    for path in [*arguments.profile, *arguments.subject]:
        if not path.is_file():
            raise Stop(EXIT_USAGE, f"input {path} is not a file")
    actual = sha256_file(arguments.binary)
    if actual != expected:
        raise Stop(EXIT_DIGEST, f"binary sha256 mismatch: expected {expected}, found {actual}; nothing was run")


def verify_binary(binary: Path, expected: str, when: str) -> None:
    """Fail with EXIT_DIGEST unless the file at the pathname has the pinned digest now."""
    actual = sha256_file(binary)
    if actual != expected:
        raise Stop(EXIT_DIGEST, f"binary sha256 mismatch {when}: expected {expected}, found {actual}")


def evaluation_tool_digest(path: Path) -> str | None:
    """The `tool.binary_sha256` an evaluation records, or None when it is absent or unreadable."""
    try:
        digest = json.loads(path.read_text(encoding="utf-8"))["tool"]["binary_sha256"]
    except (OSError, ValueError, KeyError, TypeError):
        return None
    return digest if isinstance(digest, str) else None


def output_paths(out_dir: Path, pair_count: int) -> dict[str, Path]:
    paths = {
        "plan": out_dir / "run-plan.json",
        "result": out_dir / "battery-result.json",
        "observation": out_dir / "observation.json",
    }
    for index in range(1, pair_count + 1):
        paths[f"evaluation-{index}"] = out_dir / f"evaluation-{index}.json"
    return paths


def write_new(path: Path, text: str) -> None:
    with path.open("x", encoding="utf-8") as stream:
        stream.write(text)


def main(argv: list[str]) -> int:
    arguments = parse_arguments(argv)
    verify_inputs(arguments)
    pinned = arguments.binary_sha256.lower()
    binary = arguments.binary.resolve()
    battery = arguments.battery.resolve()
    out_dir = arguments.out_dir.resolve()
    paths = output_paths(out_dir, len(arguments.profile))
    out_dir.mkdir(parents=True, exist_ok=True)
    if not os.access(out_dir, os.W_OK | os.X_OK):
        raise PermissionError(errno.EACCES, "output directory is not writable", str(out_dir))
    existing = [str(path) for path in paths.values() if path.exists() or path.is_symlink()]
    if existing:
        raise Stop(EXIT_USAGE, "refusing to overwrite existing output: " + ", ".join(existing))

    plan_command = [str(binary), "plan", "--out", str(paths["plan"])]
    for profile, subject in zip(arguments.profile, arguments.subject):
        plan_command += ["--profile", str(profile.resolve()), "--subject", str(subject.resolve())]
    verify_binary(binary, pinned, "before plan; nothing was run")
    planned = run(plan_command)
    verify_binary(binary, pinned, "after plan; no battery was run")
    if planned.returncode != 0 or not paths["plan"].is_file():
        raise Stop(
            EXIT_USAGE,
            f"plan exited {planned.returncode}; no battery was run\n{planned.stdout}{planned.stderr}",
        )

    started_at = utc_now()
    battery_command = [
        sys.executable, "-I", str(battery), "--plan", str(paths["plan"]), "--save", str(paths["result"]), "--quiet",
    ]
    ran = run(battery_command)
    finished_at = utc_now()
    if ran.returncode not in (0, 1):
        raise Stop(EXIT_NO_OBSERVATION, f"battery exited {ran.returncode}: no observation\n{ran.stderr}")
    if not paths["result"].is_file():
        raise Stop(EXIT_NO_OBSERVATION, f"battery exited {ran.returncode} but wrote no result file {paths['result']}")
    try:
        printed = json.loads(ran.stdout)
        battery_digest = printed["battery_sha256"]
        if not isinstance(battery_digest, str):
            raise TypeError("battery_sha256 is not a string")
    except (ValueError, KeyError, TypeError) as error:
        raise Stop(EXIT_NO_OBSERVATION, f"battery stdout carries no battery_sha256 ({error}): no observation")

    observation = {
        "schema": OBSERVATION_SCHEMA,
        "plan": {"path": str(paths["plan"]), "sha256": sha256_file(paths["plan"])},
        "result": {"path": str(paths["result"]), "sha256": sha256_file(paths["result"])},
        "battery_script": {"path": str(battery), "sha256": sha256_file(battery)},
        "battery_exit": ran.returncode,
        "stdout_battery_sha256": battery_digest,
        "started_at": started_at,
        "finished_at": finished_at,
        "invoker": {"script": str(Path(__file__).resolve()), "sha256": sha256_file(Path(__file__).resolve())},
    }
    write_new(paths["observation"], json.dumps(observation, indent=2, sort_keys=True) + "\n")

    evaluated_at = arguments.evaluated_at or utc_now()
    codes: list[int] = []
    evaluations = []
    digest_failures: list[str] = []
    for index, (profile, subject) in enumerate(zip(arguments.profile, arguments.subject), start=1):
        target = paths[f"evaluation-{index}"]
        verify_binary(binary, pinned, f"before evaluate pair {index}; earlier outputs stay in {out_dir}")
        evaluated = run([
            str(binary), "evaluate",
            "--profile", str(profile.resolve()), "--subject", str(subject.resolve()),
            "--run-plan", str(paths["plan"]), "--battery", str(paths["result"]),
            "--observation", str(paths["observation"]),
            "--evaluated-at", evaluated_at, "--out", str(target),
        ])
        codes.append(evaluated.returncode)
        if evaluated.returncode != 0:
            sys.stderr.write(f"evaluate pair {index} exited {evaluated.returncode}\n{evaluated.stdout}{evaluated.stderr}")
        elif target.is_file():
            ran_as = evaluation_tool_digest(target)
            evaluations.append({
                "pair": index, "path": str(target), "sha256": sha256_file(target),
                "tool_binary_sha256": ran_as,
            })
            if ran_as != pinned:
                digest_failures.append(f"evaluate pair {index} recorded tool.binary_sha256 {ran_as}, pinned {pinned}")
        else:
            codes[-1] = EXIT_USAGE
            sys.stderr.write(f"evaluate pair {index} exited 0 but wrote no {target}\n")

    summary = {
        "binary": {
            "path": str(binary), "sha256": pinned,
            "evaluated_as_sha256": [entry["tool_binary_sha256"] for entry in evaluations],
        },
        "run_plan": observation["plan"],
        "battery_result": observation["result"],
        "observation": {"path": str(paths["observation"]), "sha256": sha256_file(paths["observation"])},
        "battery_exit": ran.returncode,
        "evaluated_at": evaluated_at,
        "evaluations": evaluations,
        "evaluate_exit_codes": codes,
    }
    sys.stdout.write(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    if digest_failures:
        sys.stderr.write("binary sha256 mismatch: " + "; ".join(digest_failures) + "\n")
        return EXIT_DIGEST
    if EXIT_BINDING in codes:
        return EXIT_BINDING
    if any(code != 0 for code in codes):
        return EXIT_USAGE
    return EXIT_OK


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except Stop as stop:
        sys.stderr.write(stop.message.rstrip("\n") + "\n")
        sys.exit(stop.code)
    except OSError as error:
        sys.stderr.write(json.dumps({
            "error": "os_error",
            "class": type(error).__name__,
            "errno": error.errno,
            "path": None if error.filename is None else str(error.filename),
            "message": error.strerror or str(error),
        }, sort_keys=True) + "\n")
        sys.exit(EXIT_USAGE)
