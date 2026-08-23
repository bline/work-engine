from __future__ import annotations

import importlib.util
import base64
import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
spec = importlib.util.spec_from_file_location("durable_state_subject", ROOT / "scripts/durable_state.py")
assert spec and spec.loader
STORE = importlib.util.module_from_spec(spec)
spec.loader.exec_module(STORE)


class DurableStateTest(unittest.TestCase):
    def test_atomic_read_integrity_and_stale_revision(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STORE.GitRefDurableStateStore(repository)
            first = store.publish("campaign/slice", b"one", None)
            self.assertEqual(b"one", store.read("campaign/slice").payload)
            second = store.publish("campaign/slice", b"two", first.revision)
            with self.assertRaisesRegex(ValueError, "revision conflict"):
                store.publish("campaign/slice", b"stale", first.revision)
            self.assertEqual(second, store.read("campaign/slice"))
            self.assertEqual(first, store.read_revision("campaign/slice", first.revision))
            page = store.list_revisions("campaign/slice", None, 1)
            self.assertEqual((second,), page.values)
            self.assertEqual(second.revision, page.next_cursor)
            final_page = store.list_revisions("campaign/slice", page.next_cursor, 1)
            self.assertEqual((first,), final_page.values)
            self.assertIsNone(final_page.next_cursor)
            subprocess.run(["git", "-C", str(repository), "gc", "--prune=now"], check=True)
            self.assertEqual(first, store.read_revision("campaign/slice", first.revision))
            self.assertEqual((second.revision, first.revision), tuple(
                value.revision for value in store.list_revisions(
                    "campaign/slice", None, 10).values))
            with self.assertRaisesRegex(ValueError, "not reachable"):
                store.list_revisions("campaign/slice", "0" * 40, 1)
            original_git = store._git

            def fail_transaction(args, input_bytes=None):
                if args == ["update-ref", "--stdin"]:
                    raise STORE.DurableStateError("simulated disk failure")
                return original_git(args, input_bytes)

            store._git = fail_transaction
            with self.assertRaisesRegex(ValueError, "simulated disk failure"):
                store.publish("campaign/slice", b"three", second.revision)
            store._git = original_git
            status = subprocess.run(["git", "-C", str(repository), "status", "--short"],
                                    text=True, stdout=subprocess.PIPE, check=True).stdout
            self.assertEqual("", status)
            with self.assertRaises(ValueError):
                STORE.GitRefDurableStateStore(repository, namespace="bad..namespace")

    def test_legacy_value_is_readable_and_is_an_explicit_history_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STORE.GitRefDurableStateStore(repository)
            key = "legacy/key"
            payload = b"legacy"
            envelope = STORE.canonical({
                "schema_version": 1,
                "key": key,
                "payload_base64": base64.b64encode(payload).decode(),
                "payload_sha256": hashlib.sha256(payload).hexdigest(),
            })
            revision = subprocess.run(
                ["git", "-C", str(repository), "hash-object", "-w", "--stdin"],
                input=envelope, stdout=subprocess.PIPE, check=True).stdout.decode().strip()
            subprocess.run(["git", "-C", str(repository), "update-ref",
                            store._ref(key), revision], check=True)

            legacy = store.read(key)
            self.assertEqual(payload, legacy.payload)
            self.assertIsNone(legacy.predecessor_revision)
            current = store.publish(key, b"current", revision)
            history = store.list_revisions(key, None, 10)
            self.assertEqual((current.revision, revision),
                             tuple(value.revision for value in history.values))
            self.assertIsNone(history.next_cursor)

    def test_revision_reads_validate_key_integrity_and_missing_predecessors(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STORE.GitRefDurableStateStore(repository)
            other = store.publish("other/key", b"other", None)
            with self.assertRaisesRegex(ValueError, "key or integrity"):
                store.read_revision("wanted/key", other.revision)

            key = "broken/key"
            payload = b"broken"
            missing = "0" * len(other.revision)
            envelope = STORE.canonical({
                "schema_version": 2,
                "key": key,
                "payload_base64": base64.b64encode(payload).decode(),
                "payload_sha256": hashlib.sha256(payload).hexdigest(),
                "predecessor_revision": missing,
            })
            revision = subprocess.run(
                ["git", "-C", str(repository), "hash-object", "-w", "--stdin"],
                input=envelope, stdout=subprocess.PIPE, check=True).stdout.decode().strip()
            subprocess.run(["git", "-C", str(repository), "update-ref",
                            store._ref(key), revision], check=True)
            with self.assertRaisesRegex(ValueError, "unavailable"):
                store.list_revisions(key, None, 10)

            for bad_limit in (0, 101, True):
                with self.assertRaisesRegex(ValueError, "history limit"):
                    store.list_revisions(key, None, bad_limit)


if __name__ == "__main__":
    unittest.main()
