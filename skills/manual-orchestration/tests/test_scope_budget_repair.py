from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from test_scope_budget import S, call

ROOT = Path(__file__).resolve().parents[3]
SESSION = '11111111-1111-4111-8111-111111111111'


class RepairTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)
        self.script = self.d / 'skills/manual-orchestration/scripts/scope_budget.py'
        self.script.parent.mkdir(parents=True)
        shutil.copy(ROOT / 'skills/manual-orchestration/scripts/scope_budget.py', self.script)
        self.board = self.d / 'skills/durable-state/scripts/codex_chatboard.py'
        self.board.parent.mkdir(parents=True)
        self.board.write_text('import pathlib,sys,time\np=pathlib.Path(sys.argv[sys.argv.index("--budget-record")+1])\np.with_suffix(".started").write_text("started")\ntime.sleep(30)\n')
        self.repo = self.d / 'repo'
        reader = self.repo / 'metrics/scripts/usage_ledger.py'
        reader.parent.mkdir(parents=True)
        reader.write_text('# reader\n')
        self.record = self.d / 'assignment.json'
        self.children = []
        self.init(self.record)

    def tearDown(self):
        for proc in self.children:
            if proc.poll() is None:
                proc.terminate()
            proc.wait(timeout=5)
        self.tmp.cleanup()

    def command(self, *args, record=None):
        return [sys.executable, str(self.script), *args, '--record', str(record or self.record)]

    def init(self, record):
        subprocess.run(self.command('init', '--objective', 'x', '--item', 'a', '--pool', 'codex', '--max-usd', '2', '--set-by', 'user', record=record), check=True, capture_output=True)

    def listen_cmd(self, record=None, *args):
        return self.command('listen', '--repository', str(self.repo), '--session', SESSION, *args, record=record)

    def start(self, *args):
        return subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def wait_marker(self, record=None):
        marker = (record or self.record).with_suffix('.started')
        for _ in range(300):
            if marker.exists():
                return
            time.sleep(.01)
        self.fail('stand-in did not start')

    def kill_listener(self, entry):
        try:
            os.kill(entry['pid'], signal.SIGTERM)
        except ProcessLookupError:
            pass
        for _ in range(300):
            if S.listener_state({'listener': entry}, Path(entry['argv'][entry['argv'].index('--budget-record') + 1]))['state'] == 'exited':
                return
            time.sleep(.01)
        self.fail('listener did not terminate')

    def test_close_waits_for_listener_publication(self):
        first = self.start(*self.listen_cmd(None, '--since', '0'))
        self.wait_marker()
        second = self.start(*self.command('close', '--disposition', 'accepted', '--reason', 'done', '--set-by', 'user'))
        out, err = first.communicate(timeout=8)
        self.assertEqual(first.returncode, 0, err)
        entry = json.loads(out)
        try:
            _, err = second.communicate(timeout=8)
            self.assertEqual(second.returncode, 0, err)
            sb = json.loads(self.record.read_text())['scope_budget']
            self.assertIn('closed_at', sb)
            self.assertEqual(sb['listener']['pid'], entry['pid'])
        finally:
            self.kill_listener(entry)

    def test_two_listens_serialize_and_one_is_refused(self):
        first = self.start(*self.listen_cmd(None, '--since', '0'))
        self.wait_marker()
        second = self.start(*self.listen_cmd(None, '--since', '0'))
        out, err = first.communicate(timeout=8)
        self.assertEqual(first.returncode, 0, err)
        entry = json.loads(out)
        try:
            _, err = second.communicate(timeout=8)
            self.assertEqual(second.returncode, 2, err)
            self.assertIn('already alive', err)
            self.assertEqual(json.loads(self.record.read_text())['scope_budget']['listener']['pid'], entry['pid'])
            self.assertEqual(S.listener_state({'listener': entry}, self.record)['state'], 'alive')
        finally:
            self.kill_listener(entry)

    def test_refusals_and_board_uuid_parser(self):
        for args in [('--session', 'bad'), ('--since', '-1'), ('--topic', 'control:budget:x')]:
            cmd = self.listen_cmd(None, '--since', '0') + list(args)
            p = subprocess.run(cmd, text=True, capture_output=True)
            self.assertEqual(p.returncode, 2, p.stderr)
            self.assertFalse(self.record.with_suffix('.started').exists())
        # The actual board wait parser accepts this exact canonical session and rejects a malformed one.
        subprocess.run(['git', 'init', '-q', str(self.repo)], check=True)
        board = ROOT / 'skills/durable-state/scripts/codex_chatboard.py'
        base = [sys.executable, str(board), '--repository', str(self.repo), 'wait', '--since', '0', '--timeout-seconds', '0', '--exclude-session-id']
        good = subprocess.run(base + [SESSION], text=True, capture_output=True, timeout=10)
        bad = subprocess.run(base + ['bad'], text=True, capture_output=True, timeout=10)
        self.assertNotIn('UUID', good.stderr)
        self.assertIn('UUID', bad.stderr)

    def test_immediate_exit_records_failed_attempt(self):
        self.board.write_text('import sys\nprint("bad startup", file=sys.stderr)\nsys.exit(7)\n')
        p = subprocess.run(self.listen_cmd(None, '--since', '0'), capture_output=True, text=True, timeout=8)
        self.assertEqual(p.returncode, 3, p.stderr)
        sb = json.loads(self.record.read_text())['scope_budget']
        self.assertNotIn('listener', sb)
        entry = sb['listener_history'][-1]
        self.assertEqual(entry['exit_code'], 7)
        self.assertIn(entry['stderr'], p.stderr)
        self.assertIn('bad startup', Path(entry['stderr']).read_text())

    def test_consecutive_objectives_use_separate_output(self):
        p = subprocess.run(self.listen_cmd(None, '--since', '1'), capture_output=True, text=True, timeout=8)
        self.assertEqual(p.returncode, 0, p.stderr)
        old = json.loads(p.stdout)
        self.kill_listener(old)
        Path(old['stdout']).write_text(json.dumps({'next_since': 88, 'next_budget_cursor': 'old'}) + '\n')
        p = subprocess.run(self.command('close', '--disposition', 'accepted', '--reason', 'done', '--set-by', 'user'), capture_output=True)
        self.assertEqual(p.returncode, 0)
        new = self.d / 'next.json'
        self.init(new)
        p = subprocess.run(self.listen_cmd(new), capture_output=True, text=True, timeout=8)
        self.assertEqual(p.returncode, 2)
        p = subprocess.run(self.listen_cmd(new, '--since', '2'), capture_output=True, text=True, timeout=8)
        self.assertEqual(p.returncode, 0, p.stderr)
        entry = json.loads(p.stdout)
        try:
            self.assertNotEqual(entry['stdout'], old['stdout'])
            self.assertNotEqual(entry['stderr'], old['stderr'])
            self.assertNotIn('--budget-cursor', entry['argv'])
            self.assertEqual(entry['argv'][entry['argv'].index('--since') + 1], '2')
        finally:
            self.kill_listener(entry)

    def test_save_failure_collects_started_child(self):
        proc_holder = []
        real = subprocess.Popen
        def start(*args, **kwargs):
            proc = real(*args, **kwargs)
            proc_holder.append(proc)
            return proc
        with patch.object(S, '__file__', str(self.script)), patch.object(S.subprocess, 'Popen', side_effect=start), patch.object(S, 'save', side_effect=OSError('failure')):
            rc, _ = call('listen', '--record', str(self.record), '--repository', str(self.repo), '--session', SESSION, '--since', '0')
        self.assertEqual(rc, 2)
        self.assertIsNotNone(proc_holder[0].poll())
        self.assertNotIn('listener', json.loads(self.record.read_text())['scope_budget'])
