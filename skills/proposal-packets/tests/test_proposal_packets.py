from __future__ import annotations

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "proposal_packets.py"
FIXTURES = Path(__file__).parent / "fixtures"
SPEC = importlib.util.spec_from_file_location("proposal_packets", SCRIPT)
assert SPEC and SPEC.loader
PACKETS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKETS)


class ProposalPacketsTest(unittest.TestCase):
    def test_authority_decision_transition_is_durable_conditional_and_non_authorizing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "packets"
            shutil.copytree(FIXTURES, repository)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            subprocess.run(["git", "-C", str(repository), "config", "user.name", "Fixture"], check=True)
            subprocess.run(["git", "-C", str(repository), "config", "user.email", "fixture@example.invalid"], check=True)
            subprocess.run(["git", "-C", str(repository), "add", "."], check=True)
            subprocess.run(["git", "-C", str(repository), "commit", "-qm", "fixture baseline"], check=True)
            revision = subprocess.run(
                ["git", "-C", str(repository), "rev-parse", "HEAD"],
                check=True, capture_output=True, text=True,
            ).stdout.strip()
            manifest_path = repository / "formed-beta" / "packet.json"
            packet = json.loads(manifest_path.read_text(encoding="utf-8"))
            decision = {
                "schema_version": 1,
                "proposal_id": packet["proposal_id"],
                "source": {
                    "repository_revision": revision,
                    "lifecycle_state": packet["lifecycle_state"],
                    "placement_state": packet["placement"]["state"],
                },
                "decision": {
                    "disposition": "approve_proposal_meaning",
                    "lifecycle_state": "decided",
                    "placement_state": "probable",
                    "placement_claim": "A packet-adjacent profile is provisionally selected.",
                    "rationale": "The authority approved meaning without settling architecture.",
                    "reopening_conditions": ["A runtime consumer requires a different owner."],
                },
                "authority": {
                    "decision_owner": packet["authority"]["decision_owner"],
                    "exercised_by": "fixture user",
                    "evidence": "Explicit fixture authority input.",
                },
                "constraints": {
                    "permanent_architecture_settled": False,
                    "roadmap_priority_changed": False,
                    "implementation_authorized": False,
                },
            }
            decision_input = repository / "decision-input.json"
            decision_input.write_text(json.dumps(decision), encoding="utf-8")

            PACKETS.transition_packet(manifest_path, decision_input, repository)
            transitioned = PACKETS.discover_packets(repository)[packet["proposal_id"]].manifest
            self.assertEqual(transitioned["lifecycle_state"], "decided")
            self.assertEqual(transitioned["placement"]["state"], "probable")
            self.assertFalse(transitioned["authority"]["implementation_authorized"])

            stale = deepcopy(decision)
            stale["source"]["lifecycle_state"] = "placement_uncertain"
            stale["source"]["placement_state"] = "probable"
            (repository / "formed-alpha" / "stale.json").write_text(
                json.dumps(stale | {"proposal_id": "work-engine.fixture.formed-alpha"}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PACKETS.PacketError, "source lifecycle is stale"):
                PACKETS.transition_packet(
                    repository / "formed-alpha" / "packet.json",
                    repository / "formed-alpha" / "stale.json",
                    repository,
                )

            unauthorized = deepcopy(decision)
            unauthorized["constraints"]["implementation_authorized"] = True
            unauthorized_path = repository / "unauthorized.json"
            unauthorized_path.write_text(json.dumps(unauthorized), encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "implementation_authorized must be false"):
                PACKETS.load_decision(unauthorized_path)

            mismatched = deepcopy(decision)
            mismatched["authority"]["decision_owner"] = "another owner"
            mismatch_path = repository / "formed-alpha" / "mismatch.json"
            mismatch_path.write_text(
                json.dumps(mismatched | {"proposal_id": "work-engine.fixture.formed-alpha"}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PACKETS.PacketError, "decision owner does not match"):
                PACKETS.transition_packet(
                    repository / "formed-alpha" / "packet.json", mismatch_path, repository,
                )

            strengthened = deepcopy(decision)
            strengthened["decision"]["disposition"] = "defer_for_dogfooding"
            strengthened_path = repository / "strengthened.json"
            strengthened_path.write_text(json.dumps(strengthened), encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "disposition and placement state"):
                PACKETS.load_decision(strengthened_path)

    def test_repository_discovery_validates_related_packets_and_path_independent_identity(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "validate", str(FIXTURES)],
            check=True,
            capture_output=True,
            text=True,
        )
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "valid")
        self.assertEqual(result["packet_count"], 2)
        self.assertEqual(
            result["proposal_ids"],
            ["work-engine.fixture.formed-alpha", "work-engine.fixture.formed-beta"],
        )

        packets = PACKETS.discover_packets(FIXTURES)
        beta = packets["work-engine.fixture.formed-beta"].manifest
        self.assertEqual(beta["relationships"][0]["target_id"], "work-engine.fixture.formed-alpha")
        self.assertTrue(beta["relationships"][0]["causal"])

        with tempfile.TemporaryDirectory() as directory:
            renamed = Path(directory) / "renamed-packet-repository"
            shutil.copytree(FIXTURES, renamed)
            self.assertEqual(set(PACKETS.discover_packets(renamed)), set(packets))

    def test_duplicate_ids_and_dangling_relationships_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "packets"
            shutil.copytree(FIXTURES, repository)
            beta_path = repository / "formed-beta" / "packet.json"
            beta = json.loads(beta_path.read_text(encoding="utf-8"))
            beta["proposal_id"] = "work-engine.fixture.formed-alpha"
            beta_path.write_text(json.dumps(beta), encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "duplicate proposal_id"):
                PACKETS.discover_packets(repository)

            beta["proposal_id"] = "work-engine.fixture.formed-beta"
            beta["relationships"][0]["target_id"] = "work-engine.fixture.missing"
            beta_path.write_text(json.dumps(beta), encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "unresolved target_id"):
                PACKETS.discover_packets(repository)

    def test_decision_record_presence_matches_lifecycle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "packets"
            shutil.copytree(FIXTURES, repository)
            alpha_path = repository / "formed-alpha" / "packet.json"
            alpha = json.loads(alpha_path.read_text(encoding="utf-8"))
            alpha["lifecycle_state"] = "decided"
            alpha_path.write_text(json.dumps(alpha), encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "has no decision.json"):
                PACKETS.discover_packets(repository)

            alpha["lifecycle_state"] = "formed"
            alpha_path.write_text(json.dumps(alpha), encoding="utf-8")
            (alpha_path.parent / "decision.json").write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "cannot have a decision.json"):
                PACKETS.discover_packets(repository)

    def test_narrative_origin_authority_and_uncertainty_are_closed_invariants(self) -> None:
        source = json.loads((FIXTURES / "formed-alpha" / "packet.json").read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as directory:
            packet_dir = Path(directory) / "packet"
            shutil.copytree(FIXTURES / "formed-alpha", packet_dir)

            for mutation, message in (
                (lambda value: value["narrative"].update(path="missing.md"), "narrative.path"),
                (lambda value: value["origin_refs"].append({"path": "missing-origin.md", "kind": "idea"}), "origin_refs"),
                (lambda value: value["authority"].update(implementation_authorized=True), "must be false"),
                (lambda value: value["uncertainty"].update(state="certain"), "uncertainty.state"),
            ):
                candidate = deepcopy(source)
                mutation(candidate)
                (packet_dir / "packet.json").write_text(json.dumps(candidate), encoding="utf-8")
                with self.assertRaisesRegex(PACKETS.PacketError, message):
                    PACKETS.load_packet(packet_dir / "packet.json", Path(directory))


if __name__ == "__main__":
    unittest.main()
