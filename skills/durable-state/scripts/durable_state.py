#!/usr/bin/env python3
"""Semantically blind durable publication with opaque CAS revisions."""

from __future__ import annotations

import base64
import hashlib
import json
import subprocess
from pathlib import Path
from typing import NamedTuple, Protocol


class DurableStateError(ValueError):
    pass


class DurableValue(NamedTuple):
    key: str
    revision: str
    payload: bytes
    integrity_sha256: str
    adapter: str
    location: str
    predecessor_revision: str | None


class DurableHistoryPage(NamedTuple):
    values: tuple[DurableValue, ...]
    next_cursor: str | None


class DurableStateStore(Protocol):
    def read(self, key: str) -> DurableValue | None: ...
    def read_revision(self, key: str, revision: str) -> DurableValue: ...
    def list_revisions(self, key: str, cursor: str | None,
                       limit: int) -> DurableHistoryPage: ...
    def publish(self, key: str, payload: bytes, expected_revision: str | None) -> DurableValue: ...


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


class GitRefDurableStateStore:
    adapter = "git-ref-v1"

    def __init__(self, repository: Path, namespace: str = "live-state") -> None:
        self.repository = repository.resolve()
        if not namespace or any(part in {"", ".", ".."} for part in namespace.split("/")):
            raise DurableStateError("namespace must contain nonempty safe path components")
        self.namespace = namespace
        self._git(["rev-parse", "--git-dir"])
        self._git(["check-ref-format", f"refs/work-engine/{namespace}/{'0' * 64}"])
        self._git(["check-ref-format",
                   f"refs/work-engine/{namespace}-history/{'0' * 64}/{'0' * 40}"])

    def _git(self, args: list[str], input_bytes: bytes | None = None) -> bytes:
        try:
            result = subprocess.run(
                ["git", "-C", str(self.repository), *args], input=input_bytes,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
            )
        except OSError as error:
            raise DurableStateError(f"unable to execute git: {error}") from error
        if result.returncode:
            raise DurableStateError(result.stderr.decode(errors="replace").strip())
        return result.stdout

    def _ref(self, key: str) -> str:
        if not isinstance(key, str) or not key:
            raise DurableStateError("key must be a nonempty string")
        return f"refs/work-engine/{self.namespace}/{hashlib.sha256(key.encode()).hexdigest()}"

    def _history_ref(self, key: str, revision: str) -> str:
        key_digest = hashlib.sha256(key.encode()).hexdigest()
        return f"refs/work-engine/{self.namespace}-history/{key_digest}/{revision}"

    def _current(self, ref: str) -> str | None:
        try:
            result = subprocess.run(
                ["git", "-C", str(self.repository), "rev-parse", "--verify", "--quiet", ref],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
            )
        except OSError as error:
            raise DurableStateError(f"unable to execute git: {error}") from error
        if result.returncode == 0:
            return result.stdout.strip()
        if result.returncode == 1:
            return None
        raise DurableStateError(result.stderr.strip() or "unable to inspect durable state ref")

    def read(self, key: str) -> DurableValue | None:
        ref = self._ref(key)
        revision = self._current(ref)
        if revision is None:
            return None
        return self.read_revision(key, revision)

    def read_revision(self, key: str, revision: str) -> DurableValue:
        if not isinstance(revision, str) or not revision:
            raise DurableStateError("revision must be a nonempty string")
        try:
            raw = self._git(["cat-file", "blob", revision])
        except DurableStateError as error:
            raise DurableStateError("durable state revision is unavailable") from error
        try:
            envelope = json.loads(raw)
            payload = base64.b64decode(envelope["payload_base64"], validate=True)
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise DurableStateError("stored durable value is malformed") from error
        digest = hashlib.sha256(payload).hexdigest()
        schema_version = envelope.get("schema_version")
        predecessor = envelope.get("predecessor_revision") if schema_version == 2 else None
        if predecessor is not None and (not isinstance(predecessor, str) or not predecessor):
            raise DurableStateError("stored durable value has an invalid predecessor revision")
        expected = {"schema_version": schema_version, "key": key,
                    "payload_base64": base64.b64encode(payload).decode(),
                    "payload_sha256": digest}
        if schema_version == 2:
            expected["predecessor_revision"] = predecessor
        elif schema_version != 1:
            raise DurableStateError("stored durable value has an unsupported schema version")
        if envelope != expected:
            raise DurableStateError("stored durable value failed key or integrity validation")
        return DurableValue(key, revision, payload, digest, self.adapter,
                            self._ref(key), predecessor)

    def list_revisions(self, key: str, cursor: str | None,
                       limit: int) -> DurableHistoryPage:
        if isinstance(limit, bool) or not isinstance(limit, int) or not 1 <= limit <= 100:
            raise DurableStateError("history limit must be an integer from 1 through 100")
        current = self._current(self._ref(key))
        if current is None:
            if cursor is not None:
                raise DurableStateError("history cursor is not reachable from current state")
            return DurableHistoryPage((), None)

        revision = current
        seen: set[str] = set()
        if cursor is not None:
            if not isinstance(cursor, str) or not cursor:
                raise DurableStateError("history cursor must be a nonempty revision")
            while revision != cursor:
                if revision in seen:
                    raise DurableStateError("durable state history contains a cycle")
                seen.add(revision)
                value = self.read_revision(key, revision)
                revision = value.predecessor_revision
                if revision is None:
                    raise DurableStateError("history cursor is not reachable from current state")
            cursor_value = self.read_revision(key, revision)
            revision = cursor_value.predecessor_revision

        values: list[DurableValue] = []
        while revision is not None and len(values) < limit:
            if revision in seen:
                raise DurableStateError("durable state history contains a cycle")
            seen.add(revision)
            value = self.read_revision(key, revision)
            values.append(value)
            revision = value.predecessor_revision
        next_cursor = values[-1].revision if revision is not None and values else None
        return DurableHistoryPage(tuple(values), next_cursor)

    def publish(self, key: str, payload: bytes,
                expected_revision: str | None) -> DurableValue:
        if not isinstance(payload, bytes):
            raise DurableStateError("payload must be bytes")
        ref = self._ref(key)
        if self._current(ref) != expected_revision:
            raise DurableStateError("durable state revision conflict")
        digest = hashlib.sha256(payload).hexdigest()
        envelope = canonical({"schema_version": 2, "key": key,
                              "payload_base64": base64.b64encode(payload).decode(),
                              "payload_sha256": digest,
                              "predecessor_revision": expected_revision})
        revision = self._git(["hash-object", "-w", "--stdin"], envelope).decode().strip()
        commands = ["start", f"create {self._history_ref(key, revision)} {revision}"]
        if (expected_revision is not None
                and self._current(self._history_ref(key, expected_revision)) is None):
            # A schema-v1 head predates retention refs. Adopt it atomically with
            # the first linked publication instead of fabricating earlier ancestry.
            commands.append(
                f"create {self._history_ref(key, expected_revision)} {expected_revision}")
        commands.extend([
            f"update {ref} {revision} {expected_revision or '0' * len(revision)}",
            "prepare", "commit", "",
        ])
        try:
            self._git(["update-ref", "--stdin"], "\n".join(commands).encode())
        except DurableStateError as error:
            if self._current(ref) != expected_revision:
                raise DurableStateError("durable state revision conflict") from error
            raise
        return DurableValue(key, revision, payload, digest, self.adapter, ref,
                            expected_revision)
