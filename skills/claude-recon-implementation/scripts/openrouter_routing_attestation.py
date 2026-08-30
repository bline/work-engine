#!/usr/bin/env python3
"""Observe an OpenRouter key guardrail and emit a fail-closed attestation."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import tempfile
from typing import Any, Callable
from urllib import error, parse, request


API_BASE_URL = "https://openrouter.ai/api/v1"
SCHEMA_VERSION = 1
PAGE_LIMIT = 100


class AttestationError(ValueError):
    """Raised when the observed route cannot satisfy the attestation contract."""


JsonGet = Callable[[str, dict[str, int] | None], dict[str, Any]]


def _api_get(path: str, query: dict[str, int] | None, *, management_key: str,
             timeout_seconds: float) -> dict[str, Any]:
    url = f"{API_BASE_URL}{path}"
    if query:
        url = f"{url}?{parse.urlencode(query)}"
    call = request.Request(
        url,
        headers={"Accept": "application/json",
                 "Authorization": f"Bearer {management_key}",
                 "User-Agent": "work-engine-openrouter-routing-attestation/1"},
        method="GET",
    )
    try:
        with request.urlopen(call, timeout=timeout_seconds) as response:
            payload = response.read()
    except error.HTTPError as failure:
        raise AttestationError(
            f"OpenRouter management API returned HTTP {failure.code}"
        ) from failure
    except (error.URLError, TimeoutError, OSError) as failure:
        raise AttestationError("OpenRouter management API request failed") from failure
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as failure:
        raise AttestationError("OpenRouter management API returned invalid JSON") from failure
    if not isinstance(value, dict):
        raise AttestationError("OpenRouter management API response must be an object")
    return value


def _read_guardrail(get_json: JsonGet, guardrail_id: str) -> dict[str, Any]:
    envelope = get_json(f"/guardrails/{parse.quote(guardrail_id, safe='')}", None)
    guardrail = envelope.get("data")
    if not isinstance(guardrail, dict):
        raise AttestationError("guardrail response is missing its data object")
    if guardrail.get("id") != guardrail_id:
        raise AttestationError("guardrail response does not match the requested ID")
    return guardrail


def _read_key_assignments(get_json: JsonGet) -> list[dict[str, Any]]:
    assignments: list[dict[str, Any]] = []
    offset = 0
    total_count: int | None = None
    while total_count is None or offset < total_count:
        envelope = get_json(
            "/guardrails/assignments/keys",
            {"offset": offset, "limit": PAGE_LIMIT},
        )
        page = envelope.get("data")
        reported_total = envelope.get("total_count")
        if (not isinstance(page, list) or isinstance(reported_total, bool)
                or not isinstance(reported_total, int) or reported_total < 0):
            raise AttestationError("key-assignment response has an invalid envelope")
        if total_count is None:
            total_count = reported_total
        elif reported_total != total_count:
            raise AttestationError("key-assignment total changed during pagination")
        if not page and offset < total_count:
            raise AttestationError("key-assignment pagination ended before total_count")
        for assignment in page:
            if not isinstance(assignment, dict):
                raise AttestationError("key-assignment entry must be an object")
            assignments.append(assignment)
        offset += len(page)
    if len(assignments) != total_count:
        raise AttestationError("key-assignment pagination did not match total_count")
    return assignments


def build_attestation(
    guardrail: dict[str, Any], assignments: list[dict[str, Any]], *,
    guardrail_id: str, key_hash: str, model: str,
    observed_at: datetime | None = None,
) -> dict[str, Any]:
    """Validate exact route constraints and produce the version-1 artifact."""
    if not guardrail_id or not key_hash or not model:
        raise AttestationError("guardrail ID, key hash, and model must be non-empty")
    if guardrail.get("id") != guardrail_id:
        raise AttestationError("guardrail does not match the requested ID")
    providers = guardrail.get("allowed_providers")
    models = guardrail.get("allowed_models")
    if providers != ["anthropic"]:
        raise AttestationError("guardrail must allow only provider 'anthropic'")
    if models != [model]:
        raise AttestationError("guardrail must allow only the exact requested model")
    matching = [entry for entry in assignments if entry.get("key_hash") == key_hash]
    if len(matching) != 1:
        raise AttestationError("expected exactly one assignment for the selected key hash")
    assignment_guardrail_id = matching[0].get("guardrail_id")
    if assignment_guardrail_id != guardrail_id:
        raise AttestationError("selected key is not assigned to the requested guardrail")
    timestamp = observed_at or datetime.now(timezone.utc)
    if timestamp.tzinfo is None:
        raise AttestationError("observed_at must include a timezone")
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": "openrouter-key-guardrail",
        "observed_at": timestamp.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
        "guardrail_id": guardrail_id,
        "key_hash": key_hash,
        "allowed_providers": providers,
        "allowed_models": models,
        "assignment_guardrail_id": assignment_guardrail_id,
    }


def observe_attestation(get_json: JsonGet, *, guardrail_id: str, key_hash: str,
                        model: str, observed_at: datetime | None = None) -> dict[str, Any]:
    return build_attestation(
        _read_guardrail(get_json, guardrail_id),
        _read_key_assignments(get_json),
        guardrail_id=guardrail_id, key_hash=key_hash, model=model,
        observed_at=observed_at,
    )


def _write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    with tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        handle.write(payload)
        temporary = Path(handle.name)
    temporary.replace(path)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--guardrail-id", required=True)
    parser.add_argument("--key-hash", default=os.environ.get("OPENROUTER_API_KEY_HASH"))
    parser.add_argument("--model", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--timeout-seconds", type=float, default=20.0)
    return parser


def main() -> int:
    args = _parser().parse_args()
    management_key = os.environ.get("OPENROUTER_MANAGEMENT_KEY", "")
    if not management_key:
        print("error: OPENROUTER_MANAGEMENT_KEY is required", file=os.sys.stderr)
        return 2
    if not args.key_hash:
        print("error: --key-hash or OPENROUTER_API_KEY_HASH is required", file=os.sys.stderr)
        return 2
    if args.timeout_seconds <= 0:
        print("error: --timeout-seconds must be positive", file=os.sys.stderr)
        return 2

    def get_json(path: str, query: dict[str, int] | None) -> dict[str, Any]:
        return _api_get(path, query, management_key=management_key,
                        timeout_seconds=args.timeout_seconds)

    try:
        attestation = observe_attestation(
            get_json, guardrail_id=args.guardrail_id,
            key_hash=args.key_hash, model=args.model,
        )
        _write_json(args.output, attestation)
    except (AttestationError, OSError) as failure:
        print(f"error: {failure}", file=os.sys.stderr)
        return 1
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
