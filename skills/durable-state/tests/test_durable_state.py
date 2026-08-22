from __future__ import annotations

import importlib.util
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
            status = subprocess.run(["git", "-C", str(repository), "status", "--short"],
                                    text=True, stdout=subprocess.PIPE, check=True).stdout
            self.assertEqual("", status)
            with self.assertRaises(ValueError):
                STORE.GitRefDurableStateStore(repository, namespace="bad..namespace")


if __name__ == "__main__":
    unittest.main()
