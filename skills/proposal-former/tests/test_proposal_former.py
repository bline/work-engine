from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


WORKSPACE = Path(__file__).parents[3]
ROOT = WORKSPACE / "skills" / "proposal-former"
LEGACY_FIXTURE = Path(__file__).parent / "fixtures" / "one-proposal"
FORMATION_FIXTURE = Path(__file__).parent / "fixtures" / "intake-formation"
PACKET_SCRIPT = WORKSPACE / "skills" / "proposal-packets" / "scripts" / "proposal_packets.py"
INTAKE_SCRIPT = WORKSPACE / "skills" / "idea-intake" / "scripts" / "idea_intake.py"

FORMATION_RUN_RECEIPT = {
    "producer": {
        "kind": "agent_role",
        "id": "proposal-former",
        "runtime_attribution": "/root/raw_intake_s2_recovery/fresh_proposal_former_s2",
    },
    "input_access": {
        "projection.json": "06e6b9c8f38fa3330e12806ec28a89257aa9c220d7f545aec4a3a55aa52b9309",
        "ideas/cache.md": "4c54389a16c1876adfad5014b71d2b9075f2868d39e17c5571a84d359275fbbc",
        "evidence/authority.md": "ca1fe2694bfc1561b95a24b2163c742bbe964e0d1425eae7cdb33af35001c9e7",
        "evidence/repository.md": "70195bffcce68188b9caa32030b39678f9dde4d06d580c2d93543806dc77c921",
    },
    "formation_before_output_access": True,
    "other_formation_evidence_accessed": False,
    "outcomes": {
        "zero/disposition.md": "aa82cf2594713caaf834ab1a0542d930746ba43d8e748d84d507cdbf33ec592e",
        "one/packets/cache-invalidation/packet.json": "c19dbf19877e72f4aa488dc6e9b143ad07d95f0fb5226986cbc6c6ca98439515",
        "one/packets/cache-invalidation/proposal.md": "1cbc1a57b9618b20cce389690a6c86aca0a37472814c947bb6f948c3c4370573",
        "multiple/packets/cache-invalidation/packet.json": "bdf2612ef043144ed3fe149945a9ee6185205fd21456b663fa66e9782f1193de",
        "multiple/packets/cache-invalidation/proposal.md": "e27ecd3a379023d985220f8abc8a054bd66d647dd26bc8987e17c534f8b89fa4",
        "multiple/packets/cache-observability/packet.json": "b25797885441140caa4acb88e1a5de0a52fdbc70647c6df652a602de588a0d96",
        "multiple/packets/cache-observability/proposal.md": "6686fb61efec72581d2395c94473a75bc59252784eab2848d31023ec6b56f6bd",
    },
    "limitations": [
        "No runtime inventory, implementation inspection, accepted cache contract, or permanent ownership decision was available.",
        "The zero, one, and multiple outcomes are mutually exclusive formation exercises, not simultaneous recommendations.",
    ],
}


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PACKETS = load_module("proposal_packets", PACKET_SCRIPT)
INTAKE = load_module("idea_intake", INTAKE_SCRIPT)


def run_git(
    repository: Path,
    *args: str,
    environment: dict[str, str] | None = None,
) -> str:
    return subprocess.run(
        ["git", "-C", str(repository), *args],
        check=True,
        capture_output=True,
        text=True,
        env=({**os.environ, **environment} if environment else None),
    ).stdout.strip()


def git_bytes(repository: Path, *args: str) -> bytes:
    return subprocess.run(
        ["git", "-C", str(repository), *args],
        check=True,
        capture_output=True,
    ).stdout


def digest_lines(content: bytes, start: int, end: int) -> str:
    return hashlib.sha256(
        b"".join(content.splitlines(keepends=True)[start - 1:end])
    ).hexdigest()


def repository_evidence(
    evidence_id: str,
    kind: str,
    owner: str,
    reference: str,
    revision: str,
    content: bytes,
) -> dict[str, object]:
    return {
        "evidence_id": evidence_id,
        "kind": kind,
        "owner": owner,
        "reference": reference,
        "revision": revision,
        "attribution": f"Bounded fixture evidence owned by {owner}.",
        "verification": {
            "mode": "repository_file",
            "integrity_sha256": hashlib.sha256(content).hexdigest(),
            "freshness_rule": "exact_revision",
        },
    }


class ProposalFormerTest(unittest.TestCase):
    def test_one_idea_forms_resumable_non_authorizing_packet_state(self) -> None:
        packets = PACKETS.discover_packets(LEGACY_FIXTURE)

        self.assertEqual(
            set(packets), {"work-engine.fixture.explain-stopped-automation"}
        )
        manifest = packets["work-engine.fixture.explain-stopped-automation"].manifest
        self.assertEqual(manifest["lifecycle_state"], "formed")
        self.assertEqual(manifest["placement"]["state"], "probable")
        self.assertEqual(manifest["uncertainty"]["state"], "unresolved")
        self.assertFalse(manifest["authority"]["implementation_authorized"])
        self.assertEqual(
            manifest["origin_refs"], [{"kind": "idea_source", "path": "idea.md"}]
        )

        narrative = (LEGACY_FIXTURE / manifest["narrative"]["path"]).read_text(
            encoding="utf-8"
        )
        for consumer_fact in (
            "Candidate and consequence",
            "Boundary and placement",
            "Relationships",
            "Uncertainty and evidence needs",
            "Authority",
        ):
            self.assertIn(consumer_fact, narrative)

    def test_intake_projection_forms_zero_one_or_multiple_immutable_outcomes(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            run_git(repository, "init", "-q")
            run_git(repository, "config", "user.name", "Formation Test")
            run_git(repository, "config", "user.email", "formation@example.invalid")

            source = repository / "ideas" / "cache.md"
            source.parent.mkdir()
            source_bytes = (
                b"Define observable cache invalidation behavior.\n"
                b"Expose cache invalidation outcomes to runtime consumers.\n"
            )
            source.write_bytes(source_bytes)
            evidence_dir = repository / "evidence"
            evidence_dir.mkdir()
            authority_bytes = (
                b"The fixture user authorizes assessment and formation testing only.\n"
            )
            observation_bytes = (
                b"Runtime consumer ownership and telemetry placement remain unresolved.\n"
            )
            (evidence_dir / "authority.md").write_bytes(authority_bytes)
            (evidence_dir / "repository.md").write_bytes(observation_bytes)
            run_git(repository, "add", ".")
            run_git(
                repository,
                "commit",
                "-qm",
                "bind formation source and evidence",
                environment={
                    "GIT_AUTHOR_DATE": "2000-01-01T00:00:00+00:00",
                    "GIT_COMMITTER_DATE": "2000-01-01T00:00:00+00:00",
                },
            )
            source_revision = run_git(repository, "rev-parse", "HEAD")
            blob_oid = run_git(
                repository, "rev-parse", f"{source_revision}:ideas/cache.md"
            )

            record_dir = repository / "records" / "cache-formation"
            record_dir.mkdir(parents=True)
            (record_dir / "assessment.md").write_text(
                "# Assessment\n\nOne independently addressable candidate is ready for formation.\n",
                encoding="utf-8",
            )
            record = {
                "schema_version": 1,
                "idea_id": "work-engine.idea.cache-formation",
                "assessment_id": "work-engine.intake.cache-formation",
                "source": {
                    "repository_revision": source_revision,
                    "blob_oid": blob_oid,
                    "path": "ideas/cache.md",
                    "range": {
                        "line_start": 1,
                        "line_end": 2,
                        "content_sha256": digest_lines(source_bytes, 1, 2),
                    },
                },
                "assessment": {
                    "revision": 1,
                    "producer": {"kind": "agent_role", "id": "idea-intake"},
                    "authority": {
                        "owner": "fixture user",
                        "scope": "assess and exercise formation without downstream authority",
                        "evidence_refs": ["evidence.authority"],
                    },
                    "purpose": "Provide one bounded candidate to proposal formation.",
                    "evidence_cutoff": {
                        "repository_revision": source_revision,
                        "description": "Temporary repository at assessment time.",
                    },
                    "published_state": "ready_for_handoff",
                    "reopening_conditions": [
                        "Runtime consumer evidence changes the candidate boundary."
                    ],
                },
                "narrative": {"path": "assessment.md"},
                "evidence": [
                    repository_evidence(
                        "evidence.authority",
                        "human_decision",
                        "fixture user",
                        "evidence/authority.md",
                        source_revision,
                        authority_bytes,
                    ),
                    repository_evidence(
                        "evidence.repository",
                        "repository_observation",
                        "temporary repository",
                        "evidence/repository.md",
                        source_revision,
                        observation_bytes,
                    ),
                ],
                "relationships": [],
                "claims": [
                    {
                        "claim_id": "claim.cache-formation",
                        "revision": 1,
                        "source_range": {
                            "line_start": 1,
                            "line_end": 2,
                            "content_sha256": digest_lines(source_bytes, 1, 2),
                        },
                        "statement": "Define cache invalidation behavior and make its outcomes observable.",
                        "statement_kind": "source_assertion",
                        "disposition": {
                            "state": "ready_for_proposal_formation",
                            "status": "adjudicated",
                            "rationale": "The fixture user authorized this bounded formation exercise.",
                            "authority_ref": "evidence.authority",
                        },
                        "uncertainty": [
                            "Whether behavior and observability are independently decidable."
                        ],
                        "evidence_refs": [
                            "evidence.authority",
                            "evidence.repository",
                        ],
                        "relationship_refs": [],
                        "candidate": {
                            "meaning": "Define observable cache invalidation behavior.",
                            "boundary": "Cache semantics and observability, excluding dashboard UI.",
                            "placement_hypotheses": [
                                "cache owner",
                                "shared telemetry owner",
                            ],
                            "evidence_still_needed": [
                                "runtime consumer inventory"
                            ],
                            "unresolved_decisions": [
                                "whether observability is independently decidable"
                            ],
                            "next_consumer": "proposal-former",
                        },
                    }
                ],
                "proposal_refs": [],
                "non_authorization": {
                    "cleanup_authorized": False,
                    "implementation_authorized": False,
                    "proposal_accepted": False,
                    "permanent_placement_settled": False,
                    "roadmap_priority_changed": False,
                },
            }
            record_path = record_dir / "record.json"
            record_path.write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )
            projection = INTAKE.build_projection(
                INTAKE.validate_record(record_path, repository)
            )
            fixture_projection = json.loads(
                (FORMATION_FIXTURE / "projection.json").read_text(encoding="utf-8")
            )
            self.assertEqual(projection, fixture_projection)
            self.assertEqual(
                FORMATION_RUN_RECEIPT["input_access"],
                {
                    "projection.json": hashlib.sha256(
                        (FORMATION_FIXTURE / "projection.json").read_bytes()
                    ).hexdigest(),
                    "ideas/cache.md": hashlib.sha256(source_bytes).hexdigest(),
                    "evidence/authority.md": hashlib.sha256(authority_bytes).hexdigest(),
                    "evidence/repository.md": hashlib.sha256(
                        observation_bytes
                    ).hexdigest(),
                },
            )
            self.assertTrue(FORMATION_RUN_RECEIPT["formation_before_output_access"])
            self.assertFalse(
                FORMATION_RUN_RECEIPT["other_formation_evidence_accessed"]
            )
            self.assertEqual(
                FORMATION_RUN_RECEIPT["outcomes"],
                {
                    str(path.relative_to(FORMATION_FIXTURE)): hashlib.sha256(
                        path.read_bytes()
                    ).hexdigest()
                    for path in sorted(FORMATION_FIXTURE.glob("**/*"))
                    if path.is_file() and path.name != "projection.json"
                },
            )

            self.assertEqual(
                [item["claim_id"] for item in projection["candidates"]],
                ["claim.cache-formation"],
            )
            candidate_evidence = set(projection["candidates"][0]["evidence_refs"])
            projected_evidence = {
                item["evidence_id"] for item in projection["evidence"]
            }
            self.assertEqual(candidate_evidence, projected_evidence)
            self.assertEqual(
                projected_evidence,
                {"evidence.authority", "evidence.repository"},
            )
            self.assertFalse(
                projection["non_authorization"]["implementation_authorized"]
            )

            formation_root = repository / "formation"
            formation_root.mkdir()
            shutil.copy2(
                FORMATION_FIXTURE / "projection.json",
                formation_root / "projection.json",
            )
            for outcome in ("zero", "one", "multiple"):
                outcome_root = formation_root / outcome
                shutil.copytree(FORMATION_FIXTURE / outcome, outcome_root)

            zero_root = formation_root / "zero"
            self.assertEqual(list(zero_root.rglob("packet.json")), [])
            disposition = (zero_root / "disposition.md").read_text(encoding="utf-8")
            for fact in (
                "fresh proposal-former fixture role",
                "../projection.json",
                projection["assessment_id"],
                "evidence.authority",
                "evidence.repository",
                "no surviving proposal",
                "does not authorize implementation",
            ):
                self.assertIn(fact, disposition)

            packets = PACKETS.discover_packets(formation_root)
            expected_locations = {
                "work-engine.fixture.cache-invalidation": Path(
                    "one/packets/cache-invalidation/packet.json"
                ),
                "work-engine.fixture.cache-invalidation-semantics": Path(
                    "multiple/packets/cache-invalidation/packet.json"
                ),
                "work-engine.fixture.cache-invalidation-observability": Path(
                    "multiple/packets/cache-observability/packet.json"
                ),
            }
            actual_locations = {
                proposal_id: packet.manifest_path.relative_to(formation_root)
                for proposal_id, packet in packets.items()
            }
            self.assertEqual(actual_locations, expected_locations)
            self.assertEqual(
                {
                    proposal_id
                    for proposal_id, path in actual_locations.items()
                    if path.parts[0] == "one"
                },
                {"work-engine.fixture.cache-invalidation"},
            )
            self.assertEqual(
                {
                    proposal_id
                    for proposal_id, path in actual_locations.items()
                    if path.parts[0] == "multiple"
                },
                {
                    "work-engine.fixture.cache-invalidation-semantics",
                    "work-engine.fixture.cache-invalidation-observability",
                },
            )

            for packet in packets.values():
                manifest = packet.manifest
                self.assertEqual(manifest["lifecycle_state"], "formed")
                self.assertFalse(manifest["authority"]["implementation_authorized"])
                self.assertEqual(
                    manifest["origin_refs"],
                    [
                        {
                            "kind": "intake_projection",
                            "path": "../../../projection.json",
                        }
                    ],
                )
                origin = (
                    packet.manifest_path.parent
                    / manifest["origin_refs"][0]["path"]
                ).resolve()
                self.assertEqual(
                    json.loads(origin.read_text(encoding="utf-8")), projection
                )
                narrative = (
                    packet.manifest_path.parent / manifest["narrative"]["path"]
                ).read_text(encoding="utf-8")
                for consumer_fact in (
                    "Candidate and consequence",
                    "Boundary and placement",
                    "Relationships",
                    "Uncertainty and evidence needs",
                    "Source provenance",
                    "Authority",
                ):
                    self.assertIn(consumer_fact, narrative)

            run_git(repository, "add", ".")
            run_git(
                repository,
                "commit",
                "-qm",
                "publish formed outcomes",
                environment={
                    "GIT_AUTHOR_DATE": "2000-01-01T00:00:01+00:00",
                    "GIT_COMMITTER_DATE": "2000-01-01T00:00:01+00:00",
                },
            )
            published_revision = run_git(repository, "rev-parse", "HEAD")
            one_narrative = (
                formation_root
                / "one"
                / "packets"
                / "cache-invalidation"
                / "proposal.md"
            )
            relative_narrative = one_narrative.relative_to(repository)
            published_bytes = git_bytes(
                repository, "show", f"{published_revision}:{relative_narrative}"
            )
            self.assertEqual(published_bytes, one_narrative.read_bytes())

            one_narrative.write_text(
                "# Mutated working-tree meaning\n", encoding="utf-8"
            )
            self.assertNotEqual(one_narrative.read_bytes(), published_bytes)
            self.assertEqual(
                git_bytes(
                    repository, "show", f"{published_revision}:{relative_narrative}"
                ),
                published_bytes,
            )
            self.assertEqual(
                json.loads(
                    git_bytes(
                        repository,
                        "show",
                        f"{published_revision}:formation/projection.json",
                    )
                ),
                projection,
            )


if __name__ == "__main__":
    unittest.main()
