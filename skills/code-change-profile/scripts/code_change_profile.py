#!/usr/bin/env python3
from __future__ import annotations

import argparse
import ast
import hashlib
import importlib.util
import json
import platform
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

ANALYZER_NAME = "work-engine-deterministic-baseline"
ANALYZER_VERSION = "1"
CHECKPOINT_VALIDATOR_SHA256 = "1b814da3ef9e1f20804b934b91f6621ed65159dd9dfab5bfbfdd2df95f82d20b"
STATES = {"observed", "unknown", "unsupported", "failed", "not_applicable"}


class ProfileError(RuntimeError):
    pass


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def digest(value: Any) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def resolved_executable(command: str) -> Path:
    executable = shutil.which(command)
    if executable is None:
        raise ProfileError(f"required executable is unavailable: {command}")
    path = Path(executable).resolve()
    if not path.is_file():
        raise ProfileError(f"resolved executable is not a file: {command}")
    return path


def git_runtime_identity() -> dict[str, str]:
    executable = resolved_executable("git")
    result = subprocess.run(
        [str(executable), "--version"], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, check=False,
    )
    if result.returncode or not result.stdout.strip():
        raise ProfileError("Git runtime identity is unavailable")
    return {
        "version": result.stdout.strip(),
        "executable_sha256": sha256_file(executable),
    }


def python_runtime_identity() -> dict[str, str]:
    executable = Path(sys.executable).resolve()
    if not executable.is_file():
        raise ProfileError("Python runtime executable is unavailable")
    return {
        "implementation": platform.python_implementation(),
        "version": platform.python_version(),
        "executable_sha256": sha256_file(executable),
    }


def git(repository: Path, arguments: list[str], *, binary: bool = False) -> bytes | str:
    executable = resolved_executable("git")
    result = subprocess.run(
        [str(executable), "-C", str(repository), *arguments],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode:
        raise ProfileError(result.stderr.decode(errors="replace").strip() or "git command failed")
    return result.stdout if binary else result.stdout.decode()


def measurement(state: str, *, value: Any = None, reason: str | None = None) -> dict[str, Any]:
    if state not in STATES:
        raise ProfileError(f"invalid measurement state: {state}")
    if state == "observed":
        if reason is not None:
            raise ProfileError("observed measurement cannot carry a reason")
        return {"state": state, "value": value}
    if value is not None or not isinstance(reason, str) or not reason:
        raise ProfileError(f"{state} measurement requires a reason and no value")
    return {"state": state, "reason": reason}


def checkpoint_module(repository: Path) -> Any:
    path = repository / "skills/slice-checkpoint/scripts/checkpoint.py"
    try:
        actual_digest = sha256_file(path)
    except OSError as error:
        raise ProfileError("slice-checkpoint validator is unavailable") from error
    if actual_digest != CHECKPOINT_VALIDATOR_SHA256:
        raise ProfileError(
            "slice-checkpoint validator does not match analyzer-version binding"
        )
    spec = importlib.util.spec_from_file_location("work_engine_slice_checkpoint", path)
    if spec is None or spec.loader is None:
        raise ProfileError("slice-checkpoint validator is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def materialize_receipt(raw: dict[str, Any], repository: Path) -> dict[str, Any]:
    receipt = json.loads(json.dumps(raw))
    if receipt.get("repository") == "${REPOSITORY}":
        receipt["repository"] = str(repository)
    return receipt


def validate_subject(raw: Any, repository_override: Path | None = None) -> dict[str, Any]:
    if not isinstance(raw, dict) or set(raw) != {
        "schema_version", "construction_method", "evidence_cutoff", "checkpoint"
    }:
        raise ProfileError("subject fields must exactly match schema version 1")
    if raw["schema_version"] != 1 or raw["construction_method"] != "full_slice_checkpoint_lifecycle_receipt":
        raise ProfileError("unsupported subject schema or construction method")
    if not isinstance(raw["evidence_cutoff"], str) or not raw["evidence_cutoff"]:
        raise ProfileError("evidence_cutoff must be nonempty")
    checkpoint = raw["checkpoint"]
    if not isinstance(checkpoint, dict):
        raise ProfileError("checkpoint must be an object")
    repository = (repository_override or Path(checkpoint.get("repository", ""))).resolve()
    checkpoint = materialize_receipt(checkpoint, repository)
    kind = checkpoint.get("checkpoint_kind")
    if kind not in {"accepted", "stopped"}:
        raise ProfileError("subject requires an accepted or stopped lifecycle receipt")
    try:
        checkpoint_module(repository).validate_lifecycle_receipt(checkpoint, kind, require_paths=True)
    except SystemExit as error:
        raise ProfileError("slice-checkpoint rejected lifecycle receipt") from error
    except Exception as error:
        raise ProfileError(f"slice-checkpoint rejected lifecycle receipt: {error}") from error
    required = {
        "baseline_commit_oid", "baseline_tree_oid", "checkpoint_commit_oid", "checkpoint_tree_oid",
        "task_patch_digest", "manifest_digest", "run_id", "slice_number", "candidate_attempt",
        "plan_version", "scope_revision", "paths", "limitations",
    }
    missing = sorted(required - set(checkpoint))
    if missing:
        raise ProfileError(f"full lifecycle receipt is missing: {', '.join(missing)}")
    baseline_tree = git(repository, ["rev-parse", f"{checkpoint['baseline_commit_oid']}^{{tree}}"] ).strip()
    result_tree = git(repository, ["rev-parse", f"{checkpoint['checkpoint_commit_oid']}^{{tree}}"] ).strip()
    if baseline_tree != checkpoint["baseline_tree_oid"] or result_tree != checkpoint["checkpoint_tree_oid"]:
        raise ProfileError("receipt tree identity does not match immutable Git objects")
    patch = git(repository, [
        "diff-tree", "--binary", "--no-renames", "--no-ext-diff",
        baseline_tree, result_tree,
    ], binary=True)
    if hashlib.sha256(patch).hexdigest() != checkpoint["task_patch_digest"]:
        raise ProfileError("task patch digest does not match immutable trees")
    paths = [entry["path"] for entry in checkpoint["paths"]]
    if len(paths) != len(set(paths)):
        raise ProfileError("checkpoint manifest contains duplicate paths")
    identity_fields = required | {
        "schema_version", "checkpoint_id", "checkpoint_kind", "candidate_checkpoint_id",
        "parent_checkpoint_commit_oid", "gate_receipt_digest", "ref",
    }
    return {
        "schema_version": 1,
        "construction_method": raw["construction_method"],
        "evidence_cutoff": raw["evidence_cutoff"],
        "repository": str(repository),
        "checkpoint": {key: checkpoint[key] for key in sorted(identity_fields - {"limitations"})},
        "limitations": list(checkpoint["limitations"]),
    }


def category(path: str) -> str:
    name = Path(path).name.lower()
    suffix = Path(path).suffix.lower()
    parts = Path(path).parts
    if "tests" in parts or name.startswith("test_") or name.endswith("_test.py"):
        return "test"
    if suffix in {".md", ".rst", ".txt"}:
        return "documentation"
    if suffix in {".json", ".yaml", ".yml", ".toml", ".ini", ".cfg"} or name in {"dockerfile", "makefile"}:
        return "configuration"
    if suffix in {".py", ".js", ".mjs", ".ts", ".tsx", ".jsx", ".go", ".rs", ".java"}:
        return "source"
    return "other"


def symbol_map(source: str) -> dict[str, str]:
    tree = ast.parse(source)
    found: dict[str, str] = {}
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            found[node.name] = ast.dump(node, annotate_fields=True, include_attributes=False)
    return found


def blob(repository: Path, tree: str, path: str) -> str | None:
    executable = resolved_executable("git")
    result = subprocess.run(
        [str(executable), "-C", str(repository), "show", f"{tree}:{path}"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if result.returncode:
        return None
    return result.stdout.decode("utf-8")


def profile(raw: Any, repository_override: Path | None = None) -> dict[str, Any]:
    subject = validate_subject(raw, repository_override)
    checkpoint = subject["checkpoint"]
    repository = Path(subject["repository"])
    base = checkpoint["baseline_tree_oid"]
    result = checkpoint["checkpoint_tree_oid"]
    paths = sorted(entry["path"] for entry in checkpoint["paths"])
    numstat_text = git(repository, ["diff", "--no-renames", "--no-ext-diff", "--numstat", base, result, "--", *paths])
    stats: dict[str, dict[str, Any]] = {}
    for line in numstat_text.splitlines():
        additions, deletions, path = line.split("\t", 2)
        stats[path] = {
            "additions": None if additions == "-" else int(additions),
            "deletions": None if deletions == "-" else int(deletions),
            "binary": additions == "-",
        }
    patch_text = git(repository, ["diff", "--no-renames", "--no-ext-diff", "--unified=0", base, result, "--", *paths])
    hunks = 0
    for line in patch_text.splitlines():
        if line.startswith("@@ "):
            hunks += 1
    categories: dict[str, int] = {}
    modules: dict[str, int] = {}
    for path in paths:
        categories[category(path)] = categories.get(category(path), 0) + 1
        module = Path(path).parts[0] if len(Path(path).parts) > 1 else "."
        modules[module] = modules.get(module, 0) + 1
    symbol_files: list[dict[str, Any]] = []
    for path in paths:
        if Path(path).suffix != ".py":
            symbol_files.append({"path": path, "measurement": measurement("unsupported", reason="version 1 symbol analysis supports Python only")})
            continue
        try:
            before_source = blob(repository, base, path)
            after_source = blob(repository, result, path)
            before = symbol_map(before_source) if before_source is not None else {}
            after = symbol_map(after_source) if after_source is not None else {}
            changes = []
            for name in sorted(set(before) | set(after)):
                change = "added" if name not in before else "deleted" if name not in after else "modified" if before[name] != after[name] else None
                if change:
                    changes.append({"name": name, "change": change})
            symbol_files.append({"path": path, "measurement": measurement("observed", value=changes)})
        except (SyntaxError, UnicodeDecodeError) as error:
            error_id = hashlib.sha256(f"{type(error).__name__}:{path}".encode()).hexdigest()
            symbol_files.append({"path": path, "measurement": measurement("failed", reason=f"python_parse:{error_id}")})
    file_rows = []
    for path in paths:
        row = {"path": path, "category": category(path), **stats.get(path, {"additions": 0, "deletions": 0, "binary": False})}
        file_rows.append(row)
    observations = {
        "files": measurement("observed", value=file_rows),
        "file_count": measurement("observed", value=len(file_rows)),
        "line_totals": measurement(
            "unsupported" if any(row["binary"] for row in file_rows) else "observed",
            reason="binary diff has no textual line totals" if any(row["binary"] for row in file_rows) else None,
            value=None if any(row["binary"] for row in file_rows) else {
                "additions": sum(row["additions"] for row in file_rows),
                "deletions": sum(row["deletions"] for row in file_rows),
            },
        ),
        "hunk_count": measurement("observed", value=hunks),
        "file_categories": measurement("observed", value=dict(sorted(categories.items()))),
        "test_file_count": measurement("observed", value=categories.get("test", 0)),
        "documentation_file_count": measurement("observed", value=categories.get("documentation", 0)),
        "configuration_file_count": measurement("observed", value=categories.get("configuration", 0)),
        "changed_symbols": measurement("observed", value=symbol_files),
        "module_distribution": measurement("observed", value=dict(sorted(modules.items()))),
    }
    subject_digest = digest(subject)
    analyzer = {
        "name": ANALYZER_NAME,
        "version": ANALYZER_VERSION,
        "source_sha256": sha256_file(Path(__file__).resolve()),
        "checkpoint_validator_sha256": CHECKPOINT_VALIDATOR_SHA256,
    }
    provenance = {
        "producer": dict(analyzer),
        "derivation_sources": {
            "checkpoint_subject": {
                "use_state": "used",
                "identity": {"subject_digest": subject_digest},
            },
            "repository_trees": {
                "use_state": "used",
                "identity": {
                    "baseline_tree_oid": base,
                    "result_tree_oid": result,
                    "task_patch_digest": checkpoint["task_patch_digest"],
                },
            },
            "checkpoint_validator": {
                "use_state": "used",
                "identity": {"sha256": CHECKPOINT_VALIDATOR_SHA256},
            },
            "git_runtime": {
                "use_state": "used",
                "identity": git_runtime_identity(),
            },
            "python_runtime": {
                "use_state": "used",
                "identity": python_runtime_identity(),
            },
            "structural_graph": {
                "use_state": "not_used",
                "reason": "deferred_by_profile_scope",
            },
            "invariant_catalog": {
                "use_state": "not_used",
                "reason": "deferred_by_profile_scope",
            },
            "classifier": {
                "use_state": "not_used",
                "reason": "deferred_by_profile_scope",
            },
        },
    }
    result_profile = {
        "schema_version": 1,
        "analyzer": analyzer,
        "provenance": provenance,
        "subject": subject,
        "subject_digest": subject_digest,
        "observations": observations,
        "coverage": {"manifest_paths": len(paths), "profiled_paths": len(file_rows), "path_scope": "attributed checkpoint manifest"},
        "limitations": [
            "Renames are intentionally disabled in the deterministic baseline.",
            "Changed-symbol analysis is bounded to top-level Python definitions.",
            *subject["limitations"],
        ],
    }
    result_profile["profile_digest"] = digest(result_profile)
    return result_profile


def validate_profile(value: Any) -> dict[str, Any]:
    required = {
        "schema_version", "analyzer", "subject", "subject_digest", "observations",
        "coverage", "limitations", "provenance", "profile_digest",
    }
    if not isinstance(value, dict) or set(value) != required or value.get("schema_version") != 1:
        raise ProfileError("profile fields must exactly match schema version 1")
    analyzer = value["analyzer"]
    if not isinstance(analyzer, dict) or set(analyzer) != {
        "name", "version", "source_sha256", "checkpoint_validator_sha256"
    }:
        raise ProfileError("analyzer identity is invalid")
    if analyzer.get("name") != ANALYZER_NAME or analyzer.get("version") != ANALYZER_VERSION:
        raise ProfileError("analyzer name or version is unsupported")
    if analyzer.get("source_sha256") != sha256_file(Path(__file__).resolve()):
        raise ProfileError("analyzer source digest does not match this validator")
    if analyzer.get("checkpoint_validator_sha256") != CHECKPOINT_VALIDATOR_SHA256:
        raise ProfileError("checkpoint validator binding is invalid")
    validate_derived_subject(value["subject"])
    for field in ("subject_digest", "profile_digest"):
        item = value.get(field)
        if not isinstance(item, str) or len(item) != 64 or any(c not in "0123456789abcdef" for c in item):
            raise ProfileError(f"{field} must be lowercase SHA-256")
    if digest(value["subject"]) != value["subject_digest"]:
        raise ProfileError("subject digest does not match canonical subject")
    validate_provenance(value["provenance"], analyzer, value["subject_digest"], value["subject"])
    unsigned = dict(value)
    claimed = unsigned.pop("profile_digest")
    if digest(unsigned) != claimed:
        raise ProfileError("profile digest does not match canonical profile")
    validate_observations(value["observations"], value["subject"], value["coverage"])
    if not isinstance(value["limitations"], list) or not all(isinstance(x, str) for x in value["limitations"]):
        raise ProfileError("limitations must be an array of strings")
    return value


def validate_provenance(
    provenance: Any,
    analyzer: dict[str, Any],
    subject_digest: str,
    subject: dict[str, Any],
) -> None:
    if not isinstance(provenance, dict) or set(provenance) != {"producer", "derivation_sources"}:
        raise ProfileError("provenance structure is invalid")
    if provenance["producer"] != analyzer:
        raise ProfileError("provenance producer contradicts analyzer identity")
    sources = provenance["derivation_sources"]
    expected_names = {
        "checkpoint_subject", "repository_trees", "checkpoint_validator",
        "git_runtime", "python_runtime", "structural_graph", "invariant_catalog", "classifier",
    }
    if not isinstance(sources, dict) or set(sources) != expected_names:
        raise ProfileError("provenance derivation sources are incomplete or unsupported")
    checkpoint = subject["checkpoint"]
    expected_used = {
        "checkpoint_subject": {"subject_digest": subject_digest},
        "repository_trees": {
            "baseline_tree_oid": checkpoint["baseline_tree_oid"],
            "result_tree_oid": checkpoint["checkpoint_tree_oid"],
            "task_patch_digest": checkpoint["task_patch_digest"],
        },
        "checkpoint_validator": {"sha256": analyzer["checkpoint_validator_sha256"]},
    }
    for name, identity in expected_used.items():
        if sources[name] != {"use_state": "used", "identity": identity}:
            raise ProfileError(f"provenance source {name} contradicts derivation identity")
    expected_git = {"use_state": "used", "identity": git_runtime_identity()}
    if sources["git_runtime"] != expected_git:
        raise ProfileError("Git runtime provenance does not match the validating runtime")
    expected_python = {"use_state": "used", "identity": python_runtime_identity()}
    if sources["python_runtime"] != expected_python:
        raise ProfileError("Python runtime provenance does not match the validating runtime")
    for name in ("structural_graph", "invariant_catalog", "classifier"):
        source = sources[name]
        if source != {"use_state": "not_used", "reason": "deferred_by_profile_scope"}:
            if isinstance(source, dict) and source.get("use_state") == "used":
                identity = source.get("identity")
                if not isinstance(identity, dict) or not identity or any(not isinstance(v, str) or not v for v in identity.values()):
                    raise ProfileError(f"used provenance source {name} requires exact revision/source identity")
            raise ProfileError(f"version 1 provenance source {name} must remain deferred and unused")


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


def validate_derived_subject(subject: Any) -> None:
    fields = {"schema_version", "construction_method", "evidence_cutoff", "repository", "checkpoint", "limitations"}
    if not isinstance(subject, dict) or set(subject) != fields:
        raise ProfileError("derived subject structure is invalid")
    if subject["schema_version"] != 1 or subject["construction_method"] != "full_slice_checkpoint_lifecycle_receipt":
        raise ProfileError("derived subject identity is invalid")
    if not all(isinstance(subject[field], str) and subject[field] for field in ("evidence_cutoff", "repository")):
        raise ProfileError("derived subject strings must be nonempty")
    if not isinstance(subject["limitations"], list) or not all(isinstance(x, str) for x in subject["limitations"]):
        raise ProfileError("subject limitations are invalid")
    checkpoint = subject["checkpoint"]
    fields = {
        "schema_version", "checkpoint_id", "checkpoint_kind", "candidate_checkpoint_id",
        "parent_checkpoint_commit_oid", "baseline_commit_oid", "baseline_tree_oid",
        "checkpoint_commit_oid", "checkpoint_tree_oid", "task_patch_digest", "manifest_digest",
        "gate_receipt_digest", "run_id", "slice_number", "candidate_attempt", "plan_version",
        "scope_revision", "paths", "ref",
    }
    if not isinstance(checkpoint, dict) or set(checkpoint) != fields or checkpoint["schema_version"] != 1:
        raise ProfileError("derived checkpoint structure is invalid")
    if checkpoint["checkpoint_kind"] not in {"accepted", "stopped"}:
        raise ProfileError("derived checkpoint disposition is invalid")
    for field in ("checkpoint_id", "task_patch_digest", "manifest_digest", "gate_receipt_digest"):
        if not is_sha256(checkpoint[field]):
            raise ProfileError(f"derived checkpoint {field} is invalid")
    for field in ("baseline_commit_oid", "baseline_tree_oid", "checkpoint_commit_oid", "checkpoint_tree_oid", "parent_checkpoint_commit_oid"):
        item = checkpoint[field]
        if not isinstance(item, str) or len(item) not in {40, 64} or any(c not in "0123456789abcdef" for c in item):
            raise ProfileError(f"derived checkpoint {field} is invalid")
    for field in ("candidate_checkpoint_id", "run_id", "plan_version", "scope_revision", "ref"):
        if not isinstance(checkpoint[field], str) or not checkpoint[field]:
            raise ProfileError(f"derived checkpoint {field} is invalid")
    for field in ("slice_number", "candidate_attempt"):
        if isinstance(checkpoint[field], bool) or not isinstance(checkpoint[field], int) or checkpoint[field] < 1:
            raise ProfileError(f"derived checkpoint {field} is invalid")
    paths = checkpoint["paths"]
    if not isinstance(paths, list) or not paths:
        raise ProfileError("derived checkpoint paths are invalid")
    seen: set[str] = set()
    for entry in paths:
        if not isinstance(entry, dict) or set(entry) != {"path", "action", "attribution", "content_digest"}:
            raise ProfileError("derived checkpoint manifest entry is invalid")
        if not isinstance(entry["path"], str) or not entry["path"] or entry["path"] in seen:
            raise ProfileError("derived checkpoint manifest path is invalid")
        seen.add(entry["path"])
        if entry["action"] not in {"include", "delete"} or entry["attribution"] not in {
            "task_owned", "user_owned_baseline", "pre_existing_overlap", "generated_dependency", "validation_dependency"
        } or not is_sha256(entry["content_digest"]):
            raise ProfileError("derived checkpoint manifest semantics are invalid")


def validate_measurement(name: str, item: Any) -> str:
    if not isinstance(item, dict) or item.get("state") not in STATES:
        raise ProfileError(f"observation {name} has an invalid measurement state")
    state = item["state"]
    if state == "observed" and set(item) != {"state", "value"}:
        raise ProfileError(f"observed measurement {name} must carry only value")
    if state != "observed" and (
        set(item) != {"state", "reason"} or not isinstance(item.get("reason"), str) or not item["reason"]
    ):
        raise ProfileError(f"non-observed measurement {name} must carry only a reason")
    return state


def nonnegative_integer(value: Any) -> bool:
    return not isinstance(value, bool) and isinstance(value, int) and value >= 0


def validate_observations(observations: Any, subject: dict[str, Any], coverage: Any) -> None:
    names = {
        "files", "file_count", "line_totals", "hunk_count", "file_categories",
        "test_file_count", "documentation_file_count", "configuration_file_count",
        "changed_symbols", "module_distribution",
    }
    if not isinstance(observations, dict) or set(observations) != names:
        raise ProfileError("observation structure is invalid")
    for name, item in observations.items():
        validate_measurement(name, item)
    if any(observations[name]["state"] != "observed" for name in names - {"line_totals"}):
        raise ProfileError("version 1 baseline observations must be observed except line totals")
    files = observations["files"]["value"]
    if not isinstance(files, list):
        raise ProfileError("files observation must be an array")
    manifest_paths = sorted(entry["path"] for entry in subject["checkpoint"]["paths"])
    if [row.get("path") for row in files if isinstance(row, dict)] != manifest_paths:
        raise ProfileError("profiled files do not match attributed manifest")
    categories: dict[str, int] = {}
    modules: dict[str, int] = {}
    for row in files:
        if not isinstance(row, dict) or set(row) != {"path", "category", "additions", "deletions", "binary"}:
            raise ProfileError("file observation is invalid")
        if row["category"] not in {"test", "documentation", "configuration", "source", "other"} or not isinstance(row["binary"], bool):
            raise ProfileError("file category or binary state is invalid")
        if row["binary"]:
            if row["additions"] is not None or row["deletions"] is not None:
                raise ProfileError("binary file cannot fabricate line counts")
        elif not nonnegative_integer(row["additions"]) or not nonnegative_integer(row["deletions"]):
            raise ProfileError("text file line counts are invalid")
        categories[row["category"]] = categories.get(row["category"], 0) + 1
        module = Path(row["path"]).parts[0] if len(Path(row["path"]).parts) > 1 else "."
        modules[module] = modules.get(module, 0) + 1
    if observations["file_count"]["value"] != len(files):
        raise ProfileError("file count contradicts files observation")
    if not nonnegative_integer(observations["hunk_count"]["value"]):
        raise ProfileError("hunk count is invalid")
    expected_categories = dict(sorted(categories.items()))
    if observations["file_categories"]["value"] != expected_categories:
        raise ProfileError("file categories contradict files observation")
    for name, category_name in (("test_file_count", "test"), ("documentation_file_count", "documentation"), ("configuration_file_count", "configuration")):
        if observations[name]["value"] != categories.get(category_name, 0):
            raise ProfileError(f"{name} contradicts files observation")
    if observations["module_distribution"]["value"] != dict(sorted(modules.items())):
        raise ProfileError("module distribution contradicts files observation")
    binaries = any(row["binary"] for row in files)
    line_totals = observations["line_totals"]
    if binaries:
        if line_totals != {"state": "unsupported", "reason": "binary diff has no textual line totals"}:
            raise ProfileError("binary subject requires unsupported line totals")
    else:
        expected_lines = {"additions": sum(row["additions"] for row in files), "deletions": sum(row["deletions"] for row in files)}
        if line_totals != {"state": "observed", "value": expected_lines}:
            raise ProfileError("line totals contradict files observation")
    symbols = observations["changed_symbols"]["value"]
    if not isinstance(symbols, list) or [item.get("path") for item in symbols if isinstance(item, dict)] != manifest_paths:
        raise ProfileError("changed-symbol coverage contradicts manifest")
    for item in symbols:
        if not isinstance(item, dict) or set(item) != {"path", "measurement"}:
            raise ProfileError("changed-symbol entry is invalid")
        state = validate_measurement(f"changed_symbols:{item['path']}", item["measurement"])
        if state == "observed":
            changes = item["measurement"]["value"]
            if not isinstance(changes, list) or any(
                not isinstance(change, dict) or set(change) != {"name", "change"}
                or not isinstance(change["name"], str) or not change["name"]
                or change["change"] not in {"added", "deleted", "modified"} for change in changes
            ):
                raise ProfileError("changed-symbol value is invalid")
        elif state not in {"unsupported", "failed"}:
            raise ProfileError("changed-symbol measurement state is invalid")
    if not isinstance(coverage, dict) or set(coverage) != {"manifest_paths", "profiled_paths", "path_scope"}:
        raise ProfileError("coverage structure is invalid")
    if coverage != {"manifest_paths": len(manifest_paths), "profiled_paths": len(files), "path_scope": "attributed checkpoint manifest"}:
        raise ProfileError("coverage contradicts subject or observations")


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    command = subparsers.add_parser("profile")
    command.add_argument("--receipt", required=True, type=Path)
    command.add_argument("--repository", type=Path)
    validation = subparsers.add_parser("validate-profile")
    validation.add_argument("--profile", required=True, type=Path)
    args = parser.parse_args()
    try:
        if args.command == "profile":
            raw = json.loads(args.receipt.read_text())
            result = profile(raw, args.repository)
        else:
            result = validate_profile(json.loads(args.profile.read_text()))
    except (OSError, json.JSONDecodeError, ProfileError) as error:
        print(str(error), file=sys.stderr)
        return 1
    sys.stdout.buffer.write(canonical(result) + b"\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
