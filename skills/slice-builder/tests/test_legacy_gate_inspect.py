from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts/legacy_gate_inspect.py"
SPEC = importlib.util.spec_from_file_location("legacy_gate_inspect", SCRIPT)
assert SPEC and SPEC.loader
READER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(READER)
RUN_ID = "skills-migration-app-server-20260828"


class LegacyGateInspectTest(unittest.TestCase):
    @unittest.skipUnless(os.getenv("S5_REPAIR_GRANT"), "needs current private S5 repair grant")
    def test_v3_repair_grant_preserves_gate_selector_and_refuses_changed_descriptor(self):
        grant = json.loads(Path(os.environ["S5_REPAIR_GRANT"]).read_text())
        with tempfile.TemporaryDirectory() as directory:
            private = Path(directory) / "private"
            private.mkdir(mode=0o700)
            path = private / "grant.json"
            def write(value):
                path.write_text(json.dumps(value))
                path.chmod(0o600)
            write(grant)
            self.assertEqual(grant["gates"][2], READER.trusted_descriptor(path, 3))
            for mutate in [
                lambda x: x["repair_contract"].update(commit_oid="0" * 40),
                lambda x: x["repair_contract"].update(extra=True),
                lambda x: x.update(fields=["checkpoint"]),
            ]:
                changed = json.loads(json.dumps(grant))
                mutate(changed)
                write(changed)
                with self.assertRaisesRegex(ValueError, "does not admit"):
                    READER.trusted_descriptor(path, 3)
            write(grant)
            with path.open("a") as stream:
                stream.write('\\n')
            # A duplicate nested field must be refused at the grant parse boundary.
            raw = path.read_text().replace('"commit_oid":', '"commit_oid":"0","commit_oid":', 1)
            path.write_text(raw)
            with self.assertRaisesRegex(ValueError, "duplicate"):
                READER.trusted_descriptor(path, 3)

    def checkpoint(self, attempt: int, digest: str) -> dict:
        return {"ref": f"refs/work-engine/checkpoints/{RUN_ID}/slice-6/candidate-{attempt}",
                "commit_oid": str(attempt) * 40,
                "metadata": {"kind": "candidate", "run_id": RUN_ID, "slice_number": 6,
                             "candidate_attempt": attempt, "gate_receipt_digest": digest}}

    def fixture(self, directory: str, attempt: int):
        if attempt == 5:
            receipt = {"kind": "deterministic_gate_receipt", "run_id": RUN_ID,
                       "slice_number": 6, "candidate_attempt": 5,
                       "checks": [{"identity": "focused", "result": "passed"}]}
            row = {"type": "event_msg", "payload": {"type": "item_completed",
                   "item": {"id": "event-5", "stdout": json.dumps(receipt) + "\n"}}}
            output = {"type": "response_item", "payload": {"type": "custom_tool_call_output",
                      "call_id": "call-5", "output": [{"text": json.dumps(receipt)}]}}
            source_kind = "coordinator_constructed_receipt"
            identity = {"event_id": "event-5", "call_id": "call-5", "output_line": 2,
                        "output_sha256": hashlib.sha256((json.dumps(output) + "\n").encode()).hexdigest()}
        else:
            receipt = {"status": "passed", "checks": [{"identity": "focused", "state": "passed"}],
                       "failed_check": None, "totals": {"configured": 1, "executed": 1, "passed": 1, "failed": 0},
                       "duration_seconds": 0.1}
            row = {"type": "response_item", "payload": {"type": "custom_tool_call_output",
                   "call_id": "call-3", "output": [{"type": "input_text", "text": json.dumps({"result": receipt})}]}}
            source_kind = "slice_builder_runner_output"
            identity = {"call_id": "call-3"}
        path = Path(directory) / "capture.jsonl"
        raw = (json.dumps(row) + "\n").encode()
        path.write_bytes(raw + ((json.dumps(output) + "\n").encode() if attempt == 5 else b""))
        digest = hashlib.sha256(READER.canonical(receipt)).hexdigest()
        checkpoint = self.checkpoint(attempt, digest)
        descriptor = {"attempt": attempt, "checkpoint_ref": checkpoint["ref"],
                      "checkpoint_oid": checkpoint["commit_oid"], "capture_path": str(path),
                      "capture_line": 1, "capture_sha256": hashlib.sha256(raw).hexdigest(),
                      "source_kind": source_kind, "receipt_sha256": digest, **identity}
        return descriptor, checkpoint

    def test_both_actual_schemas_preserve_receipt_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            for attempt in (3, 5):
                descriptor, checkpoint = self.fixture(directory, attempt)
                result = READER.inspect(descriptor, attempt, checkpoint)
                self.assertEqual(1, result["passed_count"])
                self.assertEqual(descriptor["receipt_sha256"], result["receipt_sha256"])
                self.assertEqual("passed", result["checks"][0]["outcome"])

    def test_wrong_checkpoint_capture_and_native_identity_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            descriptor, checkpoint = self.fixture(directory, 3)
            other = self.checkpoint(4, checkpoint["metadata"]["gate_receipt_digest"])
            with self.assertRaisesRegex(ValueError, "paired"):
                READER.inspect(descriptor, 3, other)
            with self.assertRaisesRegex(ValueError, "capture bytes"):
                READER.inspect({**descriptor, "capture_sha256": "0" * 64}, 3, checkpoint)
            descriptor, checkpoint = self.fixture(directory, 5)
            with self.assertRaisesRegex(ValueError, "coordinator output bytes"):
                READER.inspect({**descriptor, "output_sha256": "0" * 64}, 5, checkpoint)
            row = json.loads(Path(descriptor["capture_path"]).read_text().splitlines()[0])
            receipt = json.loads(row["payload"]["item"]["stdout"])
            receipt["candidate_attempt"] = 4
            row["payload"]["item"]["stdout"] = json.dumps(receipt) + "\n"
            raw = (json.dumps(row) + "\n").encode()
            output = {"type": "response_item", "payload": {"type": "custom_tool_call_output",
                      "call_id": "call-5", "output": [{"text": json.dumps(receipt)}]}}
            output_raw = (json.dumps(output) + "\n").encode()
            Path(descriptor["capture_path"]).write_bytes(raw + output_raw)
            descriptor["capture_sha256"] = hashlib.sha256(raw).hexdigest()
            descriptor["output_sha256"] = hashlib.sha256(output_raw).hexdigest()
            with self.assertRaisesRegex(ValueError, "identity"):
                READER.inspect(descriptor, 5, checkpoint)

    def test_fifo_and_oversized_skipped_line_are_refused_before_read(self):
        with tempfile.TemporaryDirectory() as directory:
            pipe = Path(directory) / "capture.pipe"
            os.mkfifo(pipe)
            with self.assertRaisesRegex(ValueError, "regular file"):
                READER.selected_line(pipe, 1)
            capture = Path(directory) / "long.jsonl"
            capture.write_bytes(b"x" * (4 * 1024 * 1024) + b"\n{}\n")
            with self.assertRaisesRegex(ValueError, "read limit"):
                READER.selected_line(capture, 2)

    def test_v2_grant_only_adds_source_inventory_to_existing_gate_selector(self):
        with tempfile.TemporaryDirectory() as directory:
            descriptor, _ = self.fixture(directory, 3)
            private = Path(directory) / "private"
            private.mkdir(mode=0o700)
            path = private / "grant.json"
            grant = {"schema_version": 2, "profile": "historical-s5-local-inspection-v2",
                     "run_id": RUN_ID, "slice_number": 6, "peer_uid": os.geteuid(),
                     "expires_at": (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat(),
                     "gates": [{"attempt": i, **(descriptor if i == 3 else {})} for i in range(1, 6)],
                     "provider": {}, "scheduler": {}, "metrics": {}}
            path.write_text(json.dumps(grant))
            path.chmod(0o600)
            self.assertEqual(descriptor, READER.trusted_descriptor(path, 3))
            grant.pop("scheduler")
            path.write_text(json.dumps(grant))
            with self.assertRaisesRegex(ValueError, "does not admit"):
                READER.trusted_descriptor(path, 3)


if __name__ == "__main__":
    unittest.main()
