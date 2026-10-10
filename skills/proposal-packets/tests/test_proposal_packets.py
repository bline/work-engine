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

    def test_placement_revision_preserves_superseded_decision_and_validates_history(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "packets"
            shutil.copytree(FIXTURES, repository)

            def git(*args: str) -> str:
                return subprocess.run(["git", "-C", str(repository), *args],
                                      check=True, capture_output=True, text=True).stdout.strip()

            git("init", "-q")
            git("config", "user.name", "Fixture")
            git("config", "user.email", "fixture@example.invalid")
            git("add", ".")
            git("commit", "-qm", "fixture baseline")
            manifest_path = repository / "formed-beta" / "packet.json"
            packet = json.loads(manifest_path.read_text(encoding="utf-8"))
            authority = {
                "decision_owner": packet["authority"]["decision_owner"],
                "exercised_by": "fixture user",
                "evidence": "Explicit fixture authority input.",
            }
            constraints = {
                "permanent_architecture_settled": False,
                "roadmap_priority_changed": False,
                "implementation_authorized": False,
            }
            first = {
                "schema_version": 1,
                "proposal_id": packet["proposal_id"],
                "source": {"repository_revision": git("rev-parse", "HEAD"),
                           "lifecycle_state": packet["lifecycle_state"],
                           "placement_state": packet["placement"]["state"]},
                "decision": {"disposition": "approve_proposal_meaning", "lifecycle_state": "decided",
                             "placement_state": "probable",
                             "placement_claim": "Provider A probably hosts the projection.",
                             "rationale": "Meaning approved.",
                             "reopening_conditions": ["Provider A cannot host it."]},
                "authority": authority,
                "constraints": constraints,
            }
            first_input = repository / "first.json"
            first_input.write_text(json.dumps(first), encoding="utf-8")
            decision_path = PACKETS.transition_packet(manifest_path, first_input, repository)
            first_input.unlink()
            git("add", ".")
            git("commit", "-qm", "first decision")
            prior_bytes = decision_path.read_bytes()
            prior_digest = PACKETS.hashlib.sha256(prior_bytes).hexdigest()

            revision = {
                "schema_version": 2,
                "proposal_id": packet["proposal_id"],
                "source": {"repository_revision": git("rev-parse", "HEAD"),
                           "lifecycle_state": "decided", "placement_state": "probable"},
                "decision": {"disposition": "revise_placement", "lifecycle_state": "decided",
                             "placement_state": "probable",
                             "placement_claim": "Observer B probably supplies the observation.",
                             "rationale": "Later evidence withdrew the projection extension.",
                             "reopening_conditions": ["Observer B cannot bind a coherent snapshot."]},
                "authority": authority,
                "constraints": constraints,
                "supersedes": {"path": "decision-history/first.json", "sha256": prior_digest},
            }

            def attempt(record: dict, name: str) -> Path:
                path = repository / name
                path.write_text(json.dumps(record), encoding="utf-8")
                return path

            wrong_digest = deepcopy(revision)
            wrong_digest["supersedes"]["sha256"] = "0" * 64
            with self.assertRaisesRegex(PACKETS.PacketError, "does not match the current decision"):
                PACKETS.transition_packet(manifest_path, attempt(wrong_digest, "wrong.json"), repository)
            self.assertEqual(decision_path.read_bytes(), prior_bytes)

            unsuperseded = deepcopy(revision)
            unsuperseded["supersedes"] = None
            with self.assertRaisesRegex(PACKETS.PacketError, "only a placement revision"):
                PACKETS.load_decision(attempt(unsuperseded, "unsuperseded.json"))

            approval_from_decided = deepcopy(revision)
            approval_from_decided["decision"]["disposition"] = "approve_proposal_meaning"
            with self.assertRaisesRegex(PACKETS.PacketError, "only a placement revision may start"):
                PACKETS.load_decision(attempt(approval_from_decided, "approval.json"))

            for name in ("wrong.json", "unsuperseded.json", "approval.json"):
                (repository / name).unlink()
            PACKETS.transition_packet(manifest_path, attempt(revision, "revision.json"), repository)
            (repository / "revision.json").unlink()
            archived = repository / "formed-beta" / "decision-history" / "first.json"
            self.assertEqual(archived.read_bytes(), prior_bytes)
            current = PACKETS.discover_packets(repository)[packet["proposal_id"]].manifest
            self.assertEqual(current["placement"]["claim"], "Observer B probably supplies the observation.")
            self.assertFalse(current["authority"]["implementation_authorized"])

            archived.write_text(archived.read_text(encoding="utf-8").replace("Provider A", "Provider Z"),
                                encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "does not match its recorded digest"):
                PACKETS.discover_packets(repository)
            archived.write_bytes(prior_bytes)
            (archived.parent / "orphan.json").write_bytes(prior_bytes)
            with self.assertRaisesRegex(PACKETS.PacketError, "not reachable"):
                PACKETS.discover_packets(repository)

    def test_interrupted_transitions_recover_and_concurrent_transitions_serialize(self) -> None:
        class InjectedCrash(Exception):
            pass

        original_write = PACKETS.atomic_write

        def crash_on(call: int):
            calls = {"count": 0}

            def write(target: Path, content: bytes) -> None:
                calls["count"] += 1
                if calls["count"] == call:
                    raise InjectedCrash(target.name)
                original_write(target, content)
            return write

        def build(directory: str):
            repository = Path(directory) / "packets"
            shutil.copytree(FIXTURES, repository)

            def git(*args: str) -> str:
                return subprocess.run(["git", "-C", str(repository), *args],
                                      check=True, capture_output=True, text=True).stdout.strip()

            git("init", "-q")
            git("config", "user.name", "Fixture")
            git("config", "user.email", "fixture@example.invalid")
            git("add", ".")
            git("commit", "-qm", "fixture baseline")
            manifest_path = repository / "formed-beta" / "packet.json"
            packet = json.loads(manifest_path.read_text(encoding="utf-8"))
            common = {
                "proposal_id": packet["proposal_id"],
                "authority": {"decision_owner": packet["authority"]["decision_owner"],
                              "exercised_by": "fixture user",
                              "evidence": "Explicit fixture authority input."},
                "constraints": {"permanent_architecture_settled": False,
                                "roadmap_priority_changed": False,
                                "implementation_authorized": False},
            }
            first = common | {
                "schema_version": 1,
                "source": {"repository_revision": git("rev-parse", "HEAD"),
                           "lifecycle_state": packet["lifecycle_state"],
                           "placement_state": packet["placement"]["state"]},
                "decision": {"disposition": "approve_proposal_meaning", "lifecycle_state": "decided",
                             "placement_state": "probable",
                             "placement_claim": "Provider A probably hosts the projection.",
                             "rationale": "Meaning approved.",
                             "reopening_conditions": ["Provider A cannot host it."]},
            }
            inputs = Path(directory) / "inputs"
            inputs.mkdir()
            (inputs / "first.json").write_text(json.dumps(first), encoding="utf-8")
            return repository, git, manifest_path, common, inputs

        def revision_input(git, manifest_path: Path, common: dict, inputs: Path) -> Path:
            prior = (manifest_path.parent / "decision.json").read_bytes()
            revision = common | {
                "schema_version": 2,
                "source": {"repository_revision": git("rev-parse", "HEAD"),
                           "lifecycle_state": "decided", "placement_state": "probable"},
                "decision": {"disposition": "revise_placement", "lifecycle_state": "decided",
                             "placement_state": "probable",
                             "placement_claim": "Observer B probably supplies the observation.",
                             "rationale": "Later evidence withdrew the projection extension.",
                             "reopening_conditions": ["Observer B cannot bind a coherent snapshot."]},
                "supersedes": {"path": "decision-history/first.json",
                               "sha256": PACKETS.hashlib.sha256(prior).hexdigest()},
            }
            path = inputs / "revision.json"
            path.write_text(json.dumps(revision), encoding="utf-8")
            return path

        # Call 1 writes the journal; later calls write packet records. A crash
        # at any point leaves either the old state or a recoverable journal.
        for revise, calls in ((False, 3), (True, 4)):
            for call in range(1, calls + 1):
                with self.subTest(revise=revise, call=call), tempfile.TemporaryDirectory() as directory:
                    repository, git, manifest_path, common, inputs = build(directory)
                    decision_input = inputs / "first.json"
                    if revise:
                        PACKETS.transition_packet(manifest_path, decision_input, repository)
                        git("add", ".")
                        git("commit", "-qm", "first decision")
                        decision_input = revision_input(git, manifest_path, common, inputs)
                    before = {path: path.read_bytes() for path in manifest_path.parent.rglob("*.json")}
                    PACKETS.atomic_write = crash_on(call)
                    try:
                        with self.assertRaises(InjectedCrash):
                            PACKETS.transition_packet(manifest_path, decision_input, repository)
                    finally:
                        PACKETS.atomic_write = original_write

                    if call == 1:
                        self.assertEqual({path: path.read_bytes()
                                          for path in manifest_path.parent.rglob("*.json")}, before)
                        PACKETS.discover_packets(repository)
                        PACKETS.transition_packet(manifest_path, decision_input, repository)
                    else:
                        with self.assertRaisesRegex(PACKETS.PacketError, "interrupted decision transition"):
                            PACKETS.discover_packets(repository)
                        with self.assertRaisesRegex(PACKETS.PacketError, "run recover first"):
                            PACKETS.transition_packet(manifest_path, decision_input, repository)
                        PACKETS.recover_transition(manifest_path, repository)
                    current = PACKETS.discover_packets(repository)[common["proposal_id"]].manifest
                    self.assertEqual(current["lifecycle_state"], "decided")
                    if revise:
                        self.assertEqual(current["placement"]["claim"],
                                         "Observer B probably supplies the observation.")
                        self.assertEqual(
                            (manifest_path.parent / "decision-history" / "first.json").read_bytes(),
                            before[manifest_path.parent / "decision.json"])
                    with self.assertRaisesRegex(PACKETS.PacketError, "no interrupted transition"):
                        PACKETS.recover_transition(manifest_path, repository)

        with tempfile.TemporaryDirectory() as directory:
            repository, git, manifest_path, common, inputs = build(directory)
            # A record edited outside the tool after journaling is never overwritten.
            PACKETS.atomic_write = crash_on(2)
            try:
                with self.assertRaises(InjectedCrash):
                    PACKETS.transition_packet(manifest_path, inputs / "first.json", repository)
            finally:
                PACKETS.atomic_write = original_write
            (manifest_path.parent / "decision.json").write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(PACKETS.PacketError, "changed after the transition was journaled"):
                PACKETS.recover_transition(manifest_path, repository)
            (manifest_path.parent / "decision.json").unlink()
            PACKETS.recover_transition(manifest_path, repository)
            git("add", ".")
            git("commit", "-qm", "first decision")

            # A concurrent transition on the same packet is refused, not interleaved.
            revision = revision_input(git, manifest_path, common, inputs)
            lock_path, _ = PACKETS.transition_paths(PACKETS.transition_directory(repository),
                                                    manifest_path.parent)
            with PACKETS.transition_lock(lock_path):
                with self.assertRaisesRegex(PACKETS.PacketError, "holds this packet's lock"):
                    PACKETS.transition_packet(manifest_path, revision, repository)
            self.assertFalse((manifest_path.parent / "decision-history").exists())

            # History may not be redirected outside the packet through a link.
            outside = Path(directory) / "outside"
            outside.mkdir()
            (manifest_path.parent / "decision-history").symlink_to(outside)
            with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                PACKETS.transition_packet(manifest_path, revision, repository)
            self.assertEqual(list(outside.iterdir()), [])
            with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                PACKETS.discover_packets(repository)
            (manifest_path.parent / "decision-history").unlink()
            PACKETS.transition_packet(manifest_path, revision, repository)
            PACKETS.discover_packets(repository)

    def test_symlinked_or_external_manifests_cannot_redirect_transition_writes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "packets"
            shutil.copytree(FIXTURES, repository)
            # A valid packet outside the repository whose references point back inside it.
            outside = Path(directory) / "outside"
            outside.mkdir()
            packet = json.loads((FIXTURES / "formed-beta" / "packet.json").read_text(encoding="utf-8"))
            packet["narrative"]["path"] = "../packets/formed-beta/proposal.md"
            packet["origin_refs"][0]["path"] = "../packets/formed-beta/proposal.md"
            (outside / "packet.json").write_text(json.dumps(packet), encoding="utf-8")
            outside_bytes = (outside / "packet.json").read_bytes()
            (repository / "linked-file").mkdir()
            (repository / "linked-file" / "packet.json").symlink_to(outside / "packet.json")
            (repository / "linked-dir").symlink_to(outside)

            def git(*args: str) -> str:
                return subprocess.run(["git", "-C", str(repository), *args],
                                      check=True, capture_output=True, text=True).stdout.strip()

            git("init", "-q")
            git("config", "user.name", "Fixture")
            git("config", "user.email", "fixture@example.invalid")
            git("add", ".")
            git("commit", "-qm", "fixture baseline")
            decision = {
                "schema_version": 1,
                "proposal_id": packet["proposal_id"],
                "source": {"repository_revision": git("rev-parse", "HEAD"),
                           "lifecycle_state": packet["lifecycle_state"],
                           "placement_state": packet["placement"]["state"]},
                "decision": {"disposition": "approve_proposal_meaning", "lifecycle_state": "decided",
                             "placement_state": "probable",
                             "placement_claim": "Provider A probably hosts the projection.",
                             "rationale": "Meaning approved.",
                             "reopening_conditions": ["Provider A cannot host it."]},
                "authority": {"decision_owner": packet["authority"]["decision_owner"],
                              "exercised_by": "fixture user",
                              "evidence": "Explicit fixture authority input."},
                "constraints": {"permanent_architecture_settled": False,
                                "roadmap_priority_changed": False,
                                "implementation_authorized": False},
            }
            decision_input = Path(directory) / "decision-input.json"
            decision_input.write_text(json.dumps(decision), encoding="utf-8")

            for manifest in (repository / "linked-file" / "packet.json",
                             repository / "linked-dir" / "packet.json"):
                with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                    PACKETS.transition_packet(manifest, decision_input, repository)
                with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                    PACKETS.recover_transition(manifest, repository)
            with self.assertRaisesRegex(PACKETS.PacketError, "outside the packet repository"):
                PACKETS.transition_packet(outside / "packet.json", decision_input, repository)
            with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                PACKETS.discover_packets(repository)
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "transition",
                 str(repository / "linked-file" / "packet.json"), str(decision_input),
                 "--repository", str(repository)],
                capture_output=True, text=True,
            )
            self.assertEqual(completed.returncode, 2)
            self.assertEqual(json.loads(completed.stdout)["status"], "invalid")

            # A link inside the repository to another valid packet is refused too,
            # and so is a literal parent traversal that normalizes back inside.
            (repository / "aliased").mkdir()
            (repository / "aliased" / "packet.json").symlink_to(
                repository / "formed-beta" / "packet.json")
            beta_bytes = (repository / "formed-beta" / "packet.json").read_bytes()
            with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                PACKETS.transition_packet(repository / "aliased" / "packet.json",
                                          decision_input, repository)
            with self.assertRaisesRegex(PACKETS.PacketError, "must not be a symbolic link"):
                PACKETS.recover_transition(repository / "aliased" / "packet.json", repository)
            with self.assertRaisesRegex(PACKETS.PacketError, "must not traverse parent directories"):
                PACKETS.transition_packet(repository / "formed-alpha" / ".." / "formed-beta" / "packet.json",
                                          decision_input, repository)
            self.assertEqual((repository / "formed-beta" / "packet.json").read_bytes(), beta_bytes)
            self.assertFalse((repository / "formed-beta" / "decision.json").exists())

            # Nothing was written outside the repository.
            self.assertEqual(sorted(path.name for path in outside.iterdir()), ["packet.json"])
            self.assertEqual((outside / "packet.json").read_bytes(), outside_bytes)

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
