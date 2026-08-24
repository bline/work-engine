from __future__ import annotations

import hashlib
import importlib.util
import json
from copy import deepcopy
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[3]
SCRIPT = ROOT / "skills" / "idea-intake" / "scripts" / "idea_intake.py"
FIXTURE = ROOT / "skills" / "idea-intake" / "tests" / "fixtures" / "representative"
SPEC = importlib.util.spec_from_file_location("idea_intake", SCRIPT)
assert SPEC and SPEC.loader
INTAKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INTAKE)


def run_git(repository: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repository), *args], check=True, capture_output=True, text=True).stdout.strip()


def digest_lines(content: bytes, start: int, end: int) -> str:
    return hashlib.sha256(b"".join(content.splitlines(keepends=True)[start - 1:end])).hexdigest()


class IdeaIntakeTest(unittest.TestCase):
    def test_vertical_source_move_and_projection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            run_git(repository, "init", "-q")
            run_git(repository, "config", "user.name", "Intake Test")
            run_git(repository, "config", "user.email", "intake@example.invalid")
            source = repository / "ideas" / "raw.md"
            source.parent.mkdir()
            source_bytes = (FIXTURE / "raw-idea.md").read_bytes()
            source.write_bytes(source_bytes)
            evidence_dir = repository / "evidence"
            evidence_dir.mkdir()
            evidence_files = {
                "authority.md": (FIXTURE / "authority.md").read_bytes(),
                "prior-reconciliation.json": (FIXTURE / "prior-reconciliation.json").read_bytes(),
                "observation.md": (FIXTURE / "observation.md").read_bytes(),
            }
            for name, content in evidence_files.items():
                (evidence_dir / name).write_bytes(content)
            run_git(repository, "add", ".")
            run_git(repository, "commit", "-qm", "raw source")
            source_revision = run_git(repository, "rev-parse", "HEAD")
            blob_oid = run_git(repository, "rev-parse", f"{source_revision}:ideas/raw.md")

            intake_dir = repository / "records" / "sample"
            intake_dir.mkdir(parents=True)
            (intake_dir / "assessment.md").write_bytes((FIXTURE / "assessment.md").read_bytes())
            authority_revision = source_revision
            record = {
                "schema_version": 1,
                "idea_id": "work-engine.idea.sample",
                "assessment_id": "work-engine.intake.sample",
                "source": {"repository_revision": source_revision, "blob_oid": blob_oid, "path": "ideas/raw.md", "range": {"line_start": 1, "line_end": 3, "content_sha256": digest_lines(source_bytes, 1, 3)}},
                "assessment": {
                    "revision": 1,
                    "producer": {"kind": "agent_role", "id": "idea-intake"},
                    "authority": {"owner": "test-user", "scope": "assess this source without cleanup", "evidence_refs": ["evidence.authority"]},
                    "purpose": "Identify independently decidable candidate meaning.",
                    "evidence_cutoff": {"repository_revision": authority_revision, "description": "Temporary repository at intake start."},
                    "published_state": "ready_for_handoff",
                    "reopening_conditions": ["New product evidence changes the candidate boundary."]
                },
                "narrative": {"path": "assessment.md"},
                "evidence": [
                    {"evidence_id": "evidence.authority", "kind": "human_decision", "owner": "test-user", "reference": "evidence/authority.md", "revision": source_revision, "attribution": "Fixture-only authority.", "verification": {"mode": "repository_file", "integrity_sha256": hashlib.sha256(evidence_files["authority.md"]).hexdigest(), "freshness_rule": "exact_revision"}},
                    {"evidence_id": "evidence.manual", "kind": "prior_reconciliation", "owner": "ideas reconciliation snapshot", "reference": "evidence/prior-reconciliation.json", "revision": source_revision, "attribution": "Prior file-level evidence, not the current claim owner.", "verification": {"mode": "repository_file", "integrity_sha256": hashlib.sha256(evidence_files["prior-reconciliation.json"]).hexdigest(), "freshness_rule": "exact_revision"}},
                    {"evidence_id": "evidence.repository", "kind": "repository_observation", "owner": "temporary repository", "reference": "evidence/observation.md", "revision": source_revision, "attribution": "Observed by assessor.", "verification": {"mode": "repository_file", "integrity_sha256": hashlib.sha256(evidence_files["observation.md"]).hexdigest(), "freshness_rule": "exact_revision"}}
                ],
                "relationships": [
                    {"relationship_id": "relationship.export", "type": "apparently_represented_by_proposal", "source_claim_id": "claim.export", "target": {"kind": "proposal", "id": "work-engine.existing-export", "reference": "proposal fixture"}, "status": "nominated", "evidence_refs": ["evidence.manual"], "authority_ref": None, "rationale": "Manual reconciliation is prior evidence, not proof."}
                ],
                "claims": [
                    {"claim_id": "claim.export", "revision": 1, "source_range": {"line_start": 1, "line_end": 1, "content_sha256": digest_lines(source_bytes, 1, 1)}, "statement": "Export may already be represented.", "statement_kind": "assessor_interpretation", "disposition": {"state": "represented_by_proposal", "status": "nominated", "rationale": "Needs proposal-owner confirmation.", "authority_ref": None}, "uncertainty": ["Semantic equivalence is unconfirmed."], "evidence_refs": ["evidence.manual"], "relationship_refs": ["relationship.export"], "candidate": None},
                    {"claim_id": "claim.cache", "revision": 1, "source_range": {"line_start": 2, "line_end": 2, "content_sha256": digest_lines(source_bytes, 2, 2)}, "statement": "Design cache invalidation behavior.", "statement_kind": "source_assertion", "disposition": {"state": "ready_for_proposal_formation", "status": "adjudicated", "rationale": "Authorized as independently decidable candidate meaning.", "authority_ref": "evidence.authority"}, "uncertainty": ["Permanent placement remains open."], "evidence_refs": ["evidence.authority", "evidence.repository"], "relationship_refs": [], "candidate": {"meaning": "Define observable cache invalidation behavior.", "boundary": "Cache invalidation semantics, excluding dashboard UI.", "placement_hypotheses": ["planning capability"], "evidence_still_needed": ["runtime consumer inventory"], "unresolved_decisions": [], "next_consumer": "proposal-former"}},
                    {"claim_id": "claim.dashboard", "revision": 1, "source_range": {"line_start": 3, "line_end": 3, "content_sha256": digest_lines(source_bytes, 3, 3)}, "statement": "Whether a dashboard is desired is unresolved.", "statement_kind": "open_question", "disposition": {"state": "unresolved", "status": "nominated", "rationale": "Needs product preference.", "authority_ref": None}, "uncertainty": ["No product decision."], "evidence_refs": [], "relationship_refs": [], "candidate": None}
                ],
                "proposal_refs": [],
                "non_authorization": {"cleanup_authorized": False, "implementation_authorized": False, "proposal_accepted": False, "permanent_placement_settled": False, "roadmap_priority_changed": False}
            }
            record_path = intake_dir / "record.json"
            record_path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
            validated = INTAKE.validate_record(record_path, repository)
            projection = INTAKE.build_projection(validated)
            expected = json.loads((FIXTURE / "expected-projection.json").read_text(encoding="utf-8"))
            self.assertEqual([item["claim_id"] for item in projection["candidates"]], expected["candidate_claim_ids"])
            self.assertEqual(projection["candidates"][0]["candidate"]["next_consumer"], "proposal-former")
            self.assertFalse(projection["non_authorization"]["implementation_authorized"])

            run_git(repository, "mv", "ideas/raw.md", "ideas/moved.md")
            run_git(repository, "commit", "-qm", "move current source")
            moved_validation = INTAKE.validate_record(record_path, repository)
            self.assertEqual(moved_validation.source_bytes, source_bytes)
            self.assertTrue((repository / projection["canonical_record"]).is_file())
            self.assertEqual(run_git(repository, "rev-parse", f"{source_revision}:ideas/raw.md"), blob_oid)
            (intake_dir / "malformed-proposal.json").write_text(
                json.dumps({"proposal_id": "NOT A PROPOSAL ID"}), encoding="utf-8"
            )

            mutations = (
                ("tampered range", lambda value: value["claims"][0]["source_range"].update(content_sha256="0" * 64), "does not match source bytes"),
                ("stale subject", lambda value: value["source"].update(repository_revision=run_git(repository, "rev-parse", "HEAD")), "git rev-parse"),
                ("unknown field", lambda value: value.update(unexpected=True), "unknown fields"),
                ("missing field", lambda value: value.pop("narrative"), "missing fields"),
                ("duplicate claim", lambda value: value["claims"].append(deepcopy(value["claims"][0])), "claim IDs must be unique"),
                ("dangling evidence", lambda value: value["claims"][0]["evidence_refs"].append("evidence.missing"), "dangling evidence refs"),
                ("dangling relationship", lambda value: value["claims"][1]["relationship_refs"].append("relationship.missing"), "dangling relationship refs"),
                ("nomination claims authority", lambda value: value["claims"][0]["disposition"].update(authority_ref="evidence.authority"), "nomination cannot claim"),
                ("claim wrong authority kind", lambda value: value["claims"][1]["disposition"].update(authority_ref="evidence.repository"), "mechanically verified authority evidence"),
                ("relationship wrong authority kind", lambda value: value["relationships"][0].update(status="adjudicated", authority_ref="evidence.repository"), "mechanically verified authority evidence"),
                ("external authority ineligible", lambda value: value["evidence"][0].update(verification={"mode": "external_attestation", "integrity_sha256": "0" * 64, "freshness_rule": "attested"}), "mechanically verified decision or contract evidence"),
                ("tampered evidence", lambda value: value["evidence"][0]["verification"].update(integrity_sha256="0" * 64), "does not match evidence bytes"),
                ("missing disposition relationship", lambda value: value["claims"][0].update(relationship_refs=[]), "correctly typed relationship"),
                ("wrong disposition relationship kind", lambda value: value["relationships"][0]["target"].update(kind="implementation"), "correctly typed relationship"),
                ("malformed proposal ID", lambda value: value["proposal_refs"].append({"proposal_id": "NOT A PROPOSAL ID", "path": "malformed-proposal.json", "revision": source_revision}), "proposal_id is invalid"),
                ("file-wide collapse", lambda value: value.update(file_disposition="done"), "unknown fields"),
                ("cleanup overreach", lambda value: value["non_authorization"].update(cleanup_authorized=True), "cannot authorize downstream effects"),
                ("implementation overreach", lambda value: value["non_authorization"].update(implementation_authorized=True), "cannot authorize downstream effects"),
            )
            for name, mutate, message in mutations:
                with self.subTest(name=name):
                    invalid = deepcopy(record)
                    mutate(invalid)
                    record_path.write_text(json.dumps(invalid), encoding="utf-8")
                    with self.assertRaisesRegex(INTAKE.IntakeError, message):
                        INTAKE.validate_record(record_path, repository)

            record_path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
            completed = subprocess.run(
                ["python3", str(SCRIPT), "project", str(record_path), "--repository", str(repository)],
                check=True, capture_output=True, text=True,
            )
            self.assertEqual(json.loads(completed.stdout)["status"], "valid")


if __name__ == "__main__":
    unittest.main()
