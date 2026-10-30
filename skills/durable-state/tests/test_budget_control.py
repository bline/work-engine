"""Budget listener integration with local synthetic readers."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from uuid import uuid4

SCRIPT = Path(__file__).parents[1] / 'scripts/codex_chatboard.py'
sys.path.insert(0, str(SCRIPT.parent))
spec = importlib.util.spec_from_file_location('budget_board', SCRIPT)
board = importlib.util.module_from_spec(spec)
spec.loader.exec_module(board)
SESSION = '11111111-1111-4111-8111-111111111111'

READER = '''import json, sys, time
from pathlib import Path
a = sys.argv
state = Path(a[a.index("--record") + 1]).read_text()
if state == "fail": sys.exit(1)
if state == "invalid": print("bad"); sys.exit()
if state == "slow": time.sleep(3)
cursor = a[a.index("--cursor") + 1] if "--cursor" in a else None
topic = "control:budget:objective"
events = [] if cursor == state or state == "quiet" else [{"topic": topic, "authority": "observation_only", "kind": "mark_crossed"}]
print(json.dumps({"schema": "objective-budget-control-v1", "topic": topic, "events": events, "next_budget_cursor": state, "snapshot": {}}))
'''
CONTEXT = '''#!/usr/bin/env python3
import json, sys
a = sys.argv
cursor = a[a.index("--cursor") + 1] if "--cursor" in a else None
topic = "control:session:" + a[a.index("--session") + 1]
event = None if cursor else {"topic": topic, "authority": "observation_only"}
print(json.dumps({"schema": "manual-context-control-v1", "topic": topic, "event": event, "next_control_cursor": "context-cursor"}))
'''

class BudgetTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        self.reader = self.root / 'reader.py'; self.reader.write_text(READER)
        self.digest = hashlib.sha256(self.reader.read_bytes()).hexdigest()
        self.record = self.root / 'record'; self.record.write_text('quiet')
        self.context = self.root / 'context'; self.context.write_text(CONTEXT); self.context.chmod(0o700)
        self.rollout = self.root / 'rollout'; self.rollout.write_text('')

    def budget(self, **kw):
        return board.BudgetControl(record=self.record, reader=self.reader, sha256=self.digest, interval=.1, **kw)

    def post(self, topic='ordinary'):
        return board.post(self.root, author='test', session_id=SESSION, topic=topic, body='test',
                          references=[], message_id=str(uuid4()), observed_at=board.timestamp())

    def command(self):
        return [sys.executable, str(SCRIPT), '--repository', str(self.root), 'wait', '--since', '0',
                '--budget-record', str(self.record), '--budget-reader', str(self.reader),
                '--budget-sha256', self.digest, '--budget-interval', '.1']

    def test_follow_all_sources_independent_cursors(self):
        self.record.write_text('first'); self.post()
        cmd = self.command() + ['--session-id', SESSION, '--context-rollout', str(self.rollout),
            '--context-binary', str(self.context), '--context-sha256', hashlib.sha256(self.context.read_bytes()).hexdigest(),
            '--context-trigger-tokens', '100', '--context-band-floor-tokens', '200', '--follow', '--timeout-seconds', '.7']
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.addCleanup(lambda: proc.poll() is None and proc.kill())
        first = json.loads(proc.stdout.readline())
        self.assertEqual(first['status'], 'messages')
        self.assertEqual((first['next_since'], first['next_control_cursor'], first['next_budget_cursor']), (1, 'context-cursor', 'first'))
        self.assertEqual(len(first['control_events']), 1); self.assertEqual(len(first['budget_events']), 1)
        self.record.write_text('second')
        output, error = proc.communicate(timeout=4)
        self.assertEqual(proc.returncode, 2, error)
        rows = [json.loads(line) for line in output.splitlines()]
        changed = next(row for row in rows if row['status'] == 'budget')
        self.assertEqual((changed['next_since'], changed['next_control_cursor'], changed['next_budget_cursor']), (1, 'context-cursor', 'second'))
        self.assertEqual(rows[-1]['next_budget_cursor'], 'second')

    def test_interval_and_unchanged(self):
        observer = self.budget()
        first = board.wait_board(self.root, since=0, budget=observer, timeout_seconds=.15)
        self.assertEqual(first['status'], 'timeout'); self.assertEqual(first['budget_events'], [])
        self.record.write_text('changed')
        start = time.monotonic()
        result = board.wait_board(self.root, since=0, budget=observer, timeout_seconds=1)
        self.assertEqual(result['status'], 'budget'); self.assertLess(time.monotonic()-start, .8)
        self.assertEqual(result['next_since'], 0)

    def test_failure_recovery_board_and_invalid_timeout(self):
        observer = self.budget(); self.record.write_text('fail')
        failed = board.wait_board(self.root, since=0, budget=observer, timeout_seconds=0)
        self.assertEqual(failed['budget_events'][0]['kind'], 'reader_failed'); self.assertIsNone(observer.cursor)
        self.post(); self.record.write_text('invalid'); time.sleep(.11)
        message = board.wait_board(self.root, since=0, budget=observer, timeout_seconds=0)
        self.assertEqual(message['status'], 'messages'); self.assertEqual(message['budget_events'], [])
        self.record.write_text('quiet'); time.sleep(.11)
        recovered = board.wait_board(self.root, since=1, budget=observer, timeout_seconds=0)
        self.assertEqual(recovered['budget_events'][0]['kind'], 'reader_recovered')
        self.assertEqual(observer.cursor, 'quiet')
        from unittest.mock import patch
        time.sleep(.11)
        with patch.object(board.subprocess, 'run', side_effect=subprocess.TimeoutExpired('reader', 120)):
            self.assertEqual(observer.read()[0]['kind'], 'reader_failed')
        self.assertEqual(observer.cursor, 'quiet')

    def test_digest_and_reserved(self):
        observer = self.budget(); self.reader.write_text('changed')
        with self.assertRaisesRegex(board.ChatboardError, 'digest changed'): observer.read()
        with self.assertRaisesRegex(board.ChatboardError, 'local telemetry'): self.post('control:budget:objective')
        self.post()
        value, revision = board.read_board(self.root)
        value['messages'][0]['topic'] = 'control:budget:objective'
        from unittest.mock import patch
        with patch.object(board, 'read_board', return_value=(value, revision)):
            result = board.wait_board(self.root, since=0, timeout_seconds=0)
        self.assertEqual(result['messages'], [])

    def test_cli_budget_without_session(self):
        self.record.write_text('event')
        result = subprocess.run(self.command() + ['--timeout-seconds', '0'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['status'], 'budget')

if __name__ == '__main__': unittest.main()
