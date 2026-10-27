#!/usr/bin/env python3
"""Create an explicitly approved, user-visible commit for an accepted slice."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import stat
from pathlib import Path
from typing import Any, Callable


STATES = {"pending", "declined", "created", "refused"}
RECEIPT_FIELDS = {
    "schema_version", "state", "repository", "run_id", "slice_number",
    "proposal", "proposal_digest", "expected_branch", "expected_head_oid",
    "commit_oid", "reason",
}
REQUEST_CODEC = "completion-offer-request-v1"
PRESERVATION_FIELDS = {
    "mode", "worktree", "expected_index_sha256", "accepted_receipt",
    "approved_delta", "manifest", "attempt_directory",
}


def request_canonical(value: Any) -> bytes:
    # Match the offer owner's existing UTF-8 identity; do not change proposal v1.
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def request_binding(request: dict[str, Any]) -> dict[str, Any]:
    return {"schema_version": 1, "canonical_contract": REQUEST_CODEC,
            "request": request, "sha256": sha256(request_canonical(request))}


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def git(repository: Path, args: list[str], *, env: dict[str, str] | None = None,
        input_text: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repository), *args], env=env, input=input_text,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
    )


def text_git(repository: Path, args: list[str], **kwargs: Any) -> str:
    result = git(repository, args, **kwargs)
    if result.returncode:
        fail(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def validate_oid(value: Any, field: str) -> str:
    if (not isinstance(value, str) or len(value) not in {40, 64}
            or any(character not in "0123456789abcdef" for character in value)):
        fail(f"{field} must be a lowercase Git object id")
    return value


def validate_provenance(value: Any) -> dict[str, Any]:
    fields = {"schema_version", "producer", "evidence"}
    if not isinstance(value, dict) or set(value) != fields:
        fail("proposal.provenance fields must exactly match schema version 1")
    if value["schema_version"] != 1:
        fail("proposal.provenance.schema_version must be 1")
    if not isinstance(value["producer"], str) or not value["producer"].strip():
        fail("proposal.provenance.producer must be nonempty")
    if not isinstance(value["evidence"], list) or not value["evidence"]:
        fail("proposal.provenance.evidence must be a nonempty array")
    for index, item in enumerate(value["evidence"]):
        if not isinstance(item, dict) or set(item) != {"kind", "digest"}:
            fail(f"proposal.provenance.evidence[{index}] must contain kind and digest")
        if not isinstance(item["kind"], str) or not item["kind"].strip():
            fail(f"proposal.provenance.evidence[{index}].kind must be nonempty")
        digest = item["digest"]
        if (not isinstance(digest, str) or len(digest) != 64
                or any(character not in "0123456789abcdef" for character in digest)):
            fail(f"proposal.provenance.evidence[{index}].digest must be lowercase SHA-256")
    return value


def validate_proposal(value: Any) -> dict[str, Any]:
    common = {"schema_version", "subject", "body", "paths", "checkpoint_commit_oid",
              "checkpoint_tree_oid", "task_patch_digest"}
    if not isinstance(value, dict):
        fail("proposal must be an object")
    if value.get("schema_version") == 1:
        fields = common | {"origin"}
        if set(value) != fields or value["origin"] != "completing_builder":
            fail("historical proposal schema version 1 requires completing_builder origin")
    elif value.get("schema_version") == 2:
        fields = common | {"provenance"}
        if set(value) != fields:
            fail("proposal fields must exactly match schema version 2")
        validate_provenance(value["provenance"])
    else:
        fail("proposal must use supported schema version 1 or 2")
    for field in ("subject", "checkpoint_commit_oid", "checkpoint_tree_oid", "task_patch_digest"):
        if not isinstance(value[field], str) or not value[field].strip():
            fail(f"proposal.{field} must be nonempty")
    validate_oid(value["checkpoint_commit_oid"], "proposal.checkpoint_commit_oid")
    validate_oid(value["checkpoint_tree_oid"], "proposal.checkpoint_tree_oid")
    if (len(value["task_patch_digest"]) != 64
            or any(character not in "0123456789abcdef" for character in value["task_patch_digest"])):
        fail("proposal.task_patch_digest must be lowercase SHA-256")
    if "\n" in value["subject"]:
        fail("proposal.subject must be one line")
    if not isinstance(value["body"], str):
        fail("proposal.body must be a string")
    if not isinstance(value["paths"], list) or not value["paths"] or not all(
        isinstance(path, str) and path and not path.startswith("/") for path in value["paths"]
    ) or len(set(value["paths"])) != len(value["paths"]):
        fail("proposal.paths must be unique nonempty repository-relative paths")
    return value


def receipt(request: dict[str, Any], state: str, *, reason: str | None,
            commit_oid: str | None = None) -> dict[str, Any]:
    if state not in STATES:
        fail("invalid completion-commit state")
    proposal = validate_proposal(request["proposal"])
    result = {
        "schema_version": 1,
        "state": state,
        "repository": str(Path(request["repository"]).resolve()),
        "run_id": request["run_id"],
        "slice_number": request["slice_number"],
        "proposal": proposal,
        "proposal_digest": hashlib.sha256(canonical(proposal)).hexdigest(),
        "expected_branch": request["expected_branch"],
        "expected_head_oid": request["expected_head_oid"],
        "commit_oid": commit_oid,
        "reason": reason,
    }
    if "preservation" in request:
        result.update(schema_version=2, request_binding=request_binding(request), evidence=None)
    return result


def validate_receipt_shape(value: Any) -> dict[str, Any]:
    """Pure contract validation shared by terminal projections, not Git proof."""
    version = value.get("schema_version") if isinstance(value, dict) else None
    fields = RECEIPT_FIELDS | ({"request_binding", "evidence"} if version == 2 else set())
    if not isinstance(value, dict) or set(value) != fields:
        fail("completion-commit receipt fields must exactly match the contract")
    if version not in {1, 2} or value["state"] not in STATES:
        fail("completion-commit receipt must use a supported schema and valid state")
    for field in ("repository", "run_id", "expected_branch", "expected_head_oid"):
        if not isinstance(value[field], str) or not value[field].strip():
            fail(f"completion-commit receipt.{field} must be nonempty")
    validate_oid(value["expected_head_oid"], "completion-commit receipt.expected_head_oid")
    if (isinstance(value["slice_number"], bool)
            or not isinstance(value["slice_number"], int)
            or value["slice_number"] < 1):
        fail("completion-commit receipt.slice_number must be positive")
    proposal = validate_proposal(value["proposal"])
    if value["proposal_digest"] != hashlib.sha256(canonical(proposal)).hexdigest():
        fail("completion-commit receipt proposal digest does not match")
    if version == 2:
        binding = value["request_binding"]
        if not isinstance(binding, dict) or set(binding) != {
            "schema_version", "canonical_contract", "request", "sha256"
        }:
            fail("receipt request_binding is required")
        request = validate_request(binding["request"])
        if "preservation" not in request or binding != request_binding(request):
            fail("receipt request_binding codec or digest does not match")
        for field in ("repository", "run_id", "slice_number", "proposal",
                      "expected_branch", "expected_head_oid"):
            if value[field] != request[field]:
                fail(f"receipt request_binding.{field} does not match")
        evidence = value["evidence"]
        if value["state"] == "created":
            validate_effect_shape(evidence)
        elif evidence is not None:
            fail("non-created receipt cannot claim publication evidence")

    state = value["state"]
    if state == "pending":
        if value["commit_oid"] is not None or value["reason"] is not None:
            fail("pending completion-commit receipt cannot claim a commit or reason")
        return value
    if state in {"declined", "refused"}:
        if value["commit_oid"] is not None or not isinstance(value["reason"], str) or not value["reason"]:
            fail(f"{state} completion-commit receipt requires a reason and no commit")
        return value
    if not isinstance(value["commit_oid"], str) or not value["commit_oid"]:
        fail("created completion-commit receipt requires a commit")
    validate_oid(value["commit_oid"], "completion-commit receipt.commit_oid")
    if value["reason"] is not None and not isinstance(value["reason"], str):
        fail("created completion-commit receipt reason must be null or a string")
    return value


def validate_receipt(value: Any) -> dict[str, Any]:
    """Re-establish evidence without upgrading actor records to attestation."""
    validate_receipt_shape(value)
    if value["state"] != "created":
        return value
    proposal = value["proposal"]

    repository = Path(value["repository"]).resolve()
    if str(repository) != value["repository"]:
        fail("completion-commit repository must be canonical")
    root = Path(text_git(repository, ["rev-parse", "--show-toplevel"])).resolve()
    if root != repository:
        fail("completion-commit repository does not match the Git worktree root")
    commit_oid = text_git(repository, ["rev-parse", "--verify", f"{value['commit_oid']}^{{commit}}"])
    if commit_oid != value["commit_oid"]:
        fail("created completion commit object does not match the receipt")
    checkpoint_oid = text_git(
        repository, ["rev-parse", "--verify", f"{proposal['checkpoint_commit_oid']}^{{commit}}"]
    )
    if checkpoint_oid != proposal["checkpoint_commit_oid"]:
        fail("accepted checkpoint object does not match the proposal")
    if text_git(repository, ["rev-parse", f"{checkpoint_oid}^{{tree}}"] ) != proposal["checkpoint_tree_oid"]:
        fail("accepted checkpoint tree does not match the proposal")
    if text_git(repository, ["rev-parse", f"{commit_oid}^" ]) != value["expected_head_oid"]:
        fail("created completion commit parent does not match the receipt")
    if text_git(repository, ["rev-parse", f"{commit_oid}^{{tree}}"] ) != proposal["checkpoint_tree_oid"]:
        fail("created completion commit tree does not match the proposal")
    expected_message = proposal["subject"] + (
        f"\n\n{proposal['body']}" if proposal["body"] else ""
    )
    if text_git(repository, ["show", "-s", "--format=%B", commit_oid]) != expected_message:
        fail("created completion commit message does not match the proposal")
    branch_ref = f"refs/heads/{value['expected_branch']}"
    branch_tip = text_git(repository, ["rev-parse", "--verify", branch_ref])
    if text_git(repository, ["symbolic-ref", "--quiet", "HEAD"]) != branch_ref:
        fail("completion commit resulting branch attachment does not match the receipt")
    if text_git(repository, ["rev-parse", "HEAD"]) != branch_tip:
        fail("completion commit resulting HEAD does not match the receipt")
    if branch_tip != commit_oid:
        ancestry = git(repository, ["merge-base", "--is-ancestor", commit_oid, branch_tip])
        if ancestry.returncode:
            fail("completion commit publication target does not contain the recorded commit")
    if value["schema_version"] == 2:
        validate_effect(value)
    return value


def validate_request(raw: Any) -> dict[str, Any]:
    required = {"repository", "run_id", "slice_number", "expected_branch",
                "expected_head_oid", "accepted_paths", "proposal"}
    if not isinstance(raw, dict) or set(raw) not in {
        frozenset(required), frozenset(required | {"operational_paths"}),
        frozenset(required | {"preservation"}),
    }:
        fail("request fields must exactly match the completion-commit contract")
    for field in ("repository", "run_id", "expected_branch", "expected_head_oid"):
        if not isinstance(raw[field], str) or not raw[field].strip():
            fail(f"request.{field} must be nonempty")
    validate_oid(raw["expected_head_oid"], "request.expected_head_oid")
    if isinstance(raw["slice_number"], bool) or not isinstance(raw["slice_number"], int) or raw["slice_number"] < 1:
        fail("request.slice_number must be positive")
    validate_proposal(raw["proposal"])
    if not isinstance(raw["accepted_paths"], list) or not all(
        isinstance(entry, dict) and isinstance(entry.get("path"), str)
        and entry.get("action") in {"include", "delete"} for entry in raw["accepted_paths"]
    ):
        fail("request.accepted_paths must be an accepted checkpoint path manifest")
    accepted_names = {entry["path"] for entry in raw["accepted_paths"]}
    if len(accepted_names) != len(raw["accepted_paths"]):
        fail("request.accepted_paths must not contain duplicate paths")
    if accepted_names != set(raw["proposal"]["paths"]):
        fail("proposal paths must exactly match the accepted checkpoint manifest")
    operational_paths = raw.get("operational_paths", [])
    if (not isinstance(operational_paths, list)
            or not all(isinstance(path, str) and path and not path.startswith("/")
                       and ".." not in Path(path).parts for path in operational_paths)
            or len(set(operational_paths)) != len(operational_paths)
            or accepted_names.intersection(operational_paths)):
        fail("request.operational_paths must be distinct safe paths outside the proposal")
    if "preservation" in raw:
        validate_preservation_request(raw)
    return raw


def reconcile_created(raw: Any) -> dict[str, Any] | None:
    """Recover an authoritative created result without replaying publication."""
    request = validate_request(raw)
    if "preservation" in request:
        return reconcile_preserved(request)
    repository = Path(request["repository"]).resolve()
    branch_ref = f"refs/heads/{request['expected_branch']}"
    observed = git(repository, ["rev-parse", "--verify", "--quiet", branch_ref])
    if observed.returncode:
        return None
    commit_oid = observed.stdout.strip()
    if commit_oid == request["expected_head_oid"]:
        return None
    candidate = receipt(request, "created", reason=None, commit_oid=commit_oid)
    try:
        return validate_receipt(candidate)
    except ValueError as error:
        fail(f"completion publication state is ambiguous: {error}")


def decide(raw: Any, decision: str, *, after_publish: Callable[[str], None] | None = None) -> dict[str, Any]:
    request = validate_request(raw)
    if decision == "pending":
        return receipt(request, "pending", reason=None)
    if decision == "decline":
        return receipt(request, "declined", reason="user_declined")
    if decision != "create":
        fail("decision must be pending, decline, or create")
    if "preservation" in request:
        return create_preserved(request, after_publish=after_publish)

    repository = Path(request["repository"]).resolve()
    proposal = request["proposal"]
    commit_oid: str | None = None
    branch_updated = False
    try:
        branch = text_git(repository, ["symbolic-ref", "--quiet", "--short", "HEAD"])
        head = text_git(repository, ["rev-parse", "HEAD"])
        if branch != request["expected_branch"]:
            fail("current branch does not match the approved branch")
        if head != request["expected_head_oid"]:
            fail("current HEAD does not match the approved parent")
        checkpoint = text_git(repository, ["rev-parse", "--verify", proposal["checkpoint_commit_oid"]])
        tree = text_git(repository, ["rev-parse", f"{checkpoint}^{{tree}}"])
        if tree != proposal["checkpoint_tree_oid"]:
            fail("accepted checkpoint tree does not match the proposal")
        delta = git(
            repository,
            ["diff", "--name-status", "--no-renames", "-z",
             request["expected_head_oid"], proposal["checkpoint_tree_oid"], "--"],
        )
        if delta.returncode:
            fail(delta.stderr.strip() or "cannot inspect complete commit delta")
        parts = delta.stdout.split("\0")
        if parts[-1] != "" or len(parts) % 2 != 1:
            fail("cannot interpret complete commit delta")
        actual_actions = {
            parts[index + 1]: "delete" if parts[index] == "D" else "include"
            for index in range(0, len(parts) - 1, 2)
        }
        approved_actions = {entry["path"]: entry["action"] for entry in request["accepted_paths"]}
        if actual_actions != approved_actions:
            fail("complete commit delta does not match the approved path actions")
        real_index_tree = text_git(repository, ["write-tree"])
        head_tree = text_git(repository, ["rev-parse", "HEAD^{tree}"])
        if real_index_tree != head_tree:
            fail("real index contains staged user state")
        signing = git(repository, ["config", "--bool", "--get", "commit.gpgsign"])
        if signing.returncode == 0 and signing.stdout.strip() == "true":
            fail("signed commits require explicit repository-specific support")
        hooks_path = Path(text_git(repository, ["rev-parse", "--git-path", "hooks"]))
        if not hooks_path.is_absolute():
            hooks_path = repository / hooks_path
        hook_names = ("pre-commit", "prepare-commit-msg", "commit-msg", "post-commit")
        if any((hooks_path / name).is_file() and os.access(hooks_path / name, os.X_OK) for name in hook_names):
            fail("active commit hooks require explicit repository-specific support")

        with tempfile.NamedTemporaryFile(prefix="work-engine-completion-index-", delete=True) as index:
            env = dict(os.environ, GIT_INDEX_FILE=index.name)
            Path(index.name).unlink(missing_ok=True)
            text_git(repository, ["read-tree", "HEAD"], env=env)
            add = git(repository, ["add", "-A", "--", "."], env=env)
            if add.returncode:
                fail(add.stderr.strip() or "cannot inspect working tree")
            for path in request.get("operational_paths", []):
                reset = git(repository, ["reset", "-q", "HEAD", "--", path], env=env)
                if reset.returncode:
                    fail(reset.stderr.strip() or "cannot exclude declared operational state")
            worktree_tree = text_git(repository, ["write-tree"], env=env)
            if worktree_tree != proposal["checkpoint_tree_oid"]:
                fail("working tree does not exactly match the accepted checkpoint")
            message = proposal["subject"] + (f"\n\n{proposal['body']}" if proposal["body"] else "") + "\n"
        commit_oid = text_git(
            repository,
            ["commit-tree", proposal["checkpoint_tree_oid"], "-p", request["expected_head_oid"]],
            input_text=message,
        )
        if text_git(repository, ["rev-parse", f"{commit_oid}^" ]) != request["expected_head_oid"]:
            fail("created commit has an unexpected parent")
        if text_git(repository, ["rev-parse", f"{commit_oid}^{{tree}}"] ) != proposal["checkpoint_tree_oid"]:
            fail("created commit has an unexpected tree")
        actual_message = text_git(repository, ["show", "-s", "--format=%B", commit_oid])
        if actual_message != message.rstrip("\n"):
            fail("created commit has an unexpected message")
        if text_git(repository, ["symbolic-ref", "--quiet", "HEAD"]) != f"refs/heads/{request['expected_branch']}":
            fail("approved branch changed before publication")
        update = git(
            repository,
            ["update-ref", f"refs/heads/{request['expected_branch']}", commit_oid, request["expected_head_oid"]],
        )
        if update.returncode:
            fail("approved branch moved before atomic publication")
        branch_updated = True
        if after_publish is not None:
            after_publish(commit_oid)
        text_git(repository, ["read-tree", commit_oid])
        if text_git(repository, ["symbolic-ref", "--quiet", "--short", "HEAD"]) != request["expected_branch"]:
            fail("branch attachment changed after commit creation")
        pathspec = [".", *[f":(exclude){path}" for path in request.get("operational_paths", [])]]
        status = git(repository, ["status", "--porcelain=v2", "--untracked-files=all", "--", *pathspec])
        if status.returncode or status.stdout:
            fail("ordinary Git state is not clean after commit creation")
        return receipt(request, "created", reason=None, commit_oid=commit_oid)
    except ValueError as error:
        if branch_updated:
            return receipt(
                request, "created", commit_oid=commit_oid,
                reason=f"postcondition_failed: {error}",
            )
        return receipt(request, "refused", reason=str(error))


def safe_path(value: Any) -> str:
    if (not isinstance(value, str) or not value or "\x00" in value
            or value.startswith("/") or any(p in {"", ".", "..", ".git"} for p in value.split("/"))):
        fail("preservation path must be a safe literal repository-relative path")
    return value


def digest(value: Any) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
        fail("evidence digest must be lowercase SHA-256")
    return value


def external_path(value: Any, repository: Path) -> Path:
    if not isinstance(value, str) or not Path(value).is_absolute():
        fail("evidence path must be absolute")
    path = Path(value)
    if path.resolve() != path or path == repository or repository in path.parents:
        fail("evidence must be canonical and external to the worktree")
    return path


def validate_reference(value: Any, repository: Path) -> None:
    if not isinstance(value, dict) or set(value) != {"path", "sha256"}:
        fail("evidence reference requires exactly path and sha256")
    external_path(value["path"], repository)
    digest(value["sha256"])


def validate_preservation_request(request: dict[str, Any]) -> None:
    p = request["preservation"]
    if not isinstance(p, dict) or set(p) != PRESERVATION_FIELDS or p["mode"] != "preserve_unrelated_v1":
        fail("preservation fields must exactly match preserve_unrelated_v1")
    repository = Path(request["repository"])
    if not repository.is_absolute() or str(repository.resolve()) != str(repository) or p["worktree"] != str(repository):
        fail("preservation repository/worktree identity must be canonical")
    if request["proposal"]["schema_version"] != 2:
        fail("preservation requires proposal schema version 2")
    digest(p["expected_index_sha256"])
    validate_reference(p["accepted_receipt"], repository)
    validate_reference(p["manifest"], repository)
    attempt = external_path(p["attempt_directory"], repository)
    for ref in (p["accepted_receipt"], p["manifest"]):
        path = Path(ref["path"])
        if path == attempt or attempt in path.parents:
            fail("attempt directory must not contain upstream authority evidence")
    paths = sorted(safe_path(x) for x in request["proposal"]["paths"])
    if any(b.startswith(a + "/") for a, b in zip(paths, paths[1:])):
        fail("approved paths have a directory/file prefix collision")
    if any(set(x) != {"path", "action"} for x in request["accepted_paths"]):
        fail("preservation accepted_paths requires exact path/action entries")
    delta = p["approved_delta"]
    if not isinstance(delta, list) or len(delta) != len(paths):
        fail("approved full delta must contain every approved path")
    actions = {x["path"]: x["action"] for x in request["accepted_paths"]}
    seen = set()
    for x in delta:
        if not isinstance(x, dict) or set(x) != {"path", "action", "before", "after"}:
            fail("approved delta fields must be exact")
        path = safe_path(x["path"])
        if path in seen or actions.get(path) != x["action"]:
            fail("approved delta path/action mismatch")
        seen.add(path)
        for key in ("before", "after"):
            entry = x[key]
            if entry is not None:
                if not isinstance(entry, dict) or set(entry) != {"mode", "oid"} or entry["mode"] not in {"100644", "100755"}:
                    fail("approved delta supports regular files only")
                validate_oid(entry["oid"], "approved delta oid")
        if ((x["action"] == "delete") != (x["after"] is None)
                or x["before"] == x["after"]):
            fail("approved delta must describe an actual include/delete")


def git_bytes(repository: Path, args: list[str], *, env=None, input_bytes=None) -> bytes:
    result = subprocess.run(["git", "-C", str(repository), *args], env=env,
                            input=input_bytes, capture_output=True, check=False)
    if result.returncode:
        fail(result.stderr.decode(errors="replace").strip() or "Git inspection failed")
    return result.stdout


def tree_entries(repository: Path, tree: str) -> dict[str, dict[str, str]]:
    entries = {}
    for raw in git_bytes(repository, ["ls-tree", "-r", "-z", tree]).split(b"\0"):
        if not raw:
            continue
        meta, name = raw.split(b"\t", 1)
        mode, _kind, oid = meta.decode().split()
        entries[name.decode()] = {"mode": mode, "oid": oid}
    return entries


def full_delta(repository: Path, parent: str, tree: str) -> list[dict[str, Any]]:
    before, after = tree_entries(repository, parent), tree_entries(repository, tree)
    return [{"path": p, "action": "include" if p in after else "delete",
             "before": before.get(p), "after": after.get(p)}
            for p in sorted(before.keys() | after.keys()) if before.get(p) != after.get(p)]


def index_path(repository: Path) -> Path:
    return Path(text_git(repository, ["rev-parse", "--path-format=absolute", "--git-path", "index"]))


def index_entries(repository: Path, env=None) -> dict[str, dict[str, str]]:
    entries = {}
    tags = {}
    for raw in git_bytes(repository, ["ls-files", "-v", "-z"], env=env).split(b"\0"):
        if raw:
            tags[raw[2:].decode()] = raw[:1].decode()
    for raw in git_bytes(repository, ["ls-files", "--stage", "-z"], env=env).split(b"\0"):
        if not raw:
            continue
        meta, name = raw.split(b"\t", 1)
        mode, oid, stage = meta.decode().split()
        path = safe_path(name.decode())
        if stage != "0" or tags.get(path) != "H" or mode not in {"100644", "100755", "120000"}:
            fail("unsupported unmerged, flagged, sparse or submodule index entry")
        entries[path] = {"mode": mode, "oid": oid, "tag": tags[path]}
    # --debug exposes intent-to-add, which shares the empty blob with real files.
    debug = git_bytes(repository, ["ls-files", "--debug"], env=env).decode()
    for line in debug.splitlines():
        if "flags: " in line and int(line.rsplit("flags: ", 1)[1], 16) != 0:
            fail("unsupported extended index flags (including intent-to-add)")
    return entries


def working_entry(repository: Path, name: str, *, nested_repository: bool = False) -> dict[str, Any]:
    path = repository / safe_path(name)
    for parent in path.parents:
        if parent == repository:
            break
        if parent.is_symlink():
            fail("working path has a symlink ancestor")
    try:
        info = path.lstat()
    except FileNotFoundError:
        if nested_repository:
            fail("nested repository boundary is missing")
        return {"kind": "missing"}
    mode = stat.S_IMODE(info.st_mode)
    if nested_repository and not stat.S_ISDIR(info.st_mode):
        fail("nested repository boundary is no longer a directory")
    if stat.S_ISLNK(info.st_mode):
        return {"kind": "symlink", "mode": mode, "target": os.readlink(path)}
    if stat.S_ISREG(info.st_mode):
        return {"kind": "file", "mode": mode, "sha256": sha256(path.read_bytes())}
    if nested_repository:
        # Git reports an untracked nested repository as one slash-terminated
        # path. Bind its boundary without descending into its own worktree or
        # following its .git pointer into a separate Git directory.
        marker = path / ".git"
        try:
            marker_info = marker.lstat()
        except FileNotFoundError:
            fail("nested repository boundary has no .git marker")
        marker_entry: dict[str, Any] = {
            "mode": stat.S_IMODE(marker_info.st_mode),
            "dev": marker_info.st_dev, "ino": marker_info.st_ino,
        }
        if stat.S_ISDIR(marker_info.st_mode):
            marker_entry["kind"] = "directory"
        elif stat.S_ISREG(marker_info.st_mode):
            descriptor = os.open(marker, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
            try:
                opened = os.fstat(descriptor)
                if (not stat.S_ISREG(opened.st_mode) or opened.st_dev != marker_info.st_dev
                        or opened.st_ino != marker_info.st_ino):
                    fail("nested repository .git marker changed during observation")
                with os.fdopen(descriptor, "rb", closefd=False) as stream:
                    marker_digest = sha256(stream.read())
            finally:
                os.close(descriptor)
            marker_entry.update(kind="file", sha256=marker_digest)
        else:
            fail("nested repository .git marker must be a regular file or directory")
        after_root, after_marker = path.lstat(), marker.lstat()
        if ((after_root.st_dev, after_root.st_ino) != (info.st_dev, info.st_ino)
                or (after_marker.st_dev, after_marker.st_ino) != (marker_info.st_dev, marker_info.st_ino)):
            fail("nested repository boundary changed during observation")
        return {"kind": "nested_repository", "mode": mode,
                "dev": info.st_dev, "ino": info.st_ino, "git_marker": marker_entry}
    fail("unsupported working path type or directory/file collision")


def preservation_snapshot(repository: Path, approved_paths: list[str]) -> dict[str, Any]:
    entries = index_entries(repository)
    names = set(entries) | set(tree_entries(repository, "HEAD")) | set(approved_paths)
    nested = set()
    for raw in git_bytes(repository, ["ls-files", "--others", "--exclude-standard", "-z"]).split(b"\0"):
        if not raw:
            continue
        name = raw.decode()
        if name.endswith("/"):
            nested.add(safe_path(name[:-1]))
        else:
            names.add(safe_path(name))
    for boundary in nested:
        if any(name == boundary or name.startswith(boundary + "/") for name in names):
            fail("nested repository boundary overlaps an approved or tracked path")
        if any(other != boundary and (other.startswith(boundary + "/") or boundary.startswith(other + "/"))
               for other in nested):
            fail("nested repository boundaries overlap")
    return {"schema_version": 1, "worktree": str(repository), "index_entries": entries,
            "working": {name: working_entry(repository, name, nested_repository=name in nested)
                        for name in sorted(names | nested)}}


def read_reference(ref: dict[str, str], repository: Path) -> Any:
    validate_reference(ref, repository)
    data = Path(ref["path"]).read_bytes()
    if sha256(data) != ref["sha256"]:
        fail("evidence reference digest does not match")
    return json.loads(data)


def fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def write_once(path: Path, data: bytes) -> dict[str, str]:
    with path.open("xb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    fsync_directory(path.parent)
    return {"path": str(path), "sha256": sha256(data)}


def checkpoint_authority(request: dict[str, Any]) -> dict[str, Any]:
    repository = Path(request["repository"])
    accepted = read_reference(request["preservation"]["accepted_receipt"], repository)
    path = Path(__file__).parents[2] / "slice-checkpoint/scripts/checkpoint.py"
    spec = importlib.util.spec_from_file_location("completion_checkpoint_owner", path)
    if not spec or not spec.loader:
        fail("cannot load checkpoint authority owner")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    owner.validate_lifecycle_receipt(accepted, "accepted")
    for key in ("repository", "run_id", "slice_number"):
        if accepted[key] != request[key]:
            fail("accepted lifecycle identity does not match the request")
    for key in ("checkpoint_commit_oid", "checkpoint_tree_oid", "task_patch_digest"):
        if accepted[key] != request["proposal"][key]:
            fail("accepted lifecycle subject does not match the proposal")
    actions = [{"path": x["path"], "action": x["action"]} for x in accepted["paths"]]
    if sorted(actions, key=lambda x: x["path"]) != sorted(request["accepted_paths"], key=lambda x: x["path"]):
        fail("accepted lifecycle paths differ from approved paths")
    return accepted


def inspect_configuration(repository: Path) -> str:
    if any(name.startswith("GIT_") for name in os.environ if name not in {"GIT_CONFIG_NOSYSTEM", "GIT_CONFIG_GLOBAL", "GIT_TERMINAL_PROMPT", "GIT_OPTIONAL_LOCKS"}):
        fail("Git environment overrides require explicit support")
    for name in ("commit.gpgsign", "core.sparseCheckout", "core.splitIndex"):
        result = git(repository, ["config", "--bool", "--get", name])
        if result.returncode not in {0, 1} or result.stdout.strip() == "true":
            fail(f"unsupported signing or index configuration: {name}")
    if text_git(repository, ["rev-parse", "--shared-index-path"]):
        fail("split index is unsupported")
    hooks = Path(text_git(repository, ["rev-parse", "--path-format=absolute", "--git-path", "hooks"]))
    for name in ("pre-commit", "prepare-commit-msg", "commit-msg", "post-commit"):
        if (hooks / name).is_file() and os.access(hooks / name, os.X_OK):
            fail("active commit hooks require explicit repository-specific support")
    return sha256(git_bytes(repository, ["config", "--null", "--list", "--show-origin"]))


def verify_approved_working(request: dict[str, Any], repository: Path) -> None:
    for entry in request["preservation"]["approved_delta"]:
        name, after = entry["path"], entry["after"]
        working = working_entry(repository, name)
        if after is None:
            if working != {"kind": "missing"}:
                fail("approved deletion is not absent from working tree")
            continue
        if (working["kind"] != "file" or bool(working["mode"] & 0o111) != (after["mode"] == "100755")):
            fail("approved file kind/mode does not match accepted tree")
        attrs = git_bytes(repository, ["check-attr", "-z", "filter", "text", "eol", "working-tree-encoding", "ident", "--", name]).split(b"\0")
        if any(attrs[i] not in {b"unspecified", b"unset"} for i in range(2, len(attrs) - 1, 3)):
            fail("approved-path filters/encoding normalization are unsupported")
        autocrlf = git(repository, ["config", "--get", "core.autocrlf"])
        if autocrlf.returncode == 0 and autocrlf.stdout.strip() not in {"false", ""}:
            fail("approved-path autocrlf normalization is unsupported")
        if sha256(git_bytes(repository, ["cat-file", "blob", after["oid"]])) != working["sha256"]:
            fail("approved working bytes do not match accepted tree")


def expected_index(before: dict[str, Any], delta: list[dict[str, Any]]) -> dict[str, Any]:
    after = dict(before)
    for entry in delta:
        name = entry["path"]
        observed = before.get(name)
        allowed = [None if e is None else {**e, "tag": "H"} for e in (entry["before"], entry["after"])]
        if observed not in allowed:
            fail("approved path has conflicting partial staged state")
        if entry["after"] is None:
            after.pop(name, None)
        else:
            after[name] = {**entry["after"], "tag": "H"}
    return after


def validate_effect_shape(value: Any) -> None:
    if not isinstance(value, dict) or set(value) != {"prepared", "post", "publication", "index_transition", "preservation"}:
        fail("preservation effect fields must be exact")
    if value["publication"] != "verified" or any(value[k] not in {"verified", "failed", "unestablished"} for k in ("index_transition", "preservation")):
        fail("invalid preservation outcome")
    for key in ("prepared", "post"):
        ref = value[key]
        if ref is None and key == "post":
            continue
        if not isinstance(ref, dict) or set(ref) != {"path", "sha256"}:
            fail("preservation phase reference must contain path and sha256")
        digest(ref["sha256"])
        if not isinstance(ref["path"], str):
            fail("preservation phase path must be a string")
    if value["post"] is None and any(value[k] == "verified" for k in ("index_transition", "preservation")):
        fail("verified postconditions require durable post evidence")


def load_prepared(request: dict[str, Any], ref: dict[str, str]) -> dict[str, Any]:
    repository = Path(request["repository"])
    attempt = Path(request["preservation"]["attempt_directory"])
    if ref["path"] != str(attempt / "prepared.json"):
        fail("prepared reference is not this request's attempt")
    prepared = read_reference(ref, repository)
    if set(prepared) != {"schema_version", "request_binding", "commit_oid", "old_index", "new_index", "before", "after_index_entries", "configuration_sha256"} or prepared["schema_version"] != 1:
        fail("prepared phase has invalid fields")
    if prepared["request_binding"] != request_binding(request):
        fail("prepared attempt is bound to another complete request")
    validate_oid(prepared["commit_oid"], "prepared commit")
    digest(prepared["configuration_sha256"])
    for key, filename in (("old_index", "original.index"), ("new_index", "proposed.index")):
        index_ref = prepared[key]
        validate_reference(index_ref, repository)
        if index_ref["path"] != str(attempt / filename) or sha256(Path(index_ref["path"]).read_bytes()) != index_ref["sha256"]:
            fail("prepared index bytes are missing or changed")
    if prepared["old_index"]["sha256"] != request["preservation"]["expected_index_sha256"]:
        fail("prepared original index does not match approval")
    before = read_reference(request["preservation"]["manifest"], repository)
    if prepared["before"] != before:
        fail("prepared preservation manifest does not match approval")
    expected = expected_index(before["index_entries"], request["preservation"]["approved_delta"])
    if prepared["after_index_entries"] != expected:
        fail("prepared index would alter unapproved entries")
    for key, entries in (("old_index", before["index_entries"]), ("new_index", expected)):
        env = dict(os.environ, GIT_INDEX_FILE=prepared[key]["path"])
        if index_entries(repository, env) != entries:
            fail("prepared raw index does not match its entry projection")
    return prepared


def validate_effect(value: dict[str, Any]) -> None:
    request = value["request_binding"]["request"]
    repository = Path(request["repository"])
    checkpoint_authority(request)
    if full_delta(repository, request["expected_head_oid"], request["proposal"]["checkpoint_tree_oid"]) != sorted(request["preservation"]["approved_delta"], key=lambda x: x["path"]):
        fail("complete commit delta does not match approved full delta")
    evidence = value["evidence"]
    prepared = load_prepared(request, evidence["prepared"])
    if prepared["commit_oid"] != value["commit_oid"]:
        fail("receipt commit does not match immutable prepared commit")
    if evidence["post"] is None:
        return
    if evidence["post"]["path"] != str(Path(request["preservation"]["attempt_directory"]) / "post.json"):
        fail("post reference is not this request's attempt")
    post = read_reference(evidence["post"], repository)
    if set(post) != {"schema_version", "prepared", "observed_index_sha256", "observed", "index_transition", "preservation", "reason"} or post["schema_version"] != 1 or post["prepared"] != evidence["prepared"]:
        fail("post phase binding is invalid")
    for key in ("index_transition", "preservation"):
        if post[key] != evidence[key]:
            fail("postcondition projection does not match durable evidence")
    if post["index_transition"] == "verified" and post["observed_index_sha256"] != prepared["new_index"]["sha256"]:
        fail("verified index transition lacks matching bytes")
    if post["preservation"] == "verified":
        observed = post["observed"]
        if not isinstance(observed, dict) or observed != {
            **prepared["before"], "index_entries": prepared["after_index_entries"]
        }:
            fail("verified preservation lacks exact before/after evidence")


def phase_boundary(_phase: str) -> None:
    """In-process fault injection seam; never a runtime recovery instruction."""


def create_preserved(request: dict[str, Any], *, after_publish=None) -> dict[str, Any]:
    repository = Path(request["repository"])
    p = request["preservation"]
    attempt = Path(p["attempt_directory"])
    descriptor = None
    owned_lock = None
    branch_updated = False
    publication_attempted = False
    prepared_ref = None
    commit_oid = None
    try:
        if Path(text_git(repository, ["rev-parse", "--show-toplevel"])).resolve() != repository:
            fail("repository does not identify the Git worktree root")
        common = Path(text_git(repository, ["rev-parse", "--path-format=absolute", "--git-common-dir"])).resolve()
        git_dir = Path(text_git(repository, ["rev-parse", "--absolute-git-dir"])).resolve()
        for path in (attempt, Path(p["manifest"]["path"]), Path(p["accepted_receipt"]["path"])):
            if any(path == root or root in path.parents for root in (common, git_dir)):
                fail("external evidence must not alias Git metadata")
        def attachment():
            if text_git(repository, ["symbolic-ref", "--quiet", "--short", "HEAD"]) != request["expected_branch"]:
                fail("approved branch attachment changed")
        attachment()
        if text_git(repository, ["rev-parse", "HEAD"]) != request["expected_head_oid"]:
            fail("current HEAD does not match approved parent")
        config = inspect_configuration(repository)
        checkpoint_authority(request)
        delta = full_delta(repository, request["expected_head_oid"], request["proposal"]["checkpoint_tree_oid"])
        if delta != sorted(p["approved_delta"], key=lambda x: x["path"]):
            fail("complete commit delta does not match approved full delta")
        real_index = index_path(repository)
        lock = Path(str(real_index) + ".lock")
        descriptor = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        owned_lock = (lock, os.fstat(descriptor).st_ino)
        original = real_index.read_bytes()
        if sha256(original) != p["expected_index_sha256"]:
            fail("real index changed since approval")
        before = preservation_snapshot(repository, request["proposal"]["paths"])
        if before != read_reference(p["manifest"], repository):
            fail("preservation source/index manifest changed since approval")
        after_entries = expected_index(before["index_entries"], delta)
        verify_approved_working(request, repository)
        attempt.mkdir(mode=0o700)
        fsync_directory(attempt.parent)
        old_ref = write_once(attempt / "original.index", original)
        # Only this private copy is changed; never git-add the user's worktree.
        temporary = attempt / "building.index"
        write_once(temporary, original)
        env = dict(os.environ, GIT_INDEX_FILE=str(temporary))
        updates = b"".join((f"{x['after']['mode']} {x['after']['oid']}\t{x['path']}\0" if x["after"] else f"0 {'0' * len(request['expected_head_oid'])}\t{x['path']}\0").encode() for x in delta)
        git_bytes(repository, ["update-index", "-z", "--index-info"], env=env, input_bytes=updates)
        if index_entries(repository, env) != after_entries:
            fail("replacement index changed unapproved entries")
        proposed = temporary.read_bytes()
        new_ref = write_once(attempt / "proposed.index", proposed)
        message = request["proposal"]["subject"] + ("\n\n" + request["proposal"]["body"] if request["proposal"]["body"] else "")
        commit_oid = text_git(repository, ["commit-tree", request["proposal"]["checkpoint_tree_oid"], "-p", request["expected_head_oid"]], input_text=message + "\n")
        # Verify the object before it can become branch history.
        if (text_git(repository, ["rev-parse", f"{commit_oid}^"]) != request["expected_head_oid"]
                or text_git(repository, ["rev-parse", f"{commit_oid}^{{tree}}"]) != request["proposal"]["checkpoint_tree_oid"]
                or text_git(repository, ["show", "-s", "--format=%B", commit_oid]) != message):
            fail("prepared commit identity differs from approval")
        phase_boundary("before_prepared")
        prepared_ref = write_once(attempt / "prepared.json", request_canonical({
            "schema_version": 1, "request_binding": request_binding(request), "commit_oid": commit_oid,
            "old_index": old_ref, "new_index": new_ref, "before": before,
            "after_index_entries": after_entries, "configuration_sha256": config,
        }))
        phase_boundary("prepared")
        attachment()
        if (inspect_configuration(repository) != config or real_index.read_bytes() != original
                or preservation_snapshot(repository, request["proposal"]["paths"]) != before):
            fail("source/index/configuration changed before publication")
        checkpoint_authority(request)
        verify_approved_working(request, repository)
        if text_git(repository, ["rev-parse", f"refs/heads/{request['expected_branch']}"]) != request["expected_head_oid"]:
            fail("approved branch moved before atomic publication")
        publication_attempted = True
        update = git(repository, ["update-ref", f"refs/heads/{request['expected_branch']}", commit_oid, request["expected_head_oid"]])
        if update.returncode:
            fail("update-ref completion was unsuccessful")
        branch_updated = True
        phase_boundary("published")
        if after_publish is not None:
            after_publish(commit_oid)
        attachment()
        if (real_index.read_bytes() != original or inspect_configuration(repository) != config
                or preservation_snapshot(repository, request["proposal"]["paths"]) != before):
            fail("source/index/configuration changed after publication")
        if lock.lstat().st_ino != owned_lock[1]:
            fail("owned index lock was replaced")
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = None
            stream.write(proposed)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(lock, stat.S_IMODE(real_index.stat().st_mode))
        os.replace(lock, real_index)
        owned_lock = None
        fsync_directory(real_index.parent)
        phase_boundary("index_installed")
        attachment()
        observed = preservation_snapshot(repository, request["proposal"]["paths"])
        index_digest = sha256(real_index.read_bytes())
        index_ok = index_digest == new_ref["sha256"]
        preserved = observed == {**before, "index_entries": after_entries}
        if not index_ok or not preserved or text_git(repository, ["rev-parse", "HEAD"]) != commit_oid:
            fail("post-publication index/source/branch verification failed")
        post_ref = write_once(attempt / "post.json", request_canonical({
            "schema_version": 1, "prepared": prepared_ref, "observed_index_sha256": index_digest,
            "observed": observed, "index_transition": "verified", "preservation": "verified", "reason": None,
        }))
        result = receipt(request, "created", reason=None, commit_oid=commit_oid)
        result["evidence"] = {"prepared": prepared_ref, "post": post_ref, "publication": "verified",
                              "index_transition": "verified", "preservation": "verified"}
        write_once(attempt / "result.json", request_canonical(result))
        phase_boundary("receipt_written")
        return validate_receipt(result)
    except (ValueError, OSError) as error:
        if publication_attempted and not branch_updated:
            # Command completion is not the ref transaction's outcome. Reuse
            # read-only recovery; never install an index or replay create here.
            try:
                recovered = reconcile_preserved(request)
            except (ValueError, OSError) as recovery_error:
                fail(f"publication ambiguous after unsuccessful update-ref: {error}; {recovery_error}")
            if recovered is None:
                fail(f"publication ambiguous after unsuccessful update-ref: {error}; retain prepared attempt and lock")
            recovered["reason"] = f"postcondition_unestablished: update-ref completion: {error}"
            return recovered
        if not branch_updated:
            return receipt(request, "refused", reason=str(error))
        result = receipt(request, "created", reason=f"postcondition_unestablished: {error}", commit_oid=commit_oid)
        result["evidence"] = {"prepared": prepared_ref, "post": None, "publication": "verified",
                              "index_transition": "unestablished", "preservation": "unestablished"}
        return result
    finally:
        if descriptor is not None:
            os.close(descriptor)
        # An attempted CAS may have published even when completion is unknown.
        if owned_lock is not None and not branch_updated and not publication_attempted:
            path, inode = owned_lock
            if path.exists() and path.lstat().st_ino == inode:
                path.unlink()


def reconcile_preserved(request: dict[str, Any]) -> dict[str, Any] | None:
    repository = Path(request["repository"])
    attempt = Path(request["preservation"]["attempt_directory"])
    path = attempt / "prepared.json"
    if not path.exists():
        if text_git(repository, ["rev-parse", "HEAD"]) != request["expected_head_oid"]:
            fail("publication ambiguous: no immutable prepared attempt")
        return None
    ref = {"path": str(path), "sha256": sha256(path.read_bytes())}
    prepared = load_prepared(request, ref)
    commit_oid = prepared["commit_oid"]
    tip = text_git(repository, ["rev-parse", f"refs/heads/{request['expected_branch']}"])
    if tip == request["expected_head_oid"]:
        return None
    if git(repository, ["merge-base", "--is-ancestor", commit_oid, tip]).returncode:
        fail("publication ambiguous: branch does not contain exact prepared commit")
    result_path = attempt / "result.json"
    if result_path.exists():
        result = json.loads(result_path.read_bytes())
        if result.get("request_binding") != request_binding(request):
            fail("stored result is bound to another request")
        if result.get("state") != "created" or result.get("commit_oid") != commit_oid:
            fail("stored result contradicts observed prepared publication")
        validate_receipt(result)
        if result["evidence"]["prepared"] != ref:
            fail("stored result does not bind the observed prepared evidence")
        return result
    result = receipt(request, "created", reason="postconditions unestablished after interruption", commit_oid=commit_oid)
    result["evidence"] = {"prepared": ref, "post": None, "publication": "verified",
                          "index_transition": "unestablished", "preservation": "unestablished"}
    return validate_receipt(result)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request-json", required=True)
    parser.add_argument("--decision", required=True, choices=("pending", "decline", "create"))
    args = parser.parse_args()
    try:
        result = decide(json.loads(args.request_json), args.decision)
    except (json.JSONDecodeError, OSError, ValueError) as error:
        print(f"completion_commit: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
