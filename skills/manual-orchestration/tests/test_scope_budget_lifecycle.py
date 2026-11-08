from __future__ import annotations

import hashlib
import json
import os
import signal
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

from test_scope_budget import S, call

SESSION = '11111111-1111-4111-8111-111111111111'


class LifecycleTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)
        self.record = self.d / 'assignment.json'
        self.r = str(self.record)
        self.assertEqual(call('init', '--record', self.r, '--objective', 'x', '--item', 'a',
                              '--pool', 'codex', '--max-usd', '2', '--set-by', 'user')[0], 0)
        self.repo = self.d / 'repo'
        reader = self.repo / 'metrics/scripts/usage_ledger.py'
        reader.parent.mkdir(parents=True)
        reader.write_text('# reader\n')
        self.script = self.d / 'skills/manual-orchestration/scripts/scope_budget.py'
        self.board = self.d / 'skills/durable-state/scripts/codex_chatboard.py'
        self.board.parent.mkdir(parents=True)
        self.board.write_text('import json,sys,time\nfrom pathlib import Path\nPath(sys.argv[sys.argv.index("--budget-record")+1]).with_name("args.json").write_text(json.dumps(sys.argv[1:]))\ntime.sleep(30)\n')
        self.procs = []

    def tearDown(self):
        for pid in self.procs:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        self.tmp.cleanup()

    def listen(self, *args):
        with patch.object(S, '__file__', str(self.script)):
            return call('listen', '--record', self.r, '--session', SESSION, '--repository', str(self.repo), *args)

    def close(self, *args):
        return call('close', '--record', self.r, '--disposition', 'accepted', '--reason', 'done', '--set-by', 'user', *args)

    def test_close_and_refusals(self):
        self.assertEqual(self.close('--at', '2000-01-01T00:00:00Z')[0], 2)
        self.assertEqual(self.close('--at', '2999-01-01T00:00:00Z')[0], 2)
        rc, entry = self.close()
        self.assertEqual(rc, 0)
        sb = json.loads(self.record.read_text())['scope_budget']
        self.assertEqual(sb['closed_at'], entry['at'])
        self.assertEqual(sb['close'], entry)
        before = self.record.read_bytes()
        commands = [
            ('close', '--disposition', 'abandoned', '--reason', 'x', '--set-by', 'user'),
            ('add', '--item', 'x', '--reason', 'x'),
            ('revise', '--reconsider-after-additions', '3', '--set-by', 'user'),
            ('cost-limit', '--pool', 'codex', '--max-usd', '1', '--set-by', 'user', '--reason', 'x'),
            ('bind', '--session', 's', '--provider', 'codex', '--bound-by', 'user', '--reason', 'x'),
            ('add-session', '--session', 's', '--provider', 'codex', '--from', '2026-01-01T00:00:00Z', '--bound-by', 'rotation'),
            ('listen', '--session', 's', '--since', '0'),
        ]
        for cmd, *args in commands:
            rc, error = call(cmd, '--record', self.r, *args)
            self.assertEqual(rc, 2, cmd)
            self.assertIn('closed', error)
            self.assertIn('init', error)
            self.assertEqual(self.record.read_bytes(), before)
        rc, status = call('check', '--record', self.r, '--rollout', '/nonexistent')
        self.assertEqual(rc, 5)
        self.assertEqual(status, {'closed': True, 'closed_at': entry['at'], 'listener': {'state': 'none', 'pid': None}})

    def test_listener_command_resume_and_states(self):
        self.assertEqual(call('check', '--record', self.r)[1]['listener']['state'], 'none')
        self.assertEqual(self.listen()[0], 2)
        rc, entry = self.listen('--since', '7', '--topic', 'extra', '--interval', '12')
        self.assertEqual(rc, 0)
        for _ in range(200):
            if (self.d / 'args.json').exists():
                break
            time.sleep(.01)
        self.assertTrue((self.d / 'args.json').exists())
        reader = self.repo / 'metrics/scripts/usage_ledger.py'
        sha = hashlib.sha256(reader.read_bytes()).hexdigest()
        expected = [str(self.board), '--repository', str(self.repo), 'wait', '--since', '7', '--topic', f'session:{SESSION}', '--topic', 'extra', '--exclude-session-id', SESSION, '--budget-record', self.r, '--budget-reader', str(reader), '--budget-sha256', sha, '--budget-interval', '12.0', '--follow']
        self.assertEqual(entry['argv'], [sys.executable] + expected)
        self.assertEqual(json.loads((self.d / 'args.json').read_text()), expected[1:])
        self.assertEqual(entry['chatboard_sha256'], hashlib.sha256(self.board.read_bytes()).hexdigest())
        self.assertEqual(json.loads(self.record.read_text())['scope_budget']['listener'], entry)
        os.kill(entry['pid'], signal.SIGTERM)
        for _ in range(200):
            if call('check', '--record', self.r)[1]['listener']['state'] == 'exited':
                break
            time.sleep(.01)
        self.assertEqual(call('check', '--record', self.r)[1]['listener']['state'], 'exited')
        Path(entry['stdout']).write_text(json.dumps({'next_since': 42, 'next_budget_cursor': 'cursor'}) + '\n{"partial":')
        rc, resumed = self.listen('--since', '8')
        self.assertEqual(rc, 0)
        self.procs.append(resumed['pid'])
        self.assertEqual(resumed['argv'][resumed['argv'].index('--since') + 1], '42')
        self.assertEqual(resumed['argv'][resumed['argv'].index('--budget-cursor') + 1], 'cursor')
        self.assertEqual(json.loads(self.record.read_text())['scope_budget']['listener_history'], [entry])
        # A real stand-in process with the record in its argv establishes liveness.
        import subprocess
        proc = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)', self.r])
        self.procs.append(proc.pid)
        doc = json.loads(self.record.read_text())
        doc['scope_budget']['listener']['pid'] = proc.pid
        self.record.write_text(json.dumps(doc))
        self.assertEqual(call('check', '--record', self.r)[1]['listener']['state'], 'alive')
        before = self.record.read_bytes()
        self.assertEqual(self.listen('--since', '0')[0], 2)
        self.assertEqual(self.record.read_bytes(), before)
        self.close()
        self.assertEqual(call('check', '--record', self.r)[1]['listener']['state'], 'alive')
        proc.terminate()
        proc.wait()
        self.procs.remove(proc.pid)
        self.assertEqual(call('check', '--record', self.r)[1]['listener']['state'], 'exited')
