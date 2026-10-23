"""Real Rust reducer + Git CAS + filesystem-lock integration; synthetic actors only."""
from concurrent.futures import ThreadPoolExecutor, TimeoutError
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from uuid import uuid4

SCRIPTS = Path(__file__).parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location("experiment_adapter", SCRIPTS / "experiment_control.py")
ADAPTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ADAPTER)
ROOT = Path(__file__).resolve().parents[3]
BINARY = os.environ.get("EXPERIMENT_CONTROL_TEST_BINARY")
OWNER = dict(host="synthetic", session="owner", agent=None)
EVIDENCE = dict(reference="synthetic collected process/transcript", sha256="0" * 64)


@unittest.skipUnless(BINARY and sys.platform == "linux", "requires Linux and EXPERIMENT_CONTROL_TEST_BINARY")
class ExperimentControlTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid",
                        "commit", "-q", "--allow-empty", "-m", "fixture"], check=True)
        self.binary = Path(BINARY)
        self.sha = hashlib.sha256(self.binary.read_bytes()).hexdigest()
        self.specification = json.loads((ROOT / "rust/crates/experiment-control/examples/specification.json").read_text())
        self.spec = self.call(dict(op="register", specification=self.specification))["result"]["spec"]

    def wire(self, command, actor=None):
        return json.dumps(dict(id=str(uuid4()), actor=actor or OWNER,
                               observed_at="synthetic-time", command=command)).encode()

    def call(self, command, actor=None, expected=None, repository=None, emit=None):
        outputs = []
        ADAPTER.execute(repository or self.repo, "example", self.binary, self.sha,
                        self.wire(command, actor), expected, emit or outputs.append)
        return outputs[0] if outputs else None

    def store(self):
        return ADAPTER.GitRefDurableStateStore(self.repo, ADAPTER.NAMESPACE)

    def state(self):
        return json.loads(self.store().read("example").payload)

    def identity(self, handle):
        return dict(host="synthetic", session=handle, agent=None)

    def prepared(self, handle, branch=None, replaces=None):
        self.call(dict(op="prepare", handle=handle, spec=self.spec, phase="execution", profile="control",
                       unit="unit", branch=branch or handle, parent=None, replaces=replaces,
                       continuation=["handoff"] if replaces else []))
        output = self.call(dict(op="startup", handle=handle, receipt="startup-" + handle))["result"]
        self.call(dict(op="acknowledge", receipt=output["receipt"], evidence=EVIDENCE))
        ids = list(output["artifacts"])
        self.call(dict(op="bind", handle=handle, identity=self.identity(handle), exposure=dict(
            identity_evidence=EVIDENCE, observation_evidence=EVIDENCE,
            supplied_artifacts=ids, observed_artifacts=ids, prior_bindings=[], inherited_context=[])))
        self.call(dict(op="verify", handle=handle, evidence=EVIDENCE))

    def active(self, handle="c"):
        self.prepared(handle)
        self.call(dict(op="activate", handle=handle, expected_generation=0, settlement=None))

    def delivery(self, receipt="delivery"):
        return dict(op="deliver", handle="c", generation=1, artifacts=["baseline"], receipt=receipt)

    def test_exact_bytes_history_and_repository_state_are_preserved(self):
        head = subprocess.check_output(["git", "-C", str(self.repo), "rev-parse", "HEAD"])
        first = self.store().read("example").revision
        self.active()
        output = self.call(self.delivery(), self.identity("c"))["result"]
        self.assertEqual(base64.b64decode(output["artifacts"]["baseline"]["data_base64"]), b"Frozen baseline instructions.")
        subprocess.run(["git", "-C", str(self.repo), "gc", "--prune=now"], check=True, capture_output=True)
        self.assertIsNotNone(self.store().read_revision("example", first))
        self.assertEqual(subprocess.check_output(["git", "-C", str(self.repo), "rev-parse", "HEAD"]), head)
        self.assertEqual(subprocess.check_output(["git", "-C", str(self.repo), "status", "--porcelain"]), b"")
        self.assertIsNone(ADAPTER.GitRefDurableStateStore(self.repo, "codex-chatboard").read("board"))

    def test_denial_returns_no_withheld_bytes_and_publishes_nothing(self):
        self.active()
        before = self.store().read("example").revision
        outputs = []
        command = self.delivery()
        command["artifacts"] = ["treatment"]
        with self.assertRaisesRegex(ValueError, "operation refused"):
            self.call(command, self.identity("c"), emit=outputs.append)
        self.assertEqual(outputs, [])
        self.assertEqual(self.store().read("example").revision, before)

    def test_simulated_output_failure_after_publication_retains_uncertain_receipt(self):
        self.active()
        def lost(_):
            raise BrokenPipeError("synthetic lost output")
        with self.assertRaises(BrokenPipeError):
            self.call(self.delivery(), self.identity("c"), emit=lost)
        receipt = next(r for r in self.state()["receipts"].values() if r["local_id"] == "delivery")
        self.assertEqual(receipt["status"], "issued")
        with self.assertRaisesRegex(ValueError, "output unresolved"):
            self.call(dict(op="retire", handle="c", evidence=EVIDENCE))
        self.call(dict(op="dispose", receipt=receipt["id"], evidence=EVIDENCE,
                       disposition="emission unknown; producer collected; do not score as clean"))
        self.call(dict(op="retire", handle="c", evidence=EVIDENCE))

    def test_transition_waits_until_output_boundary_completes(self):
        self.active()
        entered, release = threading.Event(), threading.Event()
        def delayed(_):
            entered.set()
            if not release.wait(5):
                raise RuntimeError("test collector timed out")
        with ThreadPoolExecutor() as pool:
            output = pool.submit(self.call, self.delivery(), self.identity("c"), None, None, delayed)
            self.assertTrue(entered.wait(3))
            pause = pool.submit(self.call, dict(op="pause", handle="c", evidence=EVIDENCE))
            try:
                with self.assertRaises(TimeoutError):
                    pause.result(timeout=.1)
            finally:
                release.set()
            output.result(timeout=3)
            pause.result(timeout=3)
        with self.assertRaisesRegex(ValueError, "operation refused"):
            self.call(self.delivery("after-pause"), self.identity("c"))

    def test_competing_handoffs_have_one_winner_and_no_loser_output(self):
        self.active()
        data = b"frozen continuation"
        self.call(dict(op="import", spec=self.spec, artifact_id="handoff",
            artifact=dict(data_base64=base64.b64encode(data).decode(), sha256=hashlib.sha256(data).hexdigest(),
                          provenance="synthetic own run", resource="continuation-a"),
            evidence=EVIDENCE, producer="c", generation=1), self.identity("c"))
        self.prepared("a", "c", "c")
        self.prepared("b", "c", "c")
        revision = self.store().read("example").revision
        def activate(h):
            try:
                return self.call(dict(op="activate", handle=h, expected_generation=1, settlement=EVIDENCE), )
            except ValueError as error:
                return str(error)
        with ThreadPoolExecutor() as pool:
            results = list(pool.map(activate, ["a", "b"]))
        self.assertEqual(sum(isinstance(x, dict) for x in results), 1)
        self.assertTrue(any(isinstance(x, str) and "ownership conflict" in x for x in results))
        self.assertEqual(self.state()["branches"]["c"]["generation"], 2)

    def test_linked_worktree_uses_same_state_and_fencing(self):
        other = self.repo / "other"
        subprocess.run(["git", "-C", str(self.repo), "worktree", "add", "--detach", str(other)], check=True, capture_output=True)
        revision = self.store().read("example").revision
        result = self.call(dict(op="inspect", spec=self.spec), repository=other)
        self.assertNotEqual(result["revision"], revision)
        self.assertEqual(result["revision"], self.store().read("example").revision)
        self.active()
        with self.assertRaisesRegex(ValueError, "revision conflict"):
            self.call(dict(op="inspect", spec=self.spec), repository=other, expected=revision)

    def test_duplicate_request_keys_are_rejected_before_publication(self):
        before = self.store().read("example").revision
        request = b'{"id":"x","id":"y","actor":{},"observed_at":"now","command":{}}'
        with self.assertRaisesRegex(ValueError, "duplicate key"):
            ADAPTER.execute(self.repo, "example", self.binary, self.sha, request, None, lambda x: self.fail("unexpected output"))
        self.assertEqual(self.store().read("example").revision, before)

    def test_binary_mismatch_stale_revision_and_key_mismatch_do_not_emit(self):
        wire = self.wire(dict(op="inspect", spec=self.spec))
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            ADAPTER.execute(self.repo, "example", self.binary, "0" * 64, wire, None, lambda x: self.fail("output"))
        with self.assertRaisesRegex(ValueError, "revision conflict"):
            ADAPTER.execute(self.repo, "example", self.binary, self.sha, wire, "absent", lambda x: self.fail("output"))
        with self.assertRaisesRegex(ValueError, "incompatible reducer"):
            ADAPTER.execute(self.repo, "wrong", self.binary, self.sha,
                            self.wire(dict(op="register", specification=self.specification)), None, lambda x: self.fail("output"))
        self.assertIsNone(self.store().read("wrong"))

    def test_cli_failure_has_empty_stdout_and_success_is_compact_projection(self):
        self.active()
        cmd = [sys.executable, str(SCRIPTS / "experiment_control.py"), "--repository", str(self.repo),
               "--experiment", "example", "--binary", str(self.binary), "--sha256", self.sha]
        output = subprocess.run(cmd, input=self.wire(dict(op="status", handle="c"), self.identity("c")), capture_output=True)
        self.assertEqual(output.returncode, 0, output.stderr)
        projected = json.loads(output.stdout)["result"]
        self.assertNotIn("profile", projected)
        self.assertNotIn("arm", projected)
        command = self.delivery()
        command["artifacts"] = ["treatment"]
        output = subprocess.run(cmd, input=self.wire(command, self.identity("c")), capture_output=True)
        self.assertEqual(output.returncode, 1)
        self.assertEqual(output.stdout, b"")
        self.assertNotIn(b"Treatment-only", output.stderr)

    def test_snapshot_materializes_bytes_before_later_source_changes(self):
        source = self.repo / "instructions.md"
        source.write_bytes(b"captured instructions")
        output = subprocess.run([str(self.binary), "snapshot", str(source)], capture_output=True, check=True)
        snapshot = json.loads(output.stdout)
        spec = json.loads(json.dumps(self.specification))
        spec["artifacts"]["baseline"].update(snapshot)
        self.spec = self.call(dict(op="amend", from_spec=self.spec,
                                  specification=spec, evidence=EVIDENCE))["result"]["spec"]
        source.write_bytes(b"new global instructions must not reach this trial")
        self.active()
        delivered = self.call(self.delivery(), self.identity("c"))["result"]["artifacts"]["baseline"]
        self.assertEqual(base64.b64decode(delivered["data_base64"]), b"captured instructions")
        link = self.repo / "linked.md"
        link.symlink_to(source)
        output = subprocess.run([str(self.binary), "snapshot", str(link)], capture_output=True)
        self.assertEqual(output.returncode, 1)
        self.assertEqual(output.stdout, b"")


    def test_unread_stdout_times_out_releases_lock_and_preserves_receipt(self):
        self.active()
        data = b"x" * (256 * 1024)
        self.call(dict(op="import", spec=self.spec, artifact_id="large",
            artifact=dict(data_base64=base64.b64encode(data).decode(), sha256=hashlib.sha256(data).hexdigest(),
                          provenance="synthetic", resource="baseline"),
            evidence=EVIDENCE, producer=None, generation=None))
        command = self.delivery("blocked-output")
        command["artifacts"] = ["large"]
        request = self.repo / "request.json"
        request.write_bytes(self.wire(command, self.identity("c")))
        cmd = [sys.executable, str(SCRIPTS / "experiment_control.py"), "--repository", str(self.repo),
               "--experiment", "example", "--binary", str(self.binary), "--sha256", self.sha,
               "--request", str(request), "--output-timeout-seconds", "0.15"]
        with subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
            # Deliberately do not drain the pipe until the producer terminates.
            try:
                self.assertEqual(process.wait(timeout=5), 1)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                self.fail("unread stdout held the experiment lock indefinitely")
            stdout, stderr = process.communicate()
        self.assertIn(b"output deadline exceeded", stderr)
        self.assertGreater(len(stdout), 0)
        self.assertLess(len(stdout), len(data))
        receipt = next(r for r in self.state()["receipts"].values() if r["local_id"] == "blocked-output")
        self.assertEqual(receipt["status"], "issued")
        self.call(dict(op="pause", handle="c", evidence=EVIDENCE))
        self.assertTrue(self.state()["bindings"]["c"]["paused"])

    def test_lock_acquisition_has_a_deadline(self):
        self.active()
        entered, release = threading.Event(), threading.Event()
        def delayed(_):
            entered.set()
            if not release.wait(5):
                raise RuntimeError("test collector timed out")
        with ThreadPoolExecutor() as pool:
            output = pool.submit(self.call, self.delivery(), self.identity("c"), None, None, delayed)
            self.assertTrue(entered.wait(3))
            try:
                with self.assertRaisesRegex(TimeoutError, "lock deadline"):
                    ADAPTER.execute(self.repo, "example", self.binary, self.sha,
                        self.wire(dict(op="pause", handle="c", evidence=EVIDENCE)),
                        None, lambda x: self.fail("unexpected output"), lock_timeout=.05)
            finally:
                release.set()
            output.result(timeout=3)

    def test_raw_request_cannot_splice_adapter_state(self):
        before = self.store().read("example").revision
        wire = self.wire(dict(op="inspect", spec=self.spec)) + b',"state":null'
        with self.assertRaisesRegex(ValueError, "duplicate key"):
            ADAPTER.execute(self.repo, "example", self.binary, self.sha,
                            wire, None, lambda x: self.fail("unexpected output"))
        self.assertEqual(self.store().read("example").revision, before)


if __name__ == "__main__":
    unittest.main()
