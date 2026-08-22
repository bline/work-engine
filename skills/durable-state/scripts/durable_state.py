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


class DurableStateStore(Protocol):
    def read(self, key: str) -> DurableValue | None: ...
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
        raw = self._git(["cat-file", "blob", revision])
        try:
            envelope = json.loads(raw)
            payload = base64.b64decode(envelope["payload_base64"], validate=True)
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise DurableStateError("stored durable value is malformed") from error
        digest = hashlib.sha256(payload).hexdigest()
        expected = {"schema_version": 1, "key": key,
                    "payload_base64": base64.b64encode(payload).decode(),
                    "payload_sha256": digest}
        if envelope != expected:
            raise DurableStateError("stored durable value failed key or integrity validation")
        return DurableValue(key, revision, payload, digest, self.adapter, ref)

    def publish(self, key: str, payload: bytes,
                expected_revision: str | None) -> DurableValue:
        if not isinstance(payload, bytes):
            raise DurableStateError("payload must be bytes")
        ref = self._ref(key)
        if self._current(ref) != expected_revision:
            raise DurableStateError("durable state revision conflict")
        digest = hashlib.sha256(payload).hexdigest()
        envelope = canonical({"schema_version": 1, "key": key,
                              "payload_base64": base64.b64encode(payload).decode(),
                              "payload_sha256": digest})
        revision = self._git(["hash-object", "-w", "--stdin"], envelope).decode().strip()
        self._git(["update-ref", ref, revision, expected_revision or "0" * len(revision)])
        return DurableValue(key, revision, payload, digest, self.adapter, ref)
