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
if state == "deep": print("[" * 2000 + "0" + "]" * 2000); sys.exit()
if state == "slow": time.sleep(3)
cursor = a[a.index("--cursor") + 1] if "--cursor" in a else None
topic = "control:budget:objective"
events = [] if cursor == state or state == "quiet" else [{"topic": topic, "authority": "observation_only", "kind": "mark_crossed"}]
doc = {"schema": "objective-budget-control-v1", "topic": topic, "events": events, "next_budget_cursor": state, "snapshot": {}}
if state == "closed":
    doc["closed_at"] = "2026-01-01T00:00:00Z"
    doc["events"] = [{"topic": topic, "authority": "observation_only", "kind": "objective_closed"}]
if state == "wrong-close": doc["closed_at"] = 7
print(json.dumps(doc))
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
        self.assertEqual((rows[-1]['next_since'], rows[-1]['next_control_cursor'], rows[-1]['next_budget_cursor']),
                         (1, 'context-cursor', 'second'))
        self.assertEqual(rows[-1]['control_events'], []); self.assertEqual(rows[-1]['budget_events'], [])
        self.assertEqual(sum(len(row['budget_events']) for row in rows), 1)

    def test_follow_ends_at_close_with_cursors(self):
        self.record.write_text('first'); self.post()
        cmd = self.command() + ['--session-id', SESSION, '--context-rollout', str(self.rollout),
            '--context-binary', str(self.context), '--context-sha256', hashlib.sha256(self.context.read_bytes()).hexdigest(),
            '--context-trigger-tokens', '100', '--context-band-floor-tokens', '200', '--follow']
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.addCleanup(lambda: proc.poll() is None and proc.kill())
        first = json.loads(proc.stdout.readline())
        self.assertNotIn('budget_closed_at', first)
        self.record.write_text('closed')
        output, error = proc.communicate(timeout=4)
        self.assertEqual(proc.returncode, 0, error)
        rows = [json.loads(line) for line in output.splitlines()]
        self.assertEqual([e['kind'] for e in rows[0]['budget_events']], ['objective_closed'])
        self.assertEqual(rows[0]['budget_closed_at'], '2026-01-01T00:00:00Z')
        self.assertEqual(rows[-1]['status'], 'budget_closed')
        self.assertEqual((rows[-1]['next_since'], rows[-1]['next_control_cursor'], rows[-1]['next_budget_cursor']),
                         (1, 'context-cursor', 'closed'))
        for key in ('messages', 'control_events', 'budget_events'): self.assertEqual(rows[-1][key], [])

    def test_wrong_close_type_and_legacy_reader(self):
        observer = self.budget()
        self.record.write_text('wrong-close'); self.post()
        result = board.wait_board(self.root, since=0, budget=observer, timeout_seconds=0)
        self.assertEqual(result['status'], 'messages')
        self.assertEqual(result['budget_events'][0]['kind'], 'reader_failed')
        self.assertNotIn('budget_closed_at', result)
        self.record.write_text('quiet'); time.sleep(.11)
        result = board.wait_board(self.root, since=1, budget=observer, timeout_seconds=0)
        self.assertEqual(result['budget_events'][0]['kind'], 'reader_recovered')
        self.assertNotIn('budget_closed_at', result)

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

    def test_deep_json_failure_keeps_board_and_cursor_then_recovers(self):
        observer = self.budget()
        self.record.write_text('first')
        board.wait_board(self.root, since=0, budget=observer, timeout_seconds=0)
        self.record.write_text('deep'); self.post(); time.sleep(.11)
        failed = board.wait_board(self.root, since=0, budget=observer, timeout_seconds=0)
        self.assertEqual(failed['status'], 'messages')
        self.assertEqual(failed['budget_events'][0]['kind'], 'reader_failed')
        self.assertEqual(failed['next_budget_cursor'], 'first')
        time.sleep(.11)
        self.assertEqual(observer.read(), [])
        self.record.write_text('first'); time.sleep(.11)
        recovered = board.wait_board(self.root, since=1, budget=observer, timeout_seconds=0)
        self.assertEqual([event['kind'] for event in recovered['budget_events']], ['reader_recovered'])
        self.assertEqual(recovered['next_budget_cursor'], 'first')

    def test_invalid_structured_documents_and_changed_topic(self):
        from unittest.mock import patch
        observer = self.budget()
        valid = {'schema': 'objective-budget-control-v1', 'topic': 'control:budget:objective',
                 'events': [], 'next_budget_cursor': 'cursor', 'snapshot': {}}
        good = subprocess.CompletedProcess([], 0, json.dumps(valid), '')
        with patch.object(board.subprocess, 'run', return_value=good):
            observer.read()
        variants = [dict(valid, schema='wrong'), dict(valid, topic='ordinary'),
                    dict(valid, topic='control:budget:other'),
                    dict(valid, events=[{'topic': valid['topic'], 'authority': 'approval'}]),
                    dict(valid, events=[{'topic': 'control:budget:other', 'authority': 'observation_only'}])]
        for value in variants:
            with self.subTest(value=value):
                observer.failed = False; observer.next_run = 0
                output = subprocess.CompletedProcess([], 0, json.dumps(value), '')
                with patch.object(board.subprocess, 'run', return_value=output):
                    events = observer.read()
                self.assertEqual(events[0]['kind'], 'reader_failed')
                self.assertEqual(observer.cursor, 'cursor')
                self.assertEqual(observer.topic, valid['topic'])

    def test_digest_after_success_and_argv_timeout_interval(self):
        from unittest.mock import patch
        observer = board.BudgetControl(record=self.record, reader=self.reader, sha256=self.digest,
            cursor='prior', interval=10, limit_fractions='0.5,1')
        document = {'schema': 'objective-budget-control-v1', 'topic': 'control:budget:objective',
                    'events': [], 'next_budget_cursor': 'next', 'snapshot': {}}
        response = subprocess.CompletedProcess([], 0, json.dumps(document), '')
        with patch.object(board.time, 'monotonic', side_effect=[100, 100, 101, 109.99, 110, 110]), \
             patch.object(board.subprocess, 'run', return_value=response) as run:
            observer.read(); observer.read(); observer.read(); observer.read()
        self.assertEqual(run.call_count, 2)
        argv = run.call_args_list[0].args[0]
        self.assertEqual(argv, ['python3', '-I', str(self.reader), 'control', '--record', str(self.record),
                                '--limit-fractions', '0.5,1', '--cursor', 'prior'])
        self.assertEqual(run.call_args_list[0].kwargs['timeout'], 120)
        self.assertEqual(run.call_args_list[1].args[0][-2:], ['--cursor', 'next'])
        self.reader.write_text('changed'); observer.next_run = 0
        with self.assertRaisesRegex(board.ChatboardError, 'digest changed'): observer.read()

    def test_resume_selector_and_context_status_precedence(self):
        self.record.write_text('first')
        result = subprocess.run(self.command() + ['--budget-cursor', 'first', '--timeout-seconds', '0'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2, result.stderr)
        resumed = json.loads(result.stdout)
        self.assertEqual(resumed['budget_events'], []); self.assertEqual(resumed['next_budget_cursor'], 'first')
        with self.assertRaisesRegex(board.ChatboardError, 'budget topics'):
            board.wait_board(self.root, since=0, topics=['control:budget:objective'], timeout_seconds=0)
        context = board.ContextControl(session_id=SESSION, rollout=self.rollout, binary=self.context,
            sha256=hashlib.sha256(self.context.read_bytes()).hexdigest(), trigger_tokens=100, band_floor_tokens=200)
        result = board.wait_board(self.root, since=0, control=context, budget=self.budget(), timeout_seconds=0)
        self.assertEqual(result['status'], 'control')
        self.assertEqual(len(result['control_events']), 1); self.assertEqual(len(result['budget_events']), 1)

    def test_cli_budget_without_session(self):
        self.record.write_text('event')
        result = subprocess.run(self.command() + ['--timeout-seconds', '0'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['status'], 'budget')

if __name__ == '__main__': unittest.main()
