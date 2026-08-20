from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "resolve_provider.py"
SPEC = importlib.util.spec_from_file_location("resolve_provider", SCRIPT)
assert SPEC and SPEC.loader
RESOLVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RESOLVER)


class ResolveProviderTest(unittest.TestCase):
    def test_v2_defaults_to_direct_repository_evidence_and_separate_review(self) -> None:
        self.assertEqual(
            RESOLVER.resolve_builder_context(2, {}),
            {
                "config_version": 2,
                "compatibility": "split-roles",
                "repository_evidence": {
                    "provider": "codex-codebase-memory",
                    "skill": "repo-search",
                },
                "independent_review": {
                    "provider": "claude",
                    "skill": "claude-recon-implementation",
                },
            },
        )

    def test_v2_retrieval_change_does_not_change_review(self) -> None:
        resolved = RESOLVER.resolve_builder_context(
            2,
            {
                "repository_evidence": {
                    "provider": "claude-filesystem",
                    "skill": "claude-recon-implementation",
                }
            },
        )
        self.assertEqual(resolved["repository_evidence"]["provider"], "claude-filesystem")
        self.assertEqual(resolved["independent_review"]["provider"], "claude")

    def test_v1_explicit_claude_context_keeps_combined_historical_meaning(self) -> None:
        resolved = RESOLVER.resolve_builder_context(
            1,
            {
                "evidence_skill": "claude-recon-implementation",
                "reconnaissance": {"provider": "claude-filesystem"},
            },
        )
        self.assertEqual(resolved["compatibility"], "legacy-combined-evidence-and-review")
        self.assertEqual(resolved["repository_evidence"], resolved["independent_review"])
        self.assertEqual(resolved["repository_evidence"]["provider"], "claude-filesystem")

    def test_v1_compatibility_api_is_unchanged(self) -> None:
        self.assertEqual(
            RESOLVER.resolve_provider({}),
            {
                "provider": "claude-codebase-memory",
                "evidence_skill": "claude-recon-implementation",
            },
        )

    def test_v1_cli_output_is_unchanged(self) -> None:
        output = subprocess.check_output(
            [sys.executable, str(SCRIPT), "--context-json", "{}"], text=True
        )
        self.assertEqual(
            json.loads(output),
            {
                "provider": "claude-codebase-memory",
                "evidence_skill": "claude-recon-implementation",
            },
        )

    def test_explicit_v1_cli_returns_normalized_compatibility_shape(self) -> None:
        output = subprocess.check_output(
            [
                sys.executable,
                str(SCRIPT),
                "--config-version",
                "1",
                "--context-json",
                "{}",
            ],
            text=True,
        )
        resolved = json.loads(output)
        self.assertEqual(resolved["config_version"], 1)
        self.assertEqual(
            resolved["compatibility"], "legacy-combined-evidence-and-review"
        )
        self.assertEqual(
            resolved["repository_evidence"], resolved["independent_review"]
        )
    def test_v1_direct_codebase_memory_remains_recognized_but_unavailable(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "unavailable"):
            RESOLVER.resolve_builder_context(
                1, {"reconnaissance": {"provider": "codex-codebase-memory"}}
            )

    def test_legacy_optional_routes_remain_supported_in_v2(self) -> None:
        for provider in ("claude-codebase-memory", "claude-filesystem"):
            with self.subTest(provider=provider):
                resolved = RESOLVER.resolve_builder_context(
                    2,
                    {"repository_evidence": {
                        "provider": provider,
                        "skill": "claude-recon-implementation",
                    }},
                )
                self.assertEqual(resolved["repository_evidence"]["provider"], provider)

    def test_v1_rejects_v2_or_mixed_shape(self) -> None:
        for context in (
            {"repository_evidence": {}},
            {"reconnaissance": {}, "repository_evidence": {}},
        ):
            with self.subTest(context=context):
                with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "unknown"):
                    RESOLVER.resolve_builder_context(1, context)

    def test_v2_rejects_v1_or_mixed_shape(self) -> None:
        for context in (
            {"reconnaissance": {}},
            {"evidence_skill": "claude-recon-implementation", "repository_evidence": {}},
        ):
            with self.subTest(context=context):
                with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "unknown"):
                    RESOLVER.resolve_builder_context(2, context)

    def test_inconsistent_and_unknown_roles_fail_closed(self) -> None:
        cases = (
            {"repository_evidence": {"provider": "codex-codebase-memory", "skill": "wrong"}},
            {"repository_evidence": {"provider": "mystery"}},
            {"independent_review": {"provider": "mystery"}},
            {"independent_review": {"provider": "claude", "skill": "repo-search"}},
        )
        for context in cases:
            with self.subTest(context=context):
                with self.assertRaises(RESOLVER.ProviderResolutionError):
                    RESOLVER.resolve_builder_context(2, context)

    def test_unknown_config_version_is_rejected(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "version"):
            RESOLVER.resolve_builder_context(3, {})

    def test_boolean_config_version_is_rejected(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "version"):
            RESOLVER.resolve_builder_context(True, {})


if __name__ == "__main__":
    unittest.main()
