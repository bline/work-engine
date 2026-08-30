from __future__ import annotations

from datetime import datetime, timezone
import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).parents[1] / "scripts" / "openrouter_routing_attestation.py"
SPEC = importlib.util.spec_from_file_location("openrouter_routing_attestation", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OpenRouterRoutingAttestationTest(unittest.TestCase):
    def setUp(self) -> None:
        self.guardrail_id = "550e8400-e29b-41d4-a716-446655440001"
        self.key_hash = "c56454edb818d6b14bc0d61c46025f1450b0f4012d12304ab40aacb519fcbc93"
        self.model = "anthropic/claude-sonnet-5-20260630"
        self.guardrail = {
            "id": self.guardrail_id,
            "allowed_providers": ["anthropic"],
            "allowed_models": [self.model],
        }
        self.assignment = {
            "id": "assignment-1",
            "guardrail_id": self.guardrail_id,
            "key_hash": self.key_hash,
        }

    def test_observes_exact_guardrail_and_paginated_assignment(self) -> None:
        calls: list[tuple[str, dict[str, int] | None]] = []

        def get_json(path: str, query: dict[str, int] | None):
            calls.append((path, query))
            if path != "/guardrails/assignments/keys":
                return {"data": self.guardrail}
            if query == {"offset": 0, "limit": 100}:
                return {"data": [self.assignment], "total_count": 2}
            return {
                "data": [{
                    "id": "assignment-2",
                    "guardrail_id": "another-guardrail",
                    "key_hash": "another-key",
                }],
                "total_count": 2,
            }

        result = MODULE.observe_attestation(
            get_json,
            guardrail_id=self.guardrail_id,
            key_hash=self.key_hash,
            model=self.model,
            observed_at=datetime(2026, 8, 29, 18, 0, tzinfo=timezone.utc),
        )

        self.assertEqual(result["schema_version"], 1)
        self.assertEqual(result["observed_at"], "2026-08-29T18:00:00Z")
        self.assertEqual(result["allowed_providers"], ["anthropic"])
        self.assertEqual(result["allowed_models"], [self.model])
        self.assertEqual(result["assignment_guardrail_id"], self.guardrail_id)
        self.assertEqual(calls, [
            (f"/guardrails/{self.guardrail_id}", None),
            ("/guardrails/assignments/keys", {"offset": 0, "limit": 100}),
            ("/guardrails/assignments/keys", {"offset": 1, "limit": 100}),
        ])

    def test_rejects_provider_fallbacks(self) -> None:
        guardrail = dict(
            self.guardrail, allowed_providers=["anthropic", "amazon-bedrock"]
        )
        with self.assertRaisesRegex(MODULE.AttestationError, "only provider"):
            self.build(guardrail, [self.assignment])

    def test_rejects_additional_or_missing_models(self) -> None:
        for models in (None, [self.model, "anthropic/claude-opus-4.1"]):
            with self.subTest(models=models):
                with self.assertRaisesRegex(
                    MODULE.AttestationError, "exact requested model"
                ):
                    self.build(dict(self.guardrail, allowed_models=models), [self.assignment])

    def test_rejects_wrong_or_duplicate_key_assignments(self) -> None:
        wrong = dict(self.assignment, guardrail_id="another-guardrail")
        with self.assertRaisesRegex(MODULE.AttestationError, "not assigned"):
            self.build(self.guardrail, [wrong])
        with self.assertRaisesRegex(MODULE.AttestationError, "exactly one"):
            self.build(
                self.guardrail,
                [self.assignment, dict(self.assignment, id="assignment-2")],
            )

    def test_rejects_incomplete_or_changing_pagination(self) -> None:
        responses = iter([
            {"data": [self.assignment], "total_count": 2},
            {"data": [], "total_count": 2},
        ])
        with self.assertRaisesRegex(MODULE.AttestationError, "ended before"):
            MODULE._read_key_assignments(lambda _path, _query: next(responses))

        responses = iter([
            {"data": [self.assignment], "total_count": 2},
            {"data": [dict(self.assignment, key_hash="other")], "total_count": 3},
        ])
        with self.assertRaisesRegex(MODULE.AttestationError, "changed"):
            MODULE._read_key_assignments(lambda _path, _query: next(responses))

    def test_management_key_never_enters_artifact(self) -> None:
        result = self.build(self.guardrail, [self.assignment])
        self.assertNotIn("management", str(result).lower())

    def build(self, guardrail, assignments):
        return MODULE.build_attestation(
            guardrail,
            assignments,
            guardrail_id=self.guardrail_id,
            key_hash=self.key_hash,
            model=self.model,
        )


if __name__ == "__main__":
    unittest.main()
