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



def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def build_repository(root: Path, count: int = 19, decided_every: int = 2) -> dict[str, Path]:
    """A repository with `count` packets under proposals/, references outside it,
    and unrelated malformed packet.json files elsewhere."""
    (root / "ideas").mkdir(parents=True)
    (root / "ideas" / "origin.md").write_text("# Origin idea\n", encoding="utf-8")
    # Unrelated artifacts that happen to be named packet.json.
    write_json(root / "pilot" / "runs" / "pass-01" / "packet.json", {"run": 1})
    (root / "skills" / "fixtures" / "packet.json").parent.mkdir(parents=True)
    (root / "skills" / "fixtures" / "packet.json").write_text("not json", encoding="utf-8")
    manifests: dict[str, Path] = {}
    for index in range(count):
        name = f"candidate-{index:02d}"
        packet_dir = root / "proposals" / f"family-{index % 3}" / name
        packet_dir.mkdir(parents=True)
        (packet_dir / "proposal.md").write_text(
            f"# {name}\n\n## Identity\n\n- Proposal ID: `work-engine.{name}`\n"
            "- Decision owner: user\n\nCanonical lifecycle metadata is in packet.json.\n",
            encoding="utf-8")
        decided = index % decided_every == 0
        manifest = {
            "schema_version": 1,
            "proposal_id": f"work-engine.{name}",
            "family_id": f"work-engine.family-{index % 3}",
            "title": name,
            "lifecycle_state": "decided" if decided else "formed",
            "narrative": {"path": "proposal.md"},
            # Packet-relative, outside proposals/, inside the repository.
            "origin_refs": [{"kind": "idea", "path": "../../../ideas/origin.md"}],
            "placement": {"state": "probable", "claim": f"{name} has a probable owner."},
            "uncertainty": {"state": "unresolved",
                            "reopening_conditions": [f"{name} loses its owner.",
                                                     f"{name} gains a second owner."]},
            "authority": {"decision_owner": "user", "implementation_authorized": False},
            "relationships": ([{"type": "related_to", "target_id": "work-engine.candidate-00",
                                "rationale": "Shares a family root.", "causal": False}]
                              if index else []),
        }
        write_json(packet_dir / "packet.json", manifest)
        if decided:
            write_json(packet_dir / "decision.json", {
                "schema_version": 1,
                "proposal_id": manifest["proposal_id"],
                "source": {"repository_revision": "0" * 40, "lifecycle_state": "formed",
                           "placement_state": "probable"},
                "decision": {"disposition": "approve_proposal_meaning", "lifecycle_state": "decided",
                             "placement_state": "probable",
                             "placement_claim": manifest["placement"]["claim"],
                             "rationale": "Meaning approved.",
                             "reopening_conditions": manifest["uncertainty"]["reopening_conditions"]},
                "authority": {"decision_owner": "user", "exercised_by": "user",
                              "evidence": "Fixture ruling."},
                "constraints": {"permanent_architecture_settled": False,
                                "roadmap_priority_changed": False,
                                "implementation_authorized": False},
            })
        manifests[manifest["proposal_id"]] = packet_dir / "packet.json"
    return manifests


class RepositoryValidationTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name) / "repository"
        self.manifests = build_repository(self.root)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def cli(self, *args: str, cwd: Path | None = None) -> tuple[int, dict, str]:
        completed = subprocess.run([sys.executable, str(SCRIPT), "validate", *args],
                                   cwd=cwd, capture_output=True, text=True)
        return completed.returncode, json.loads(completed.stdout), completed.stderr

    def failures(self) -> list[dict]:
        report = PACKETS.validate_repository(self.root, self.root / "proposals")
        return [error.report() for error in report.failures]

    def edit(self, proposal_id: str, mutate) -> Path:
        path = self.manifests[proposal_id]
        manifest = json.loads(path.read_text(encoding="utf-8"))
        mutate(manifest)
        write_json(path, manifest)
        return path

    def test_valid_nineteen_packet_repository_ignores_unrelated_packet_files(self) -> None:
        status, result, _ = self.cli("--repository", str(self.root), str(self.root / "proposals"))
        self.assertEqual((status, result["status"]), (0, "valid"))
        self.assertEqual(result["discovered_count"], 19)
        self.assertEqual(result["packet_count"], 19)
        self.assertEqual(result["decided_count"], 10)
        self.assertEqual(result["packet_root"], "proposals")
        # Without a separate packet root, the unrelated files block discovery,
        # which is the failure this boundary exists to prevent.
        status, result, _ = self.cli(str(self.root))
        self.assertEqual((status, result["status"]), (2, "invalid"))
        self.assertTrue(all(failure["category"] == "malformed_packet"
                            and not failure["file"].startswith("proposals/")
                            for failure in result["failures"]))

    def test_invocation_is_independent_of_working_directory(self) -> None:
        elsewhere = Path(self.directory.name) / "elsewhere"
        elsewhere.mkdir()
        results = [
            self.cli("--repository", str(self.root), str(self.root / "proposals"), cwd=elsewhere),
            self.cli("--repository", ".", "proposals", cwd=self.root),
            self.cli("--repository", ".", cwd=self.root),
            self.cli("--repository", "..", ".", cwd=self.root / "proposals"),
        ]
        for status, result, _ in results:
            self.assertEqual((status, result["status"], result["packet_count"]), (0, "valid", 19))
        self.assertEqual(len({json.dumps(result, sort_keys=True) for _, result, _ in results}), 1)

    def test_references_resolve_packet_relative_within_the_repository(self) -> None:
        # Repository-root-relative spelling is not reinterpreted: from the packet
        # directory, "ideas/origin.md" names a file that does not exist.
        target = "work-engine.candidate-03"
        self.edit(target, lambda value: value["origin_refs"][0].update(path="ideas/origin.md"))
        self.edit("work-engine.candidate-05",
                  lambda value: value["origin_refs"][0].update(path="../../../../outside.md"))
        self.edit("work-engine.candidate-07",
                  lambda value: value["narrative"].update(path="missing.md"))
        failures = {failure["proposal_id"] if "proposal_id" in failure else failure["packet"]: failure
                    for failure in self.failures()}
        self.assertEqual(len(failures), 3)
        for failure in failures.values():
            self.assertEqual(failure["category"], "missing_reference")
            self.assertTrue(failure["file"].endswith("packet.json"))
        self.assertEqual(failures["proposals/family-0/candidate-03/packet.json"]["observed"],
                         "ideas/origin.md")
        self.assertIn("escapes", failures["proposals/family-2/candidate-05/packet.json"]["invariant"])
        self.assertIn("narrative.path",
                      failures["proposals/family-1/candidate-07/packet.json"]["invariant"])

    def test_malformed_packets_are_reported_never_skipped(self) -> None:
        self.manifests["work-engine.candidate-04"].write_text("{ truncated", encoding="utf-8")
        self.edit("work-engine.candidate-06", lambda value: value.pop("placement"))
        self.edit("work-engine.candidate-08", lambda value: value.update(lifecycle_state="approved"))
        # Every other packet relates to candidate-00; breaking it must not
        # turn those relationships into misleading dangling-target failures.
        self.edit("work-engine.candidate-00", lambda value: value.pop("title"))
        failures = self.failures()
        self.assertEqual(sorted(failure["packet"] for failure in failures), [
            "proposals/family-0/candidate-00/packet.json",
            "proposals/family-0/candidate-06/packet.json",
            "proposals/family-1/candidate-04/packet.json",
            "proposals/family-2/candidate-08/packet.json",
        ])
        self.assertEqual({failure["category"] for failure in failures}, {"malformed_packet"})
        # Packets related to an invalid one are not misreported as dangling.
        self.assertFalse(any(failure["category"] == "relationship" for failure in failures))
        status, result, stderr = self.cli("--repository", str(self.root))
        self.assertEqual((status, result["failure_count"], result["discovered_count"]), (2, 4, 19))
        self.assertIn("proposals/family-1/candidate-04/packet.json: [malformed_packet]", stderr)

    def test_decision_binding_drift_names_expected_and_observed_values(self) -> None:
        dropped = "work-engine.candidate-02 gains a second owner.".replace(
            "work-engine.candidate-02", "candidate-02")
        self.edit("work-engine.candidate-02",
                  lambda value: value["uncertainty"]["reopening_conditions"].remove(dropped))
        [failure] = self.failures()
        self.assertEqual(failure["category"], "decision_binding")
        self.assertEqual(failure["file"], "proposals/family-2/candidate-02/packet.json")
        self.assertEqual(failure["missing_from_packet"], [dropped])
        self.assertEqual(failure["not_in_decision"], [])
        self.assertEqual(len(failure["expected"]), 2)
        self.assertEqual(len(failure["observed"]), 1)

    def test_hand_edited_decided_packet_cannot_disagree_with_its_decision(self) -> None:
        target = "work-engine.candidate-10"
        original = self.manifests[target].read_bytes()
        mutations = {
            "placement state": (lambda value: value["placement"].update(state="uncertain"),
                                "decision_binding"),
            "placement claim": (lambda value: value["placement"].update(claim="A different owner."),
                                "decision_binding"),
            "added condition": (lambda value: value["uncertainty"]["reopening_conditions"]
                                .append("A condition no authority approved."), "decision_binding"),
            "reordered conditions": (lambda value: value["uncertainty"]["reopening_conditions"]
                                     .reverse(), "decision_binding"),
            "lifecycle reverted": (lambda value: value.update(lifecycle_state="formed"),
                                   "decision_presence"),
            "authorized": (lambda value: value["authority"].update(implementation_authorized=True),
                           "malformed_packet"),
        }
        for name, (mutate, category) in mutations.items():
            with self.subTest(name):
                self.manifests[target].write_bytes(original)
                self.edit(target, mutate)
                failures = self.failures()
                self.assertEqual([failure["category"] for failure in failures], [category])
                self.assertEqual(failures[0]["packet"], "proposals/family-1/candidate-10/packet.json")
        self.manifests[target].write_bytes(original)
        self.assertEqual(self.failures(), [])

    def test_narrative_lifecycle_state_is_rejected_as_duplicate(self) -> None:
        narrative = self.manifests["work-engine.candidate-01"].parent / "proposal.md"
        narrative.write_text(narrative.read_text(encoding="utf-8").replace(
            "- Decision owner: user", "- State: formed; not evaluated\n- Decision owner: user"),
            encoding="utf-8")
        [failure] = self.failures()
        self.assertEqual(failure["category"], "duplicated_state")
        self.assertEqual(failure["file"], "proposals/family-1/candidate-01/proposal.md")
        self.assertEqual(failure["line"], 6)
        self.assertEqual(failure["observed"], "- State: formed; not evaluated")

    def test_every_failure_in_one_run_is_reported(self) -> None:
        self.manifests["work-engine.candidate-04"].write_text("[]", encoding="utf-8")
        self.edit("work-engine.candidate-02", lambda value: value["placement"].update(claim="Drifted."))
        self.edit("work-engine.candidate-09",
                  lambda value: value["relationships"][0].update(target_id="work-engine.missing"))
        (self.manifests["work-engine.candidate-11"].parent / "decision.json").write_text(
            "{}", encoding="utf-8")
        categories = sorted(failure["category"] for failure in self.failures())
        self.assertEqual(categories, ["decision_binding", "decision_presence",
                                      "malformed_packet", "relationship"])


if __name__ == "__main__":
    unittest.main()
