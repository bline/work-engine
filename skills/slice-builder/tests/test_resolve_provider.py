from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "resolve_provider.py"
SPEC = importlib.util.spec_from_file_location("resolve_provider", SCRIPT)
assert SPEC and SPEC.loader
RESOLVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RESOLVER)


class ResolveProviderTest(unittest.TestCase):
    def test_default_resolves_to_claude_codebase_memory_adapter(self) -> None:
        self.assertEqual(
            RESOLVER.resolve_provider({}),
            {
                "provider": "claude-codebase-memory",
                "evidence_skill": "claude-recon-implementation",
            },
        )

    def test_explicit_supported_provider_and_adapter_resolve(self) -> None:
        self.assertEqual(
            RESOLVER.resolve_provider(
                {
                    "reconnaissance": {"provider": "claude-filesystem"},
                    "evidence_skill": "claude-recon-implementation",
                }
            )["provider"],
            "claude-filesystem",
        )

    def test_claude_codebase_memory_provider_resolves(self) -> None:
        self.assertEqual(
            RESOLVER.resolve_provider(
                {
                    "reconnaissance": {"provider": "claude-codebase-memory"},
                    "evidence_skill": "claude-recon-implementation",
                }
            ),
            {
                "provider": "claude-codebase-memory",
                "evidence_skill": "claude-recon-implementation",
            },
        )

    def test_inconsistent_adapter_is_rejected(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "requires evidence_skill"):
            RESOLVER.resolve_provider(
                {
                    "reconnaissance": {"provider": "claude-filesystem"},
                    "evidence_skill": "different-adapter",
                }
            )

    def test_unknown_provider_is_rejected(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "unknown"):
            RESOLVER.resolve_provider({"reconnaissance": {"provider": "mystery"}})

    def test_misspelled_context_key_does_not_silently_default(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "unknown builder.context"):
            RESOLVER.resolve_provider(
                {"reconnaisance": {"provider": "claude-codebase-memory"}}
            )

    def test_codex_codebase_memory_provider_is_unavailable(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "unavailable"):
            RESOLVER.resolve_provider(
                {"reconnaissance": {"provider": "codex-codebase-memory"}}
            )

    def test_auto_is_explicitly_deferred(self) -> None:
        with self.assertRaisesRegex(RESOLVER.ProviderResolutionError, "deferred"):
            RESOLVER.resolve_provider({"reconnaissance": {"provider": "auto"}})


if __name__ == "__main__":
    unittest.main()
