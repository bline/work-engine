#!/usr/bin/env python3
"""Read one locally admitted historical S5 review case without writer authority."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import independent_review_state as owner

PROFILE = "historical-s5-local-inspection-v1"
PROFILE_V2 = "historical-s5-local-inspection-v2"
RUN_ID = "skills-migration-app-server-20260828"
ALLOWED_FIELDS = {"checkpoint", "gate", "review_summary", "source_limit", "metric_discrepancy"}
ALLOWED_FIELDS_V2 = ALLOWED_FIELDS | {"provider_attempts", "scheduler_item", "metrics_comparison"}


def fail(message: str) -> None:
    raise ValueError(message)


def read_grant(path: Path) -> dict[str, Any]:
    if not path.is_absolute():
        fail("inspection grant path must be absolute")
    parent = path.parent
    parent_fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        directory = os.fstat(parent_fd)
        if directory.st_uid != os.geteuid() or stat.S_IMODE(directory.st_mode) != 0o700:
            fail("inspection grant directory owner or mode is invalid")
        fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent_fd)
        try:
            info = os.fstat(fd)
            if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                    or stat.S_IMODE(info.st_mode) != 0o600 or info.st_size > 64 * 1024):
                fail("inspection grant file type, owner, mode or size is invalid")
            with os.fdopen(fd, "rb", closefd=False) as stream:
                raw = stream.read(64 * 1024 + 1)
                if len(raw) > 64 * 1024:
                    fail("inspection grant grew beyond read limit")
                value = json.loads(raw)
        finally:
            os.close(fd)
    finally:
        os.close(parent_fd)
    required = {"schema_version", "profile", "grant_id", "repository", "run_id", "slice_number",
                "issued_at", "expires_at", "issuer_provenance", "peer_uid", "audience", "fields",
                "checkpoints", "gates", "reviews"}
    if not isinstance(value, dict):
        fail("inspection grant schema is invalid")
    version = value.get("schema_version")
    if version == 2:
        required = required | {"provider", "scheduler", "metrics"}
    if (set(value) != required or (version, value.get("profile")) not in
            {(1, PROFILE), (2, PROFILE_V2)}):
        fail("inspection grant schema is invalid")
    if (value["run_id"] != RUN_ID or value["slice_number"] != 6
            or value["audience"] != "local_terminal_inspection"
            or not isinstance(value["grant_id"], str) or not value["grant_id"]
            or not isinstance(value["issuer_provenance"], str) or not value["issuer_provenance"]
            or value["peer_uid"] != os.geteuid()
            or value["fields"] != sorted(ALLOWED_FIELDS_V2 if version == 2 else ALLOWED_FIELDS)):
        fail("inspection grant case, audience or local identity is invalid")
    if version == 2 and (not isinstance(value["provider"], dict)
                         or not isinstance(value["scheduler"], dict)
                         or not isinstance(value["metrics"], dict)):
        fail("inspection grant source descriptors are invalid")
    issued = datetime.fromisoformat(value["issued_at"].replace("Z", "+00:00"))
    expires = datetime.fromisoformat(value["expires_at"].replace("Z", "+00:00"))
    if (issued.tzinfo is None or expires.tzinfo is None or issued > datetime.now(timezone.utc)
            or expires <= datetime.now(timezone.utc) or expires <= issued):
        fail("inspection grant time window is invalid")
    repository = Path(value["repository"])
    if not repository.is_absolute() or repository.resolve() != repository:
        fail("inspection repository is invalid")
    if (not isinstance(value["checkpoints"], list) or len(value["checkpoints"]) != 6
            or not isinstance(value["gates"], list) or len(value["gates"]) != 5
            or not isinstance(value["reviews"], list) or len(value["reviews"]) != 3):
        fail("inspection source inventory is incomplete")
    return value


def inspect(grant: dict[str, Any]) -> dict[str, Any]:
    # This adapter is the historical read boundary. The durable owner uses Git
    # subprocesses; disable replacement refs for every one of those reads.
    previous = os.environ.get("GIT_NO_REPLACE_OBJECTS")
    os.environ["GIT_NO_REPLACE_OBJECTS"] = "1"
    try:
        return _inspect_raw(grant)
    finally:
        if previous is None:
            os.environ.pop("GIT_NO_REPLACE_OBJECTS", None)
        else:
            os.environ["GIT_NO_REPLACE_OBJECTS"] = previous


def _inspect_raw(grant: dict[str, Any]) -> dict[str, Any]:
    repository = Path(grant["repository"])
    store = owner.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
    checkpoints = {item["ref"]: item["oid"] for item in grant["checkpoints"]}
    if len(checkpoints) != 6:
        fail("checkpoint descriptor refs are duplicated")
    result: list[dict[str, Any]] = []
    for entry in grant["reviews"]:
        key, head = entry["key"], entry["head"]
        if not isinstance(key, str) or not key.startswith("independent-review-episode-v1/"):
            fail("inspection review key is invalid")
        current = store.read(key)
        if current is None or current.revision != head:
            fail("review head changed since grant issuance")
        page = store.list_revisions(key, None, 32)
        if page.next_cursor is not None or not page.values or page.values[0].revision != head:
            fail("review history is incomplete or changed")
        revisions = []
        prior = None
        for index, stored in enumerate(page.values):
            if store._git(["cat-file", "-t", stored.revision]).strip() != b"blob":
                fail("review revision is not a blob")
            raw = store._git(["cat-file", "blob", stored.revision])
            object_bytes = b"blob " + str(len(raw)).encode() + b"\0" + raw
            if hashlib.sha1(object_bytes).hexdigest() != stored.revision:
                fail("review revision bytes do not match object id")
            value = owner.decode(stored)
            identity = value["identity"]
            if (owner.stable_key(identity) != key or identity["run_id"] != RUN_ID
                    or identity["slice_number"] != 6):
                fail("review identity is outside inspection case")
            if index and prior != stored.revision:
                fail("review predecessor chain is broken")
            for subject in value["reviewed_subject"]:
                if subject["owner"] == "slice-checkpoint" and checkpoints.get(subject["reference"]) != subject["revision"]:
                    fail("review subject does not match admitted checkpoint")
            revisions.append({
                "revision": stored.revision, "predecessor_revision": stored.predecessor_revision,
                "payload_sha256": stored.integrity_sha256, "status": value["status"],
                "phase": value["current_review_phase"], "subject": [
                    {"owner": subject["owner"], "reference": subject["reference"],
                     "revision": subject["revision"]} for subject in value["reviewed_subject"]],
                "finding_states": [{"id": finding["finding_id"], "status": finding["status"],
                                    "severity": finding["severity"]} for finding in value["findings"]],
                "authority_source": value["authority_binding"]["source"],
                "authority_manifest_sha256": value["authority_binding"]["manifest_sha256"],
                "historical_grant_id": value["authority_binding"]["grant_id"],
                "writer_generation": value["writer_binding"]["generation"],
            })
            prior = stored.predecessor_revision
        if prior is not None:
            fail("review predecessor chain did not reach its beginning")
        result.append({"key": key, "head": head, "revisions": revisions,
                       "source_owner": "independent-review-state", "integrity": "envelope_history_and_state_validated"})
    links = []
    marker = "replacement-authority:reported_episode_cannot_bind_later_candidate:"
    for review in result:
        for revision in review["revisions"][:1]:
            reference = revision["authority_source"]["reference"]
            if not reference.startswith(marker):
                continue
            subject = reference.removeprefix(marker)
            matches = [(other["key"], target) for other in result
                       for target in other["revisions"]
                       if subject == f"{store._ref(other['key'])}@{target['revision']}"
                       and target["phase"] == "reported"]
            if len(matches) != 1:
                fail("replacement authority does not bind one admitted reported revision")
            original_key, target = matches[0]
            links.append({"replacement_key": review["key"], "replacement_revision": revision["revision"],
                          "original_key": original_key, "original_reported_revision": target["revision"],
                          "authority_reference": reference})
    return {"source_owner": "independent-review-state", "grant_id": grant["grant_id"],
            "reviews": result, "replacement_links": links,
            "source_limit": "reported is a review phase, not retirement or candidate acceptance"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inspection-grant", required=True, type=Path)
    args = parser.parse_args()
    try:
        result = inspect(read_grant(args.inspection_grant))
    except (OSError, RuntimeError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"historical_inspect: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
