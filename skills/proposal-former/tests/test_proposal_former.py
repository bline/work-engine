from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
FIXTURE = Path(__file__).parent / "fixtures" / "one-proposal"
PACKET_SCRIPT = ROOT.parent / "proposal-packets" / "scripts" / "proposal_packets.py"
SPEC = importlib.util.spec_from_file_location("proposal_packets", PACKET_SCRIPT)
assert SPEC and SPEC.loader
PACKETS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKETS)


class ProposalFormerTest(unittest.TestCase):
    def test_one_idea_forms_resumable_non_authorizing_packet_state(self) -> None:
        packets = PACKETS.discover_packets(FIXTURE)

        self.assertEqual(set(packets), {"work-engine.fixture.explain-stopped-automation"})
        manifest = packets["work-engine.fixture.explain-stopped-automation"].manifest
        self.assertEqual(manifest["lifecycle_state"], "formed")
        self.assertEqual(manifest["placement"]["state"], "probable")
        self.assertEqual(manifest["uncertainty"]["state"], "unresolved")
        self.assertFalse(manifest["authority"]["implementation_authorized"])
        self.assertEqual(manifest["origin_refs"], [{"kind": "idea_source", "path": "idea.md"}])

        narrative = (FIXTURE / manifest["narrative"]["path"]).read_text(encoding="utf-8")
        for consumer_fact in (
            "Candidate and consequence",
            "Boundary and placement",
            "Relationships",
            "Uncertainty and evidence needs",
            "Authority",
        ):
            self.assertIn(consumer_fact, narrative)


if __name__ == "__main__":
    unittest.main()
