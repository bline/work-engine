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
