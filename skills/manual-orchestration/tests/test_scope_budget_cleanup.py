from __future__ import annotations

import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

from test_scope_budget import S, call

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/scope_budget.py'
SESSION = '11111111-1111-4111-8111-111111111111'


class CleanupTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)
        self.record = self.d / 'assignment.json'
        self.assertEqual(call('init', '--record', str(self.record), '--objective', 'x', '--item', 'a', '--pool', 'codex', '--max-usd', '1', '--set-by', 'user')[0], 0)
        self.board = self.d / 'skills/durable-state/scripts/codex_chatboard.py'
        self.board.parent.mkdir(parents=True)
        self.repo = self.d / 'repo'
        reader = self.repo / 'metrics/scripts/usage_ledger.py'
        reader.parent.mkdir(parents=True)
        reader.write_text('# stand-in\n')
        self.marker = self.d / 'child.pid'
        self.harness = self.d / 'harness.py'
        args = ['listen', '--record', str(self.record), '--session', SESSION, '--since', '0', '--repository', str(self.repo)]
        fake_file = self.d / 'skills/manual-orchestration/scripts/scope_budget.py'
        self.harness.write_text('import importlib.util\nfrom unittest.mock import patch\n'
            f'spec=importlib.util.spec_from_file_location("subject", {str(SCRIPT)!r})\n'
            's=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)\n'
            f'with patch.object(s,"__file__",{str(fake_file)!r}),patch.object(s,"save",side_effect=OSError("publication failure")):\n'
            f' raise SystemExit(s.main({args!r}))\n')
        self.procs = []
        self.libc = ctypes.CDLL(None)
        self.old_subreaper = ctypes.c_int()
        self.assertEqual(self.libc.prctl(37, ctypes.byref(self.old_subreaper), 0, 0, 0), 0)
        self.assertEqual(self.libc.prctl(36, 1, 0, 0, 0), 0)

    def tearDown(self):
        for proc in self.procs:
            if proc.poll() is None:
                proc.kill()
            proc.communicate(timeout=5)
        if self.marker.exists():
            pid = int(self.marker.read_text())
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                os.waitpid(pid, 0)
            except ChildProcessError:
                pass
        self.libc.prctl(36, self.old_subreaper.value, 0, 0, 0)
        self.tmp.cleanup()

    def launch(self, argv):
        p = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.procs.append(p)
        return p

    def wait_marker(self):
        deadline = time.monotonic() + 4
        while not self.marker.exists() and time.monotonic() < deadline:
            time.sleep(.01)
        self.assertTrue(self.marker.exists())
        return int(self.marker.read_text())

    def test_reader_survives_leader_but_is_stopped_after_save_failure(self):
        # The board exits after launching a slow reader: cleanup must signal the
        # group even though the leader is already reaped at publication time.
        reader = self.d / 'slow_reader.py'
        reader.write_text('import os,time\nfrom pathlib import Path\n'
                          f'Path({str(self.marker)!r}).write_text(str(os.getpid()))\n'
                          'time.sleep(30)\n')
        self.board.write_text('import subprocess,sys,time\n'
                              f'p=subprocess.Popen([sys.executable,{str(reader)!r}])\n'
                              f'from pathlib import Path\nwhile not Path({str(self.marker)!r}).exists(): time.sleep(.01)\n')
        owner = self.launch([sys.executable, str(self.harness)])
        pid = self.wait_marker()
        _, err = owner.communicate(timeout=9)
        self.assertEqual(owner.returncode, 2, err)
        waited, status = os.waitpid(pid, 0)
        self.assertEqual(waited, pid)
        self.assertTrue(os.WIFSIGNALED(status))
        self.marker.unlink()
        self.assertNotIn('listener', json.loads(self.record.read_text())['scope_budget'])

    def test_term_resistant_listener_is_killed_and_releases_lock(self):
        self.board.write_text('import os,signal,time\nfrom pathlib import Path\n'
                              'signal.signal(signal.SIGTERM,signal.SIG_IGN)\n'
                              f'Path({str(self.marker)!r}).write_text(str(os.getpid()))\n'
                              'time.sleep(30)\n')
        start = time.monotonic()
        owner = self.launch([sys.executable, str(self.harness)])
        self.wait_marker()
        close = self.launch([sys.executable, str(SCRIPT), 'close', '--record', str(self.record),
                             '--disposition', 'accepted', '--reason', 'done', '--set-by', 'user'])
        _, err = owner.communicate(timeout=9)
        self.assertEqual(owner.returncode, 2, err)
        self.assertLess(time.monotonic() - start, 8.5)
        _, err = close.communicate(timeout=3)
        self.assertEqual(close.returncode, 0, err)
        self.assertIn('closed_at', json.loads(self.record.read_text())['scope_budget'])
        self.marker.unlink()
