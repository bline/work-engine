#!/usr/bin/env python3
"""Resolve versioned repository-evidence and independent-review roles."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any


LEGACY_DEFAULT_PROVIDER = "claude-codebase-memory"
LEGACY_DEFAULT_SKILL = "claude-recon-implementation"
LEGACY_PROVIDER_ADAPTERS = {
    "claude-codebase-memory": LEGACY_DEFAULT_SKILL,
    "claude-filesystem": LEGACY_DEFAULT_SKILL,
}
LEGACY_PLANNED_PROVIDERS = {"codex-codebase-memory"}

DEFAULT_REPOSITORY_PROVIDER = "codex-codebase-memory"
DEFAULT_REPOSITORY_SKILL = "repo-search"
REPOSITORY_PROVIDER_ADAPTERS = {
    DEFAULT_REPOSITORY_PROVIDER: DEFAULT_REPOSITORY_SKILL,
    "claude-codebase-memory": LEGACY_DEFAULT_SKILL,
    "claude-filesystem": LEGACY_DEFAULT_SKILL,
}
DEFAULT_REVIEW_PROVIDER = "claude"
DEFAULT_REVIEW_SKILL = LEGACY_DEFAULT_SKILL
REVIEW_PROVIDER_ADAPTERS = {
    DEFAULT_REVIEW_PROVIDER: DEFAULT_REVIEW_SKILL,
}
ADVERSARIAL_REVIEW_PROVIDER = "codex"
ADVERSARIAL_REVIEW_SKILL = "codex-adversarial-review"
ADVERSARIAL_REVIEW_MODEL = "gpt-5.6-sol"
ADVERSARIAL_REVIEW_EVIDENCE_CLASS = "accepted_same_model_review"
ADVERSARIAL_REVIEW_ISOLATION = "fresh_process"


class ProviderResolutionError(ValueError):
    """Raised when provider configuration cannot be resolved safely."""


def _require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ProviderResolutionError(f"{path} must be an object")
    return value


def _reject_unknown(value: dict[str, Any], allowed: set[str], path: str) -> None:
    unknown = set(value) - allowed
    if unknown:
        raise ProviderResolutionError(
            f"unknown {path} fields: {', '.join(sorted(unknown))}"
        )


def _resolve_role(
    value: Any,
    *,
    path: str,
    default_provider: str,
    default_skill: str,
    adapters: dict[str, str],
) -> dict[str, str]:
    role = _require_object(value, path)
    _reject_unknown(role, {"provider", "skill"}, path)
    provider = role.get("provider", default_provider)
    skill = role.get("skill", default_skill)
    if not isinstance(provider, str) or not provider.strip():
        raise ProviderResolutionError(f"{path}.provider must be a nonempty string")
    if not isinstance(skill, str) or not skill.strip():
        raise ProviderResolutionError(f"{path}.skill must be a nonempty string")
    if provider == "auto":
        raise ProviderResolutionError(f"{path}.provider 'auto' is deferred")
    if provider not in adapters:
        raise ProviderResolutionError(f"unknown {path}.provider '{provider}'")
    expected_skill = adapters[provider]
    if skill != expected_skill:
        raise ProviderResolutionError(
            f"{path}.provider '{provider}' requires skill '{expected_skill}', not '{skill}'"
        )
    return {"provider": provider, "skill": skill}


def _resolve_v1(context: dict[str, Any]) -> dict[str, Any]:
    _reject_unknown(context, {"evidence_skill", "reconnaissance"}, "builder.context")
    reconnaissance = _require_object(
        context.get("reconnaissance", {}), "builder.context.reconnaissance"
    )
    _reject_unknown(reconnaissance, {"provider"}, "reconnaissance")
    provider = reconnaissance.get("provider", LEGACY_DEFAULT_PROVIDER)
    skill = context.get("evidence_skill", LEGACY_DEFAULT_SKILL)
    if not isinstance(provider, str) or not provider.strip():
        raise ProviderResolutionError("reconnaissance.provider must be a nonempty string")
    if provider == "auto":
        raise ProviderResolutionError("reconnaissance provider 'auto' is deferred")
    if provider in LEGACY_PLANNED_PROVIDERS:
        raise ProviderResolutionError(
            f"reconnaissance provider '{provider}' is recognized but unavailable"
        )
    if provider not in LEGACY_PROVIDER_ADAPTERS:
        raise ProviderResolutionError(f"unknown reconnaissance provider '{provider}'")
    if not isinstance(skill, str) or not skill.strip():
        raise ProviderResolutionError("builder.context.evidence_skill must be a nonempty string")
    expected = LEGACY_PROVIDER_ADAPTERS[provider]
    if skill != expected:
        raise ProviderResolutionError(
            f"provider '{provider}' requires evidence_skill '{expected}', not '{skill}'"
        )
    combined = {"provider": provider, "skill": skill}
    return {
        "config_version": 1,
        "compatibility": "legacy-combined-evidence-and-review",
        "repository_evidence": combined,
        "independent_review": combined,
    }


def _resolve_v2(context: dict[str, Any]) -> dict[str, Any]:
    _reject_unknown(
        context,
        {"repository_evidence", "independent_review", "adversarial_review"},
        "builder.context",
    )
    repository = _resolve_role(
        context.get("repository_evidence", {}),
        path="builder.context.repository_evidence",
        default_provider=DEFAULT_REPOSITORY_PROVIDER,
        default_skill=DEFAULT_REPOSITORY_SKILL,
        adapters=REPOSITORY_PROVIDER_ADAPTERS,
    )
    if "independent_review" in context and "adversarial_review" in context:
        raise ProviderResolutionError(
            "builder.context must configure exactly one review role, not both "
            "independent_review and adversarial_review"
        )
    result = {
        "config_version": 2,
        "compatibility": "split-roles",
        "repository_evidence": repository,
    }
    if "adversarial_review" not in context:
        result["independent_review"] = _resolve_role(
            context.get("independent_review", {}),
            path="builder.context.independent_review",
            default_provider=DEFAULT_REVIEW_PROVIDER,
            default_skill=DEFAULT_REVIEW_SKILL,
            adapters=REVIEW_PROVIDER_ADAPTERS,
        )
        return result

    review = _require_object(
        context["adversarial_review"], "builder.context.adversarial_review"
    )
    fields = {
        "provider", "skill", "model", "reasoning_effort", "evidence_class", "isolation"
    }
    _reject_unknown(review, fields, "builder.context.adversarial_review")
    expected = {
        "provider": ADVERSARIAL_REVIEW_PROVIDER,
        "skill": ADVERSARIAL_REVIEW_SKILL,
        "model": ADVERSARIAL_REVIEW_MODEL,
        "evidence_class": ADVERSARIAL_REVIEW_EVIDENCE_CLASS,
        "isolation": ADVERSARIAL_REVIEW_ISOLATION,
    }
    for field, expected_value in expected.items():
        value = review.get(field)
        if value != expected_value:
            raise ProviderResolutionError(
                f"builder.context.adversarial_review.{field} must be "
                f"'{expected_value}'"
            )
    reasoning_effort = review.get("reasoning_effort")
    if not isinstance(reasoning_effort, str) or not reasoning_effort.strip():
        raise ProviderResolutionError(
            "builder.context.adversarial_review.reasoning_effort must be a nonempty string"
        )
    result["adversarial_review"] = {
        **expected,
        "reasoning_effort": reasoning_effort,
    }
    return result


def resolve_builder_context(version: int, context: dict[str, Any]) -> dict[str, Any]:
    context = _require_object(context, "builder.context")
    if isinstance(version, bool):
        raise ProviderResolutionError("config version must be 1 or 2")
    if version == 1:
        return _resolve_v1(context)
    if version == 2:
        return _resolve_v2(context)
    raise ProviderResolutionError("config version must be 1 or 2")


def resolve_provider(context: dict[str, Any]) -> dict[str, str]:
    """Preserve the historical v1 Python API for installed callers."""
    resolved = resolve_builder_context(1, context)
    repository = resolved["repository_evidence"]
    return {"provider": repository["provider"], "evidence_skill": repository["skill"]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config-version", type=int)
    parser.add_argument("--context-json", required=True)
    args = parser.parse_args()
    try:
        context = json.loads(args.context_json)
        # Omitting the flag preserves the historical flat CLI response. An
        # explicit version requests the normalized, provenance-bearing shape.
        result = (
            resolve_provider(context)
            if args.config_version is None
            else resolve_builder_context(args.config_version, context)
        )
    except (json.JSONDecodeError, ProviderResolutionError) as error:
        print(str(error), file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
