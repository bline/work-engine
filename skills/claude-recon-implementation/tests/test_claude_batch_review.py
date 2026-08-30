from __future__ import annotations

import importlib.util
import argparse
from datetime import datetime, timezone
import io
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "scripts" / "claude_batch_review.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("claude_batch_review", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ClaudeBatchReviewTest(unittest.TestCase):
    def test_event_summary_requires_full_request_to_terminal_lineage(self) -> None:
        complete = MODULE.summarize_events([
            {"event": "proxy_started"},
            {"event": "submitted", "batch_id": "b1"},
            {"event": "completed", "batch_id": "b1", "usage": {
                "prompt_tokens": 10, "completion_tokens": 2,
                "total_tokens": 12, "cost": 0.01,
            }},
            {"event": "turn_completed", "successful": True},
        ])
        self.assertTrue(complete["terminal_lineage_complete"])
        self.assertEqual(complete["actual_openrouter_usage"]["cost"], 0.01)

        incomplete = MODULE.summarize_events([
            {"event": "proxy_started"},
            {"event": "submitted", "batch_id": "b1"},
            {"event": "turn_completed", "successful": True},
        ])
        self.assertFalse(incomplete["terminal_lineage_complete"])

    def test_local_worker_binds_environment_output_and_terminal_lineage(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            proxy = root / "fake_proxy.py"
            proxy.write_text(r'''#!/usr/bin/env python3
import json
from pathlib import Path
import signal
import sys
import time

event_log = Path(sys.argv[sys.argv.index("--event-log") + 1])
model = sys.argv[sys.argv.index("--batch-model") + 1]
events = [
    {"event": "proxy_started"},
    {"event": "submitted", "batch_id": "batch-1"},
    {"event": "completed", "batch_id": "batch-1", "usage": {
        "prompt_tokens": 10, "completion_tokens": 2,
        "total_tokens": 12, "cost": 0.01,
    }},
    {"event": "turn_completed", "successful": True},
]
event_log.write_text("".join(json.dumps(item) + "\n" for item in events))
print(json.dumps({
    "schema_version": 1, "kind": "claude-batch-loopback-ready",
    "base_url": "http://127.0.0.1:43210", "batch_model": model,
}), flush=True)

def stop(_signal, _frame):
    with event_log.open("a") as handle:
        handle.write(json.dumps({"event": "proxy_stopped"}) + "\n")
    raise SystemExit(0)

signal.signal(signal.SIGINT, stop)
while True:
    time.sleep(1)
''', encoding="utf-8")
            claude = root / "fake_claude.py"
            claude.write_text(r'''#!/usr/bin/env python3
import json
import os
import sys
if "--version" in sys.argv:
    print("fake-claude 1.0")
else:
    print(json.dumps({
        "base_url": os.environ.get("ANTHROPIC_BASE_URL"),
        "openrouter_key_present": bool(os.environ.get("OPENROUTER_API_KEY")),
        "betas_disabled": os.environ.get("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"),
    }))
''', encoding="utf-8")
            claude.chmod(claude.stat().st_mode | stat.S_IXUSR)
            config = root / "config"
            config.mkdir()
            (config / "settings.json").write_text("{}", encoding="utf-8")
            model = "anthropic/claude-sonnet-5-20260630"
            attestation = root / "attestation.json"
            attestation.write_text(json.dumps({
                "schema_version": 1,
                "kind": "openrouter-key-guardrail",
                "observed_at": datetime.now(timezone.utc).isoformat(),
                "guardrail_id": "guardrail-1",
                "key_hash": "key-hash-1",
                "allowed_providers": ["anthropic"],
                "allowed_models": [model],
                "assignment_guardrail_id": "guardrail-1",
            }), encoding="utf-8")
            arguments = argparse.Namespace(
                receipt=root / "receipt.json",
                event_log=root / "events.jsonl",
                stdout=root / "stdout.json",
                stderr=root / "stderr.txt",
                claude_config_dir=config,
                working_directory=root,
                batch_model=model,
                routing_attestation=attestation,
                attestation_max_age_seconds=900,
                poll_interval_seconds=0.01,
                poll_timeout_seconds=2,
                collection_window_seconds=0,
                proxy_start_timeout_seconds=2,
                command=[str(claude), "-p", "prompt"],
            )
            prior_proxy = MODULE.PROXY_SCRIPT
            prior_environment = dict(os.environ)
            prior_stdout = MODULE.sys.stdout
            prior_stderr = MODULE.sys.stderr

            class Capture:
                def __init__(self):
                    self.buffer = io.BytesIO()

                def write(self, value):
                    self.buffer.write(value.encode("utf-8"))

                def flush(self):
                    return None

            MODULE.PROXY_SCRIPT = proxy
            MODULE.sys.stdout = Capture()
            MODULE.sys.stderr = Capture()
            os.environ["OPENROUTER_API_KEY"] = "secret"
            os.environ["OPENROUTER_API_KEY_HASH"] = "key-hash-1"
            try:
                returncode = MODULE.run(arguments)
            finally:
                MODULE.PROXY_SCRIPT = prior_proxy
                MODULE.sys.stdout = prior_stdout
                MODULE.sys.stderr = prior_stderr
                os.environ.clear()
                os.environ.update(prior_environment)
            self.assertEqual(returncode, 0)
            output = json.loads((root / "stdout.json").read_text())
            self.assertTrue(output["base_url"].startswith("http://127.0.0.1:"))
            self.assertFalse(output["openrouter_key_present"])
            self.assertEqual(output["betas_disabled"], "1")
            receipt = json.loads((root / "receipt.json").read_text())
            self.assertEqual(receipt["status"], "success")
            self.assertTrue(receipt["batch_summary"]["terminal_lineage_complete"])
            self.assertEqual(
                receipt["batch_summary"]["actual_openrouter_usage"]["cost"], 0.01
            )


if __name__ == "__main__":
    unittest.main()
