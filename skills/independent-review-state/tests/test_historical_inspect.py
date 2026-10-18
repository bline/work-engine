from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


SCRIPTS = Path(__file__).parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))
import historical_inspect as READER  # noqa: E402

OWNER = READER.owner
RUN = READER.RUN_ID


def reference(owner: str, name: str, revision: str) -> dict:
    return {"owner": owner, "reference": name, "revision": revision,
            "integrity_sha256": "a" * 64, "freshness_rule": "exact_revision"}


class HistoricalReviewInspectTest(unittest.TestCase):
    @unittest.skipUnless(os.getenv("S5_REPAIR_GRANT"), "needs current private S5 repair grant")
    def test_v3_fixed_repair_descriptor_preserves_review_read_and_rejects_widening(self):
        grant = json.loads(Path(os.environ["S5_REPAIR_GRANT"]).read_text())
        with tempfile.TemporaryDirectory() as directory:
            private = Path(directory) / "private"
            private.mkdir(mode=0o700)
            path = private / "grant.json"
            def write(value):
                path.write_text(json.dumps(value))
                path.chmod(0o600)
            write(grant)
            admitted = READER.read_grant(path)
            result = READER.inspect(admitted)
            self.assertEqual(3, len(result["reviews"]))
            self.assertEqual(grant["reviews"][0]["head"], result["reviews"][0]["head"])
            for change in [
                lambda x: x["repair_contract"].update(commit_oid="0" * 40),
                lambda x: x["repair_contract"].update(extra=True),
                lambda x: x.update(fields=["checkpoint"]),
            ]:
                changed = json.loads(json.dumps(grant))
                change(changed)
                write(changed)
                with self.assertRaisesRegex(ValueError, "invalid|differs"):
                    READER.read_grant(path)
            raw = json.dumps(grant).replace('"commit_oid":', '"commit_oid":"0","commit_oid":', 1)
            path.write_text(raw)
            with self.assertRaisesRegex(ValueError, "duplicate"):
                READER.read_grant(path)

    def authority(self, episode: str, subject: dict, source: dict) -> dict:
        return {"schema_version": 1, "profile": OWNER.PROFILE, "grant_id": f"writer-{episode}",
                "identity": {"run_id": RUN, "slice_number": 6, "attempt_id": episode,
                             "plan_version": "plan", "review_obligation_id": episode,
                             "review_episode_id": episode},
                "source": source, "writer": {"logical_actor_id": episode, "provider": "test",
                    "generation": 1, "runtime_session_ref": reference("test", episode, "generation-1")},
                "readers": ["coordinating_builder"], "initial_subject": [subject],
                "expires_at": None, "predecessor_revision": None}

    def published(self, store, authority):
        digest = OWNER.digest(authority)
        begun = OWNER.begin(store, authority, digest, {"transition_id": "begin", "evidence_references": [],
            "claim_references": [], "unresolved_questions": []})
        result = OWNER.advance(store, authority, digest, begun["durable_revision"], "record_initial_result",
            "result", {"findings": [], "unresolved_questions": [], "evidence_references": [], "claim_references": []})
        return OWNER.stable_key(authority["identity"]), result["durable_revision"]

    def fixture(self, directory: str):
        repo = Path(directory).resolve() / "repository"
        repo.mkdir()
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        store = OWNER.DURABLE.GitRefDurableStateStore(repo, namespace="independent-review-state")
        cp = [{"kind": f"candidate-{i}", "attempt": i,
               "ref": f"refs/work-engine/checkpoints/{RUN}/slice-6/candidate-{i}", "oid": str(i) * 40}
              for i in range(1, 6)]
        cp.append({**cp[-1], "kind": "accepted", "ref": f"refs/work-engine/checkpoints/{RUN}/slice-6/accepted"})
        default_source = reference("test", "accepted-review-profile", "v1")
        first = self.authority("original", reference("slice-checkpoint", cp[0]["ref"], cp[0]["oid"]), default_source)
        first_key, first_head = self.published(store, first)
        instruction = self.authority("instruction", reference("slice-checkpoint", cp[2]["ref"], cp[2]["oid"]), default_source)
        instruction_key, instruction_head = self.published(store, instruction)
        source = reference("slice-supervisor", f"replacement-authority:reported_episode_cannot_bind_later_candidate:{store._ref(first_key)}@{first_head}", "selection")
        replacement = self.authority("replacement", reference("slice-checkpoint", cp[4]["ref"], cp[4]["oid"]), source)
        replacement_key, replacement_head = self.published(store, replacement)
        grant = {"schema_version": 1, "profile": READER.PROFILE, "grant_id": "current-local-read",
                 "repository": str(repo), "run_id": RUN, "slice_number": 6,
                 "issued_at": (datetime.now(timezone.utc) - timedelta(minutes=1)).isoformat(),
                 "expires_at": (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat(),
                 "issuer_provenance": "trusted-local-test", "peer_uid": os.geteuid(),
                 "audience": "local_terminal_inspection", "fields": sorted(READER.ALLOWED_FIELDS),
                 "checkpoints": cp, "gates": [{} for _ in range(5)],
                 "reviews": [{"key": first_key, "head": first_head},
                             {"key": instruction_key, "head": instruction_head},
                             {"key": replacement_key, "head": replacement_head}]}
        private = Path(directory) / "private"
        private.mkdir(mode=0o700)
        path = private / "grant.json"
        path.write_text(json.dumps(grant))
        path.chmod(0o600)
        return path, grant

    def test_private_grant_and_exact_linked_history(self):
        with tempfile.TemporaryDirectory() as directory:
            path, grant = self.fixture(directory)
            admitted = READER.read_grant(path)
            result = READER.inspect(admitted)
            self.assertEqual([2, 2, 2], [len(r["revisions"]) for r in result["reviews"]])
            self.assertEqual(grant["reviews"][0]["head"], result["replacement_links"][0]["original_reported_revision"])
            self.assertEqual("reported", result["reviews"][0]["revisions"][0]["phase"])
            self.assertNotIn("observation", json.dumps(result))
            wrong = {**admitted, "reviews": [*admitted["reviews"]]}
            wrong["reviews"][0] = {**wrong["reviews"][0], "head": "0" * 40}
            with self.assertRaisesRegex(ValueError, "head changed"):
                READER.inspect(wrong)

    def test_grant_permissions_and_symlink_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path, _ = self.fixture(directory)
            READER.read_grant(path)
            path.chmod(0o644)
            with self.assertRaisesRegex(ValueError, "file type, owner, mode"):
                READER.read_grant(path)
            path.chmod(0o600)
            path.parent.chmod(0o600)
            with self.assertRaisesRegex(ValueError, "directory owner or mode"):
                READER.read_grant(path)
            path.parent.chmod(0o700)
            alias = path.parent / "alias.json"
            alias.symlink_to(path)
            with self.assertRaises(OSError):
                READER.read_grant(alias)

    def test_v2_grant_preserves_exact_review_read_without_extra_authority(self):
        with tempfile.TemporaryDirectory() as directory:
            path, grant = self.fixture(directory)
            legacy = READER.inspect(READER.read_grant(path))
            grant.update(schema_version=2, profile=READER.PROFILE_V2,
                         fields=sorted(READER.ALLOWED_FIELDS_V2),
                         provider={}, scheduler={}, metrics={})
            path.write_text(json.dumps(grant))
            expanded = READER.inspect(READER.read_grant(path))
            self.assertEqual(legacy["reviews"], expanded["reviews"])
            grant.pop("scheduler")
            path.write_text(json.dumps(grant))
            with self.assertRaisesRegex(ValueError, "schema"):
                READER.read_grant(path)

    def test_replace_ref_cannot_substitute_review_blob(self):
        with tempfile.TemporaryDirectory() as directory:
            _, grant = self.fixture(directory)
            baseline = READER.inspect(grant)
            head = grant["reviews"][2]["head"]
            repo = grant["repository"]
            alternate = subprocess.run(["git", "-C", repo, "hash-object", "-w", "--stdin"],
                                       input=b'{"substituted-finding":true}', stdout=subprocess.PIPE,
                                       check=True).stdout.decode().strip()
            subprocess.run(["git", "-C", repo, "replace", head, alternate], check=True)
            ordinary = subprocess.run(["git", "-C", repo, "cat-file", "blob", head],
                                      stdout=subprocess.PIPE, check=True).stdout
            self.assertIn(b"substituted-finding", ordinary)
            current = READER.inspect(grant)
            self.assertEqual(baseline["reviews"], current["reviews"])


if __name__ == "__main__":
    unittest.main()
