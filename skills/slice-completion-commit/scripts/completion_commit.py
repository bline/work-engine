#!/usr/bin/env python3
"""Create an explicitly approved, user-visible commit for an accepted slice."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable


STATES = {"pending", "declined", "created", "refused"}
RECEIPT_FIELDS = {
    "schema_version", "state", "repository", "run_id", "slice_number",
    "proposal", "proposal_digest", "expected_branch", "expected_head_oid",
    "commit_oid", "reason",
}


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
    return {
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


def validate_receipt(value: Any) -> dict[str, Any]:
    """Re-establish the evidence claimed by a completion-commit receipt."""
    if not isinstance(value, dict) or set(value) != RECEIPT_FIELDS:
        fail("completion-commit receipt fields must exactly match the contract")
    if value["schema_version"] != 1 or value["state"] not in STATES:
        fail("completion-commit receipt must use schema version 1 and a valid state")
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
    return value


def validate_request(raw: Any) -> dict[str, Any]:
    required = {"repository", "run_id", "slice_number", "expected_branch",
                "expected_head_oid", "accepted_paths", "proposal"}
    if not isinstance(raw, dict) or set(raw) not in {frozenset(required), frozenset(required | {"operational_paths"})}:
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
    return raw


def reconcile_created(raw: Any) -> dict[str, Any] | None:
    """Recover an authoritative created result without replaying publication."""
    request = validate_request(raw)
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
