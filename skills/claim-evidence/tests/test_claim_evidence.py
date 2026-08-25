from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


ROOT = Path(__file__).parents[1]
SPEC = importlib.util.spec_from_file_location("claim_evidence", ROOT / "scripts" / "claim_evidence.py")
assert SPEC and SPEC.loader
CE = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(CE)
SHA = "a" * 64


def ref(name="evidence", status="verified"):
    return {"owner": "repository", "reference": name, "revision": "blob-1", "integrity_sha256": SHA, "freshness": "exact_revision", "status": status}


def authority(profile, actor, permissions=None):
    return {"schema_version": 1, "grant_id": f"grant:{profile}:{actor}", "actor": actor, "profile": profile, "permissions": permissions or sorted(CE.PERMISSIONS), "decision_scope": "formation", "authority_reference": ref("human-decision")}


def subject(namespace, stable):
    return {"namespace": namespace, "subject_kind": "artifact", "stable_subject_id": stable, "evidence_baseline": ref("baseline"), "content_set": ["proposal.md"]}


def revision(profile):
    payload = {"proposition": "bounded statement", "support_qualification": "supported", "assumptions": [], "limitations": ["bounded test"], "confidence": "high", "evidence_references": [ref()], "sensitivity_references": [ref("contract")], "evidence_mode": "direct_source", "judgment_kind": "domain_judgment", "decision_scope": "formation", "reopening_conditions": ["evidence changes"], "tombstone": False}
    payload["profile_payload"] = ({"materiality": "material", "support_qualification": "supported"} if profile == "proposal-research-v1" else {"finding_id": "F-1", "severity": "high", "episode": "episode-1", "outcome": "open"})
    return payload


def request(op, action, profile, payload, expected=None):
    return {"schema_version": 1, "operation_id": op, "action": action, "profile": profile, "expected_state": expected, "payload": payload}


class ClaimEvidenceTest(unittest.TestCase):
    @staticmethod
    def admit(store, auth):
        if not any(item["grant_id"] == auth["grant_id"] for item in store["authorities"]):
            store["authorities"].append(deepcopy(auth))

    def publish_claim(self, store, profile, namespace, stable, op):
        auth = authority(profile, f"producer-{stable}")
        self.admit(store, auth)
        outcome = CE.apply_operation(store, request(op, "create_claim", profile, {"subject": subject(namespace, stable), "statement_identity": "stable proposition", "initial_revision": revision(profile)}), auth)
        return outcome["result_identity"], auth

    def test_vertical_publication_discovery_and_exact_revision_reliance(self):
        store = CE.blank_store()
        research_revision, research_auth = self.publish_claim(store, "proposal-research-v1", "proposal-research", "candidate-a", "op-create-research")
        review_revision, review_auth = self.publish_claim(store, "revision-bound-review-finding-v1", "review", "finding-a", "op-create-review")
        reliance_payload = {"consumer": "proposal:consumer-a", "consumer_revision": "tree-abc", "decision_scope": "formation", "claim_revision_id": research_revision, "state": "active", "predecessor_reliance": None}
        reliance = CE.apply_operation(store, request("op-rely", "record_reliance", "proposal-research-v1", reliance_payload), research_auth)["result_identity"]
        projection = CE.build_projection(store)
        found = CE.discover(projection, {"namespace": "proposal-research", "profile": "proposal-research-v1"})
        self.assertEqual(len(found["candidates"]), 1)
        candidate = found["candidates"][0]
        self.assertEqual(candidate["revisions"][0]["id"], research_revision)
        self.assertEqual(candidate["revisions"][0]["limitations"], ["bounded test"])
        self.assertEqual(found["applicability"], "not_assessed")
        self.assertTrue(any(item["id"] == reliance and item["claim_revision_id"] == research_revision for item in projection["reliances"]))
        self.assertTrue(any(item["id"] == review_revision for item in projection["revisions"]))

    def test_idempotency_and_operation_collision(self):
        store = CE.blank_store(); auth = authority("proposal-research-v1", "producer")
        self.admit(store, auth)
        operation = request("same-op", "create_claim", "proposal-research-v1", {"subject": subject("research", "a"), "statement_identity": "statement", "initial_revision": revision("proposal-research-v1")})
        first = CE.apply_operation(store, operation, auth); second = CE.apply_operation(store, operation, auth)
        self.assertFalse(first["idempotent"]); self.assertTrue(second["idempotent"])
        changed = deepcopy(operation); changed["payload"]["statement_identity"] = "different"
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "operation identity conflict"): CE.apply_operation(store, changed, auth)
        changed_state = deepcopy(operation); changed_state["expected_state"] = "different"
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "operation identity conflict"): CE.apply_operation(store, changed_state, auth)

    def test_unauthorized_and_conflicting_predecessor_fail_closed(self):
        store = CE.blank_store(); profile = "proposal-research-v1"
        restricted = authority(profile, "producer", ["record_reliance"])
        self.admit(store, restricted)
        operation = request("create", "create_claim", profile, {"subject": subject("research", "a"), "statement_identity": "statement", "initial_revision": revision(profile)})
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "unauthorized"): CE.apply_operation(store, operation, restricted)
        head, auth = self.publish_claim(store, profile, "research", "a", "create-ok")
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "conflicting predecessor"):
            CE.apply_operation(store, request("next", "publish_revision", profile, {"claim_id": head.rsplit("@", 1)[0], "revision": revision(profile)}, "not-a-head"), auth)

    def test_unavailable_is_visible_and_does_not_mean_empty(self):
        store = CE.blank_store(); payload = revision("proposal-research-v1"); payload["evidence_references"] = [ref("offline", "unavailable")]
        auth = authority("proposal-research-v1", "producer")
        self.admit(store, auth)
        CE.apply_operation(store, request("create", "create_claim", "proposal-research-v1", {"subject": subject("research", "a"), "statement_identity": "statement", "initial_revision": payload}), auth)
        projection = CE.build_projection(store)
        self.assertEqual(projection["unresolved_references"], [{"owner": "repository", "reference": "offline", "revision": "blob-1", "status": "unavailable"}])
        projection["completeness"] = "unavailable"
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "completeness is unavailable"): CE.discover(projection, {"namespace": "research"})

    def test_closed_records_dangling_lineage_and_cycles_are_rejected(self):
        store = CE.blank_store(); head, auth = self.publish_claim(store, "proposal-research-v1", "research", "a", "create")
        changed = deepcopy(store); changed["claims"][0]["unknown"] = True
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "missing or unknown"): CE.validate_store(changed)
        dangling = deepcopy(store); dangling["lineage"].append({"id": "edge", "schema_version": 1, "relationship": "derivation", "sources": [head], "target": "missing", "authority_ref": auth["grant_id"], "operation_id": "edge-op"})
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "target is invalid"): CE.validate_store(dangling)
        cyclic = deepcopy(store); cyclic["lineage"].append({"id": "edge", "schema_version": 1, "relationship": "derivation", "sources": [head], "target": head, "authority_ref": auth["grant_id"], "operation_id": "edge-op"})
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "cyclic lineage"): CE.validate_store(cyclic)

    def test_scope_integrity_and_unsupported_versions_fail_closed(self):
        store = CE.blank_store(); auth = authority("proposal-research-v1", "producer")
        self.admit(store, auth)
        changed = request("create", "create_claim", "proposal-research-v1", {"subject": subject("research", "a"), "statement_identity": "statement", "initial_revision": revision("proposal-research-v1")})
        changed["payload"]["initial_revision"]["decision_scope"] = "different"
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "decision scope mismatch"): CE.apply_operation(store, changed, auth)
        unsupported = CE.blank_store(); unsupported["schema_version"] = 2
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "unsupported store version"): CE.validate_store(unsupported)
        invalid_hex = deepcopy(store); invalid_hex["authorities"][0]["authority_reference"]["integrity_sha256"] = "z" * 64
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "integrity is invalid"): CE.validate_store(invalid_hex)

    def test_confidence_rejects_json_values_that_transport_lossily(self):
        store = CE.blank_store(); auth = authority("proposal-research-v1", "producer")
        self.admit(store, auth)
        unsafe_revision = revision("proposal-research-v1")
        unsafe_revision["confidence"] = {"estimate": 2**53 + 1}
        operation = request("unsafe-confidence", "create_claim", "proposal-research-v1", {
            "subject": subject("research", "unsafe-confidence"),
            "statement_identity": "statement",
            "initial_revision": unsafe_revision,
        })
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "not lossless across JSON transports"):
            CE.apply_operation(store, operation, auth)

    def test_successor_lineage_and_reliance_retirement_preserve_history(self):
        store = CE.blank_store(); profile = "proposal-research-v1"
        first, auth = self.publish_claim(store, profile, "research", "a", "create")
        claim_id = first.rsplit("@", 1)[0]
        second = CE.apply_operation(store, request("revise", "publish_revision", profile, {"claim_id": claim_id, "revision": revision(profile)}, first), auth)["result_identity"]
        edge = CE.apply_operation(store, request("supersede", "publish_lineage", profile, {"relationship": "supersession", "sources": [first], "target": second}), auth)["result_identity"]
        active_payload = {"consumer": "proposal:a", "consumer_revision": "tree-1", "decision_scope": "formation", "claim_revision_id": first, "state": "active", "predecessor_reliance": None}
        active = CE.apply_operation(store, request("rely", "record_reliance", profile, active_payload), auth)["result_identity"]
        retired_payload = {**active_payload, "state": "retired", "predecessor_reliance": active}
        retired = CE.apply_operation(store, request("retire", "retire_reliance", profile, retired_payload, active), auth)["result_identity"]
        self.assertEqual(len(store["revisions"]), 2)
        self.assertTrue(any(item["id"] == edge for item in store["lineage"]))
        self.assertEqual([item["state"] for item in store["reliances"] if item["id"] in {active, retired}], ["active", "retired"])

    def test_representative_fixture_exercises_both_named_profiles(self):
        fixture = json.loads((ROOT / "tests" / "fixtures" / "representative-domain-records.json").read_text())
        self.assertEqual({item["profile"] for item in fixture["records"]}, CE.PROFILES)
        for item in fixture["records"]:
            CE.validate_profile_payload(item["profile"], item["profile_payload"])

    def test_publication_cannot_self_admit_authority(self):
        store = CE.blank_store(); auth = authority("proposal-research-v1", "self-declared")
        operation = request("create", "create_claim", "proposal-research-v1", {"subject": subject("research", "a"), "statement_identity": "statement", "initial_revision": revision("proposal-research-v1")})
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "trusted launcher"):
            CE.apply_operation(store, operation, auth)

    def test_structured_claim_identity_is_collision_safe(self):
        left = subject("a:b", "d"); left["subject_kind"] = "c"
        right = subject("a", "c:d"); right["subject_kind"] = "b"
        self.assertNotEqual(CE.stable_claim_id(left), CE.stable_claim_id(right))

    def test_cross_profile_lineage_is_rejected(self):
        store = CE.blank_store()
        research, auth = self.publish_claim(store, "proposal-research-v1", "research", "a", "research")
        review, _ = self.publish_claim(store, "revision-bound-review-finding-v1", "review", "b", "review")
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "resource profile mismatch"):
            CE.apply_operation(store, request("cross", "publish_lineage", "proposal-research-v1", {"relationship": "retraction", "sources": [review], "target": research}), auth)

    def test_projection_queries_preserve_provenance_and_detect_staleness(self):
        store = CE.blank_store(); first, auth = self.publish_claim(store, "proposal-research-v1", "research", "a", "create")
        claim_id = first.rsplit("@", 1)[0]
        second = CE.apply_operation(store, request("next", "publish_revision", "proposal-research-v1", {"claim_id": claim_id, "revision": revision("proposal-research-v1")}, first), auth)["result_identity"]
        CE.apply_operation(store, request("edge", "publish_lineage", "proposal-research-v1", {"relationship": "supersession", "sources": [first], "target": second}), auth)
        reliance_payload = {"consumer": "proposal:a", "consumer_revision": "tree-1", "decision_scope": "formation", "claim_revision_id": first, "state": "active", "predecessor_reliance": None}
        CE.apply_operation(store, request("rely", "record_reliance", "proposal-research-v1", reliance_payload), auth)
        projection = CE.build_projection(store)
        expected_context = CE.projection_context(projection)
        discovery = CE.discover(projection, {"evidence_baseline": "baseline", "content_reference": "proposal.md"})
        claim_resolution = CE.resolve_record(projection, claim_id)
        self.assertEqual(claim_resolution["kind"], "claim")
        self.assertEqual(claim_resolution["authority"]["grant_id"], auth["grant_id"])
        revision_resolution = CE.resolve_record(projection, first)
        traversal = CE.traverse(projection, first, "successors")
        direct_reliance = CE.query_reliance(projection, first, None)
        reverse_reliance = CE.query_reliance(projection, None, "proposal:a")
        self.assertEqual(revision_resolution["authority"]["grant_id"], auth["grant_id"])
        self.assertEqual(set(traversal["revision_ids"]), {first, second})
        self.assertEqual(len(direct_reliance["reliances"]), 1)
        self.assertEqual(len(reverse_reliance["reliances"]), 1)
        self.assertEqual(len(discovery["candidates"]), 1)
        for result in (discovery, claim_resolution, revision_resolution, traversal, direct_reliance, reverse_reliance):
            self.assertEqual({key: result[key] for key in expected_context}, expected_context)
        stale_store = deepcopy(store); stale_store["projection_boundary"]["source_watermark"] = "changed"
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "projection is stale"): CE.require_fresh_projection(stale_store, projection)

    def test_cli_reads_do_not_create_missing_root_or_lock(self):
        script = ROOT / "scripts" / "claim_evidence.py"
        with tempfile.TemporaryDirectory() as directory:
            missing_root = Path(directory) / "missing"
            result = subprocess.run(
                ["python3", str(script), "--root", str(missing_root), "discover", "--criteria-json", '{"namespace":"research"}'],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(missing_root.exists())

            initialized_root = Path(directory) / "initialized"
            subprocess.run(["python3", str(script), "--root", str(initialized_root), "init"], check=True, capture_output=True, text=True)
            lock_path = initialized_root / "canonical" / "store.lock"
            lock_path.unlink()
            before = sorted(path.relative_to(initialized_root) for path in initialized_root.rglob("*"))
            result = subprocess.run(
                ["python3", str(script), "--root", str(initialized_root), "verify"],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(sorted(path.relative_to(initialized_root) for path in initialized_root.rglob("*")), before)
            self.assertFalse(lock_path.exists())

    def test_cli_serializes_concurrent_publications_without_lost_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); auth = authority("proposal-research-v1", "concurrent-producer")
            authority_path = root / "authority.json"; authority_path.write_text(json.dumps(auth))
            script = ROOT / "scripts" / "claim_evidence.py"
            subprocess.run(["python3", str(script), "--root", str(root), "init", "--authority", str(authority_path)], check=True, capture_output=True, text=True)
            request_paths = []
            for suffix in ("a", "b"):
                operation = request(f"create-{suffix}", "create_claim", "proposal-research-v1", {"subject": subject("research", suffix), "statement_identity": f"statement-{suffix}", "initial_revision": revision("proposal-research-v1")})
                path = root / f"request-{suffix}.json"; path.write_text(json.dumps(operation)); request_paths.append(path)
            processes = [subprocess.Popen(["python3", str(script), "--root", str(root), "apply", "--request", str(path), "--authority", str(authority_path)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) for path in request_paths]
            results = [process.communicate(timeout=10) + (process.returncode,) for process in processes]
            self.assertEqual([item[2] for item in results], [0, 0], results)
            store = CE.load(root / "canonical" / "store.json"); CE.validate_store(store)
            self.assertEqual(len(store["claims"]), 2)
            self.assertEqual(len(store["operations"]), 2)
            projection = CE.load(root / "generated" / "projection.json"); CE.require_fresh_projection(store, projection)

    def test_only_typed_admitted_authority_can_satisfy_authority_reference(self):
        store = CE.blank_store(); bounded_subject = subject("research", "self-authorizing")
        claim_id = CE.stable_claim_id(bounded_subject)
        store["claims"].append({"id": claim_id, "schema_version": 1, "profile": "proposal-research-v1", "subject": bounded_subject, "statement_identity": "invalid self authority", "created_by": "producer", "authority_ref": claim_id})
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "profile or authority is invalid"):
            CE.validate_store(store)

    def test_claim_validation_binds_creator_and_create_permission_to_typed_authority(self):
        profile = "proposal-research-v1"
        admitted = authority(profile, "admitted-producer")
        store = CE.blank_store([admitted])
        bounded_subject = subject("research", "authority-binding")
        claim = {"id": CE.stable_claim_id(bounded_subject), "schema_version": 1, "profile": profile, "subject": bounded_subject, "statement_identity": "authority-bound claim", "created_by": admitted["actor"], "authority_ref": admitted["grant_id"]}
        store["claims"].append(claim)
        CE.validate_store(store)  # Positive control: matching actor with create_claim permission.

        mismatched_actor = deepcopy(store)
        mismatched_actor["claims"][0]["created_by"] = "different-producer"
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "creator does not match authority actor"):
            CE.validate_store(mismatched_actor)

        missing_permission = deepcopy(store)
        missing_permission["authorities"][0]["permissions"].remove("create_claim")
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "lacks create_claim permission"):
            CE.validate_store(missing_permission)

    def test_revision_and_receipt_validation_bind_actor_permission_and_result_authority(self):
        store = CE.blank_store()
        first, admitted = self.publish_claim(store, "proposal-research-v1", "research", "canonical-authority", "create")
        claim_id = first.rsplit("@", 1)[0]
        second = CE.apply_operation(store, request("revise", "publish_revision", "proposal-research-v1", {"claim_id": claim_id, "revision": revision("proposal-research-v1")}, first), admitted)["result_identity"]
        CE.validate_store(store)  # Positive control for actor, permission, and result binding.

        mismatched_actor = deepcopy(store)
        changed_revision = next(item for item in mismatched_actor["revisions"] if item["id"] == second)
        changed_revision["producer"] = "unauthorized-producer"
        changed_revision["id"] = f"{claim_id}@{CE.digest({key: value for key, value in changed_revision.items() if key != 'id'})}"
        next(item for item in mismatched_actor["operations"] if item["operation_id"] == "revise")["result_identity"] = changed_revision["id"]
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "producer does not match authority actor"):
            CE.validate_store(mismatched_actor)

        missing_permission = deepcopy(store)
        next(item for item in missing_permission["authorities"] if item["grant_id"] == admitted["grant_id"])["permissions"].remove("publish_revision")
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "lacks action permission"):
            CE.validate_store(missing_permission)

        relabeled_receipt = deepcopy(store)
        alternate = deepcopy(admitted); alternate["grant_id"] = "grant:alternate"
        relabeled_receipt["authorities"].append(alternate)
        next(item for item in relabeled_receipt["operations"] if item["operation_id"] == "revise")["authority_ref"] = alternate["grant_id"]
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "result authority is invalid"):
            CE.validate_store(relabeled_receipt)

    def test_lineage_validation_binds_authority_decision_scope(self):
        store = CE.blank_store()
        first, admitted = self.publish_claim(store, "proposal-research-v1", "research", "lineage-scope", "create")
        claim_id = first.rsplit("@", 1)[0]
        second = CE.apply_operation(store, request("revise", "publish_revision", "proposal-research-v1", {"claim_id": claim_id, "revision": revision("proposal-research-v1")}, first), admitted)["result_identity"]
        edge_id = CE.apply_operation(store, request("supersede", "publish_lineage", "proposal-research-v1", {"relationship": "supersession", "sources": [first], "target": second}), admitted)["result_identity"]
        CE.validate_store(store)  # Positive control for the admitted matching scope.

        wrong_scope = deepcopy(admitted)
        wrong_scope["grant_id"] = "grant:wrong-lineage-scope"
        wrong_scope["decision_scope"] = "different-scope"
        store["authorities"].append(wrong_scope)
        next(item for item in store["lineage"] if item["id"] == edge_id)["authority_ref"] = wrong_scope["grant_id"]
        next(item for item in store["operations"] if item["operation_id"] == "supersede")["authority_ref"] = wrong_scope["grant_id"]
        with self.assertRaisesRegex(CE.ClaimEvidenceError, "lineage authority scope is invalid"):
            CE.validate_store(store)

    def test_plain_revision_predecessor_is_traversable_without_lineage_edge(self):
        store = CE.blank_store(); first, auth = self.publish_claim(store, "proposal-research-v1", "research", "plain-successor", "create")
        claim_id = first.rsplit("@", 1)[0]
        second = CE.apply_operation(store, request("next", "publish_revision", "proposal-research-v1", {"claim_id": claim_id, "revision": revision("proposal-research-v1")}, first), auth)["result_identity"]
        projection = CE.build_projection(store)
        self.assertEqual(store["lineage"], [])
        backwards = CE.traverse(projection, second, "predecessors")
        self.assertEqual(set(backwards["revision_ids"]), {first, second})
        self.assertEqual(backwards["lineage"][0]["relationship"], "revision_predecessor")
        forwards = CE.traverse(projection, first, "successors")
        self.assertEqual(set(forwards["revision_ids"]), {first, second})


if __name__ == "__main__": unittest.main()
