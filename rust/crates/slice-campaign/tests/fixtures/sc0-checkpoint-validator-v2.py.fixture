#!/usr/bin/env python3
"""Create isolated, Git-native slice checkpoints from explicit manifests."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ATTRIBUTIONS = {
    "task_owned", "user_owned_baseline", "pre_existing_overlap",
    "generated_dependency", "validation_dependency",
}
DIGEST = re.compile(r"[0-9a-f]{64}")
OID = re.compile(r"[0-9a-f]{40,64}")
IDENTITY = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*")
SENSITIVE_NAMES = {".env", "id_rsa", "id_dsa", "id_ed25519"}


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def git(repository: Path, args: list[str], *, env: dict[str, str] | None = None,
        input_bytes: bytes | None = None) -> bytes:
    complete = subprocess.run(
        ["git", "-C", str(repository), *args], input=input_bytes,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, check=False,
    )
    if complete.returncode:
        fail(f"git {' '.join(args)}: {complete.stderr.decode(errors='replace').strip()}")
    return complete.stdout


def text_git(repository: Path, args: list[str], **kwargs: Any) -> str:
    return git(repository, args, **kwargs).decode().strip()


def validate_identity(value: Any, path: str) -> str:
    if not isinstance(value, str) or not IDENTITY.fullmatch(value):
        fail(f"{path} must be a safe nonempty identity")
    return value


def validate_oid(repository: Path, value: Any, path: str) -> str:
    if not isinstance(value, str) or not OID.fullmatch(value):
        fail(f"{path} must be a Git object id")
    resolved = text_git(repository, ["rev-parse", f"{value}^{{commit}}"])
    if resolved != value:
        fail(f"{path} must name a commit")
    return value


def validate_digest(value: Any, path: str) -> str:
    if not isinstance(value, str) or not DIGEST.fullmatch(value):
        fail(f"{path} must be lowercase SHA-256")
    return value


def validate_relative_path(value: Any) -> str:
    if not isinstance(value, str) or not value or "\\" in value:
        fail("manifest path must be a nonempty repository-relative POSIX path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        fail(f"unsafe manifest path: {value}")
    if path.name in SENSITIVE_NAMES or path.suffix.lower() in {".pem", ".key", ".p12", ".pfx"}:
        fail(f"sensitive-looking manifest path is excluded: {value}")
    return str(path)


def validate_request(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("checkpoint request must be an object")
    required = {
        "schema_version", "repository", "run_id", "slice_number",
        "candidate_attempt", "baseline_commit_oid", "parent_checkpoint_commit_oid",
        "plan_version", "scope_revision", "gate_receipt_digest", "created_at", "paths",
    }
    if set(value) != required:
        fail("checkpoint request fields must exactly match schema version 1")
    if value["schema_version"] != 1:
        fail("schema_version must be 1")
    repository = Path(value["repository"]).resolve()
    if not repository.is_dir() or text_git(repository, ["rev-parse", "--show-toplevel"]) != str(repository):
        fail("repository must be a Git worktree root")
    validate_identity(value["run_id"], "run_id")
    for field in ("slice_number", "candidate_attempt"):
        if isinstance(value[field], bool) or not isinstance(value[field], int) or value[field] < 1:
            fail(f"{field} must be a positive integer")
    validate_oid(repository, value["baseline_commit_oid"], "baseline_commit_oid")
    parent = value["parent_checkpoint_commit_oid"]
    if parent is not None:
        validate_oid(repository, parent, "parent_checkpoint_commit_oid")
    for field in ("plan_version", "scope_revision", "created_at"):
        if not isinstance(value[field], str) or not value[field].strip():
            fail(f"{field} must be a nonempty string")
    validate_digest(value["gate_receipt_digest"], "gate_receipt_digest")
    if not isinstance(value["paths"], list) or not value["paths"]:
        fail("paths must be a nonempty array")
    seen: set[str] = set()
    for entry in value["paths"]:
        if not isinstance(entry, dict) or set(entry) != {"path", "action", "attribution"}:
            fail("each manifest entry requires path, action, and attribution")
        path = validate_relative_path(entry["path"])
        if path in seen:
            fail(f"duplicate manifest path: {path}")
        seen.add(path)
        if entry["action"] not in {"include", "delete"}:
            fail(f"invalid action for {path}")
        if entry["attribution"] not in ATTRIBUTIONS:
            fail(f"invalid attribution for {path}")
        ignored = subprocess.run(
            ["git", "-C", str(repository), "check-ignore", "--no-index", "--quiet", "--", path],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
        ).returncode == 0
        if ignored:
            fail(f"ignored manifest path is excluded: {path}")
    result = dict(value)
    result["repository"] = str(repository)
    return result


def private_ref(request: dict[str, Any], kind: str, attempt: int | None = None) -> str:
    base = f"refs/work-engine/checkpoints/{request['run_id']}/slice-{request['slice_number']}"
    return f"{base}/{kind}-{attempt}" if attempt is not None else f"{base}/{kind}"


def current_ref(repository: Path, ref: str) -> str | None:
    complete = subprocess.run(
        ["git", "-C", str(repository), "rev-parse", "--verify", "--quiet", ref],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    return complete.stdout.decode().strip() if complete.returncode == 0 else None


def update_ref(repository: Path, ref: str, oid: str, expected: str | None) -> None:
    observed = current_ref(repository, ref)
    if observed == oid:
        return
    if observed != expected:
        fail(f"checkpoint ref conflict: {ref}")
    text_git(repository, ["update-ref", ref, oid, expected or ("0" * len(oid))])


def blob_for_path(repository: Path, relative: str) -> tuple[str, str, str]:
    parts = PurePosixPath(relative).parts
    descriptors: list[int] = []
    try:
        directory_fd = os.open(repository, os.O_RDONLY | os.O_DIRECTORY)
        descriptors.append(directory_fd)
        for part in parts[:-1]:
            try:
                directory_fd = os.open(
                    part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                    dir_fd=directory_fd,
                )
            except OSError as error:
                fail(f"manifest path traverses an unsafe directory: {relative}: {error}")
            descriptors.append(directory_fd)
        info = os.stat(parts[-1], dir_fd=directory_fd, follow_symlinks=False)
        if stat.S_ISLNK(info.st_mode):
            content = os.readlink(parts[-1], dir_fd=directory_fd).encode()
            mode = "120000"
        elif stat.S_ISREG(info.st_mode):
            file_fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
            descriptors.append(file_fd)
            with os.fdopen(os.dup(file_fd), "rb") as stream:
                content = stream.read()
            mode = "100755" if info.st_mode & stat.S_IXUSR else "100644"
        else:
            fail(f"unsupported checkpoint path type: {relative}")
    finally:
        for descriptor in reversed(descriptors):
            os.close(descriptor)
    oid = text_git(repository, ["hash-object", "-w", "--stdin"], input_bytes=content)
    return oid, mode, hashlib.sha256(content).hexdigest()


def commit(repository: Path, tree: str, parent: str, message: dict[str, Any], created_at: str) -> str:
    env = dict(os.environ)
    env.update({
        "GIT_AUTHOR_NAME": "Work Engine", "GIT_AUTHOR_EMAIL": "work-engine@local",
        "GIT_COMMITTER_NAME": "Work Engine", "GIT_COMMITTER_EMAIL": "work-engine@local",
        "GIT_AUTHOR_DATE": created_at, "GIT_COMMITTER_DATE": created_at,
    })
    return text_git(
        repository, ["commit-tree", tree, "-p", parent], env=env,
        input_bytes=canonical(message) + b"\n",
    )


def commit_record(repository: Path, oid: str) -> tuple[str, list[str], dict[str, Any]]:
    raw = git(repository, ["cat-file", "-p", oid]).decode()
    headers, separator, message = raw.partition("\n\n")
    if not separator:
        fail("checkpoint commit has no metadata message")
    tree = ""
    parents: list[str] = []
    for line in headers.splitlines():
        if line.startswith("tree "):
            tree = line.removeprefix("tree ")
        elif line.startswith("parent "):
            parents.append(line.removeprefix("parent "))
    try:
        metadata = json.loads(message)
    except json.JSONDecodeError as error:
        fail(f"checkpoint commit metadata is invalid: {error}")
    if not tree or not isinstance(metadata, dict):
        fail("checkpoint commit metadata is incomplete")
    return tree, parents, metadata


def validate_candidate_receipt(candidate: Any) -> dict[str, Any]:
    if not isinstance(candidate, dict) or candidate.get("checkpoint_kind") != "candidate":
        fail("candidate must be a candidate checkpoint receipt")
    repository = Path(candidate.get("repository", "")).resolve()
    validate_identity(candidate.get("run_id"), "run_id")
    validate_oid(repository, candidate.get("checkpoint_commit_oid"), "checkpoint_commit_oid")
    validate_oid(repository, candidate.get("baseline_commit_oid"), "baseline_commit_oid")
    ref = candidate.get("ref")
    expected_ref = private_ref(candidate, "candidate", candidate.get("candidate_attempt"))
    if ref != expected_ref or current_ref(repository, ref) != candidate["checkpoint_commit_oid"]:
        fail("candidate private ref does not match receipt")
    tree, parents, metadata = commit_record(repository, candidate["checkpoint_commit_oid"])
    expected_metadata = {
        "work_engine_checkpoint": 1, "kind": "candidate",
        "request_id": candidate.get("checkpoint_id"), "run_id": candidate.get("run_id"),
        "slice_number": candidate.get("slice_number"),
        "candidate_attempt": candidate.get("candidate_attempt"), "tree": tree,
        "task_patch_digest": candidate.get("task_patch_digest"),
        "manifest_digest": candidate.get("manifest_digest"),
        "gate_receipt_digest": candidate.get("gate_receipt_digest"),
        "plan_version": candidate.get("plan_version"),
        "scope_revision": candidate.get("scope_revision"),
    }
    if metadata != expected_metadata or tree != candidate.get("checkpoint_tree_oid"):
        fail("candidate receipt does not match immutable commit metadata")
    if hashlib.sha256(canonical(candidate.get("paths"))).hexdigest() != candidate.get("manifest_digest"):
        fail("candidate path attribution manifest does not match immutable metadata")
    if parents != [candidate.get("parent_checkpoint_commit_oid")]:
        fail("candidate parent does not match receipt")
    baseline_tree = text_git(repository, ["rev-parse", f"{candidate['baseline_commit_oid']}^{{tree}}"])
    patch = git(
        repository,
        ["diff-tree", "--binary", "--no-renames", "--no-ext-diff", baseline_tree, tree],
    )
    if hashlib.sha256(patch).hexdigest() != candidate.get("task_patch_digest"):
        fail("candidate task patch digest does not match immutable trees")
    return candidate


def validate_lifecycle_receipt(
    receipt: Any, kind: str, *, require_paths: bool = True
) -> dict[str, Any]:
    if not isinstance(receipt, dict) or receipt.get("checkpoint_kind") != kind:
        fail(f"checkpoint receipt must be {kind}")
    repository = Path(receipt.get("repository", "")).resolve()
    validate_oid(repository, receipt.get("checkpoint_commit_oid"), "checkpoint_commit_oid")
    ref_attempt = receipt.get("candidate_attempt") if kind == "stopped" else None
    expected_ref = private_ref(receipt, kind, ref_attempt)
    if receipt.get("ref") != expected_ref or current_ref(repository, expected_ref) != receipt["checkpoint_commit_oid"]:
        fail(f"{kind} private ref does not match receipt")
    tree, parents, metadata = commit_record(repository, receipt["checkpoint_commit_oid"])
    expected_metadata = {
        "work_engine_checkpoint": 1, "kind": kind,
        "candidate_checkpoint_id": receipt.get("candidate_checkpoint_id"),
        "run_id": receipt.get("run_id"), "slice_number": receipt.get("slice_number"),
        "candidate_attempt": receipt.get("candidate_attempt"), "tree": tree,
        "task_patch_digest": receipt.get("task_patch_digest"),
        "manifest_digest": receipt.get("manifest_digest"),
        "gate_receipt_digest": receipt.get("gate_receipt_digest"),
        "plan_version": receipt.get("plan_version"),
        "scope_revision": receipt.get("scope_revision"),
    }
    if metadata != expected_metadata or tree != receipt.get("checkpoint_tree_oid"):
        fail(f"{kind} receipt does not match immutable commit metadata")
    if require_paths:
        if not isinstance(receipt.get("paths"), list):
            fail(f"{kind} receipt must retain the attributed path manifest")
        if hashlib.sha256(canonical(receipt["paths"])).hexdigest() != receipt.get("manifest_digest"):
            fail(f"{kind} path attribution manifest does not match immutable metadata")
    if parents != [receipt.get("parent_checkpoint_commit_oid")]:
        fail(f"{kind} parent does not match receipt")
    if hashlib.sha256(canonical(metadata)).hexdigest() != receipt.get("checkpoint_id"):
        fail(f"{kind} checkpoint identity does not match metadata")
    return receipt


def create_candidate(raw: Any) -> dict[str, Any]:
    request = validate_request(raw)
    repository = Path(request["repository"])
    baseline = request["baseline_commit_oid"]
    baseline_tree = text_git(repository, ["rev-parse", f"{baseline}^{{tree}}"])
    included: list[dict[str, str]] = []
    with tempfile.NamedTemporaryFile(prefix="work-engine-index-", delete=True) as index:
        env = dict(os.environ, GIT_INDEX_FILE=index.name)
        Path(index.name).unlink(missing_ok=True)
        text_git(repository, ["read-tree", baseline_tree], env=env)
        for entry in request["paths"]:
            path = entry["path"]
            if entry["action"] == "delete":
                text_git(repository, ["update-index", "--force-remove", "--", path], env=env)
                included.append({**entry, "content_digest": hashlib.sha256(b"").hexdigest()})
                continue
            oid, mode, digest = blob_for_path(repository, path)
            text_git(repository, ["update-index", "--add", "--cacheinfo", f"{mode},{oid},{path}"], env=env)
            included.append({**entry, "content_digest": digest})
        tree = text_git(repository, ["write-tree"], env=env)
    patch = git(
        repository,
        ["diff-tree", "--binary", "--no-renames", "--no-ext-diff", baseline_tree, tree],
    )
    task_patch_digest = hashlib.sha256(patch).hexdigest()
    manifest_digest = hashlib.sha256(canonical(included)).hexdigest()
    request_id = hashlib.sha256(canonical(request)).hexdigest()
    parent = request["parent_checkpoint_commit_oid"] or baseline
    metadata = {
        "work_engine_checkpoint": 1, "kind": "candidate", "request_id": request_id,
        "run_id": request["run_id"], "slice_number": request["slice_number"],
        "candidate_attempt": request["candidate_attempt"], "tree": tree,
        "task_patch_digest": task_patch_digest,
        "manifest_digest": manifest_digest,
        "gate_receipt_digest": request["gate_receipt_digest"],
        "plan_version": request["plan_version"], "scope_revision": request["scope_revision"],
    }
    checkpoint_commit = commit(repository, tree, parent, metadata, request["created_at"])
    ref = private_ref(request, "candidate", request["candidate_attempt"])
    update_ref(repository, ref, checkpoint_commit, None)
    return {
        "schema_version": 1, "checkpoint_id": request_id, "checkpoint_kind": "candidate",
        "repository": str(repository), "run_id": request["run_id"],
        "slice_number": request["slice_number"], "candidate_attempt": request["candidate_attempt"],
        "baseline_commit_oid": baseline, "baseline_tree_oid": baseline_tree,
        "checkpoint_commit_oid": checkpoint_commit, "checkpoint_tree_oid": tree,
        "parent_checkpoint_commit_oid": parent, "plan_version": request["plan_version"],
        "scope_revision": request["scope_revision"],
        "gate_receipt_digest": request["gate_receipt_digest"],
        "task_patch_digest": task_patch_digest, "paths": included, "ref": ref,
        "manifest_digest": manifest_digest,
        "created_at": request["created_at"], "limitations": [],
    }


def transition(candidate: dict[str, Any], kind: str, *, expected_accepted: str | None = None) -> dict[str, Any]:
    if kind not in {"accepted", "stopped"}:
        fail("transition kind must be accepted or stopped")
    validate_candidate_receipt(candidate)
    repository = Path(candidate["repository"]).resolve()
    request = {
        "run_id": validate_identity(candidate["run_id"], "run_id"),
        "slice_number": candidate["slice_number"],
    }
    tree = candidate["checkpoint_tree_oid"]
    parent = candidate["checkpoint_commit_oid"]
    metadata = {
        "work_engine_checkpoint": 1, "kind": kind,
        "candidate_checkpoint_id": candidate["checkpoint_id"],
        "run_id": candidate["run_id"], "slice_number": candidate["slice_number"],
        "candidate_attempt": candidate["candidate_attempt"], "tree": tree,
        "task_patch_digest": candidate["task_patch_digest"],
        "manifest_digest": candidate["manifest_digest"],
        "gate_receipt_digest": candidate["gate_receipt_digest"],
        "plan_version": candidate["plan_version"],
        "scope_revision": candidate["scope_revision"],
    }
    lifecycle_commit = commit(repository, tree, parent, metadata, candidate["created_at"])
    ref = private_ref(
        request, kind, candidate["candidate_attempt"] if kind == "stopped" else None
    )
    update_ref(repository, ref, lifecycle_commit, expected_accepted if kind == "accepted" else None)
    result = dict(candidate)
    result.update({
        "checkpoint_kind": kind, "candidate_checkpoint_id": candidate["checkpoint_id"],
        "checkpoint_id": hashlib.sha256(canonical(metadata)).hexdigest(),
        "checkpoint_commit_oid": lifecycle_commit,
        "parent_checkpoint_commit_oid": parent, "ref": ref,
    })
    validate_lifecycle_receipt(result, kind)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request-json", required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(create_candidate(json.loads(args.request_json)), separators=(",", ":")))
    except (json.JSONDecodeError, OSError, ValueError) as error:
        print(f"checkpoint: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
