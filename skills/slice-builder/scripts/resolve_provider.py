#!/usr/bin/env python3
"""Resolve a reconnaissance provider to its concrete evidence adapter."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any


DEFAULT_PROVIDER = "claude-codebase-memory"
DEFAULT_EVIDENCE_SKILL = "claude-recon-implementation"
PROVIDER_ADAPTERS = {
    DEFAULT_PROVIDER: DEFAULT_EVIDENCE_SKILL,
    "claude-filesystem": DEFAULT_EVIDENCE_SKILL,
}
PLANNED_PROVIDERS = {}


class ProviderResolutionError(ValueError):
    """Raised when provider configuration cannot be resolved safely."""


def resolve_provider(context: dict[str, Any]) -> dict[str, str]:
    if not isinstance(context, dict):
        raise ProviderResolutionError("builder.context must be an object")
    unknown_context = set(context) - {"evidence_skill", "reconnaissance"}
    if unknown_context:
        names = ", ".join(sorted(unknown_context))
        raise ProviderResolutionError(f"unknown builder.context fields: {names}")

    reconnaissance = context.get("reconnaissance", {})
    if not isinstance(reconnaissance, dict):
        raise ProviderResolutionError("builder.context.reconnaissance must be an object")
    unknown_reconnaissance = set(reconnaissance) - {"provider"}
    if unknown_reconnaissance:
        names = ", ".join(sorted(unknown_reconnaissance))
        raise ProviderResolutionError(f"unknown reconnaissance fields: {names}")

    provider = reconnaissance.get("provider", DEFAULT_PROVIDER)
    if not isinstance(provider, str) or not provider.strip():
        raise ProviderResolutionError("reconnaissance.provider must be a nonempty string")
    if provider == "auto":
        raise ProviderResolutionError("reconnaissance provider 'auto' is deferred")
    if provider in PLANNED_PROVIDERS:
        raise ProviderResolutionError(
            f"reconnaissance provider '{provider}' is recognized but unavailable"
        )
    if provider not in PROVIDER_ADAPTERS:
        raise ProviderResolutionError(f"unknown reconnaissance provider '{provider}'")

    configured_skill = context.get("evidence_skill", DEFAULT_EVIDENCE_SKILL)
    if not isinstance(configured_skill, str) or not configured_skill.strip():
        raise ProviderResolutionError("builder.context.evidence_skill must be a nonempty string")
    expected_skill = PROVIDER_ADAPTERS[provider]
    if configured_skill != expected_skill:
        raise ProviderResolutionError(
            f"provider '{provider}' requires evidence_skill '{expected_skill}', "
            f"not '{configured_skill}'"
        )
    return {"provider": provider, "evidence_skill": configured_skill}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--context-json", required=True)
    args = parser.parse_args()
    try:
        context = json.loads(args.context_json)
        result = resolve_provider(context)
    except (json.JSONDecodeError, ProviderResolutionError) as error:
        print(str(error), file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
