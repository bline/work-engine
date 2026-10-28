"""Round-2 tooling tests. All run directories and evidence are synthetic.

Process barriers, injected failures and TASK_PROFILE_RECORD_TEST_SCRIPT are
test-only instrumentation; the production recorder has no test hook.
"""
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import stat
import sys
import tempfile
import time
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import task_profile_record as rec

# Test-only override for the isolated lock-removal mutation experiment.
SCRIPT = os.environ.get("TASK_PROFILE_RECORD_TEST_SCRIPT", str(Path(rec.__file__).resolve()))


class Round2Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.run = self.root / 'run'
        self.receipt = self.root / 'review.md'
        self.receipt.write_text('synthetic review\n')
        self.repair_calls = 0
        self.call('init-round', '--cap', '20', '--terra-in-flight', '2', '--terra-repair-budget', '3')

    def call(self, command, *args, code=0):
        out = io.StringIO()
        with mock.patch.object(sys, 'argv', [SCRIPT, command, '--run-dir', str(self.run), *map(str, args)]), contextlib.redirect_stdout(out):
            try:
                rec.main()
                actual = 0
            except SystemExit as e:
                actual = e.code
        self.assertEqual(actual, code, out.getvalue())
        # Round commands emit one JSON object; verify retains legacy text lines.
        if command == "verify":
            return {"text": out.getvalue()}
        result = json.loads(out.getvalue())
        self.assertIsInstance(result, dict)
        return result

    def enroll(self, task='a', byte=0, eligible='yes', outcome='enrolled'):
        with mock.patch.object(rec.os, 'urandom', return_value=bytes([byte])) as draw:
            out = self.call('enroll', '--task-id', task, '--task-revision', task + '@' + 'a' * 16,
                            '--eligible', eligible, '--reason', 'both builders acceptable')
        self.assertEqual(out['outcome'], outcome)
        if outcome == 'enrolled':
            draw.assert_called_once_with(1)
        else:
            draw.assert_not_called()
        return out

    def episode(self, task='a'):
        return self.run / 'enrollments' / task

    def log(self):
        return [json.loads(line) for line in (self.run / 'screening.jsonl').read_text().splitlines()]

    def review(self, task='a', outcome='pass', p1=0, p2=0, p3=0, code=0):
        return self.call('review', '--task-id', task, '--outcome', outcome, '--p1', p1, '--p2', p2, '--p3', p3,
                         '--receipt', self.receipt, code=code)

    def repair(self, task='a'):
        self.repair_calls += 1
        receipt = self.root / f'repair-review-{self.repair_calls}.md'
        receipt.write_text(f'synthetic review requiring repair {self.repair_calls}')
        return self.call('repair', '--task-id', task, '--review-receipt', receipt)

    def close(self, task='a', code=0, disposition='accepted'):
        return self.call('close', '--task-id', task, '--disposition', disposition, '--reason', 'test outcome',
                         '--pre-review-fixes', '2', '--escalations', '1', '--review-receipt', self.receipt,
                         '--artifact', self.receipt, code=code)

    def enroll_args(self, task='a'):
        return ['enroll', '--task-id', task, '--task-revision', task + '@' + 'a' * 16,
                '--eligible', 'yes', '--reason', 'synthetic barrier test']

    def profile_stub(self, task='a'):
        rev = task + '@' + 'a' * 16
        d = Path(rec.record_dir(str(self.run), rev))
        d.mkdir(parents=True, exist_ok=True)
        rec.write_once(d / 'profile-failed.json', {'stage': 'test'})
        rec.write_once(d / 'overhead.json', {'frozen_at': '2026-01-01T00:00:00Z'})
        packet = self.root / 'packet.md'
        packet.write_text('Synthetic assignment packet')
        return ['assign', '--task-revision', rev, '--role', 'builder', '--requested-model', 'synthetic',
                '--requested-effort', 'medium', '--harness', 'test', '--run', 'synthetic',
                '--dispatched-packet', str(packet), '--child-session', 'fake-session']

    def wait_marker(self, choices):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            for path in choices:
                if path.exists():
                    return path
            time.sleep(0.01)
        self.fail('barrier timed out: ' + ', '.join(map(str, choices)))

    def controlled_pair(self, first_args, second_args, ready='draw', codes=(0, 0)):
        # Test-only process bootstrap. No test hooks exist in the production recorder.
        # The first process holds the actual lock at a chosen publication boundary.
        # The second records EWOULDBLOCK from a nonblocking probe of that same lock.
        bootstrap = r'''import importlib.util, json, os, pathlib, sys, time
script, controls, label, ready = sys.argv[1:5]
args = sys.argv[5:]
spec = importlib.util.spec_from_file_location('rec', script)
rec = importlib.util.module_from_spec(spec); spec.loader.exec_module(rec)
controls = pathlib.Path(controls)
def mark(name): (controls / (label + '.' + name)).touch()
def gate():
    mark('ready')
    deadline = time.monotonic() + 15
    while not (controls / 'release').exists():
        if time.monotonic() > deadline: raise RuntimeError('release barrier timeout')
        time.sleep(.005)
flock = rec.fcntl.flock
def probe(fd, operation):
    if label == 'second':
        try: flock(fd, operation | rec.fcntl.LOCK_NB)
        except BlockingIOError:
            mark('blocked')
            flock(fd, operation)
    else: flock(fd, operation)
rec.fcntl.flock = probe
real_publish, real_write = rec.atomic_publish, rec.write_once
def publish(path, data):
    if label == 'first' and os.path.basename(path) == ready: gate()
    return real_publish(path, data)
def write(path, data):
    if label == 'first' and os.path.basename(path) == ready: gate()
    return real_write(path, data)
def draw(n):
    mark('draw')
    if label == 'first' and ready == 'draw': gate()
    return b'\x01'
rec.atomic_publish, rec.write_once, rec.os.urandom = publish, write, draw
sys.argv = [script] + args
try: rec.main()
finally: mark('done')
'''
        controls = self.root / 'barriers'
        controls.mkdir()
        processes = []
        outputs = []
        try:
            def launch(label, args):
                return subprocess.Popen([sys.executable, '-I', '-c', bootstrap, SCRIPT, str(controls), label, ready,
                                         args[0], '--run-dir', str(self.run), *args[1:]],
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            processes.append(launch('first', first_args))
            self.wait_marker([controls / 'first.ready', controls / 'first.done'])
            self.assertTrue((controls / 'first.ready').exists())
            processes.append(launch('second', second_args))
            reached = self.wait_marker([controls / 'second.blocked', controls / 'second.draw', controls / 'second.done'])
            self.assertEqual(reached.name, 'second.blocked', 'second command crossed the held critical section')
        finally:
            (controls / 'release').touch()
            for index, process in enumerate(processes):
                try:
                    stdout, stderr = process.communicate(timeout=20)
                except subprocess.TimeoutExpired:
                    process.kill()
                    stdout, stderr = process.communicate()
                    self.fail('barrier child hung: ' + stderr)
                self.assertEqual(process.returncode, codes[index], stderr + stdout)
                outputs.append(json.loads(stdout))
        return outputs, controls

    def test_controlled_same_task_enroll_serializes_draw(self):
        outputs, controls = self.controlled_pair(self.enroll_args(), self.enroll_args())
        self.assertEqual([o['outcome'] for o in outputs], ['enrolled', 'already_enrolled'])
        self.assertEqual(outputs[0]['arm'], outputs[1]['arm'])
        self.assertFalse((controls / 'second.draw').exists())
        self.assertFalse(rec.integrity_problems(str(self.episode())))

    def test_controlled_cap_enroll_serializes_admission(self):
        self.run = self.root / 'cap-one'
        self.call('init-round', '--cap', '1', '--terra-in-flight', '2', '--terra-repair-budget', '3')
        outputs, controls = self.controlled_pair(self.enroll_args('one'), self.enroll_args('two'))
        self.assertEqual([o['outcome'] for o in outputs], ['enrolled', 'cap_reached'])
        self.assertFalse((controls / 'second.draw').exists())
        self.assertEqual(self.call('status')['enrollments']['total'], 1)

    def test_controlled_withdraw_blocks_assignment_publication(self):
        self.enroll()
        assign = self.profile_stub()
        outputs, _ = self.controlled_pair(['withdraw', '--task-id', 'a', '--reason', 'withdraw first'], assign,
                                          ready='withdraw.json', codes=(0, 2))
        self.assertEqual(outputs[1]['outcome'], 'refused')
        self.assertFalse(list((self.run / 'task-profiles').glob('*/assignment*.json')))

    def test_controlled_assignment_blocks_withdrawal(self):
        self.enroll()
        assign = self.profile_stub()
        outputs, _ = self.controlled_pair(assign, ['withdraw', '--task-id', 'a', '--reason', 'assign first'],
                                          ready='assignment.json', codes=(0, 2))
        self.assertEqual(outputs[1]['outcome'], 'refused')
        self.assertFalse((self.episode() / 'withdraw.json').exists())

    def test_failed_p1_pause_is_derived_before_any_new_draw(self):
        self.enroll('t', byte=1)
        real_publish = rec.atomic_publish
        def fail_pause(path, data):
            if os.path.basename(path) == 'PAUSED':
                raise OSError('injected pause publication failure')
            return real_publish(path, data)
        with mock.patch.object(rec, 'atomic_publish', side_effect=fail_pause):
            self.review('t', outcome='fail', p1=1, code=1)
            self.assertTrue((self.episode('t') / 'review-first.json').exists())
            with mock.patch.object(rec.os, 'urandom') as draw:
                out = self.call(*self.enroll_args('blocked'), code=1)
                draw.assert_not_called()
                self.assertNotIn('arm', out)
        self.assertFalse((self.run / 'PAUSED').exists())
        self.enroll('blocked', outcome='paused')
        self.assertTrue((self.run / 'PAUSED').exists())
        history = self.call('resume', '--owner-approval', 'owner accepts known P1')['record']
        self.assertEqual(history['covered_triggers']['p1_episodes'], ['t'])
        self.enroll('t2', byte=1)  # The acknowledged old fact does not pause again.
        self.review('t2', outcome='fail', p1=1)
        self.assertTrue((self.run / 'PAUSED').exists())

    def test_failed_budget_pause_and_receipt_idempotence_across_resume(self):
        self.enroll(byte=1)
        self.repair()
        self.repair()
        real_publish = rec.atomic_publish
        def fail_pause(path, data):
            if os.path.basename(path) == 'PAUSED':
                raise OSError('pause write failed at budget')
            return real_publish(path, data)
        with mock.patch.object(rec, 'atomic_publish', side_effect=fail_pause):
            self.call('repair', '--task-id', 'a', '--review-receipt', self.receipt, code=1)
        self.assertEqual(len(list(self.episode().glob('repair-*.json'))), 3)
        self.enroll('blocked', outcome='paused')
        repeated = self.call('repair', '--task-id', 'a', '--review-receipt', self.receipt)
        self.assertEqual((repeated['outcome'], repeated['terra_repair_rounds']), ('already_repaired', 3))
        history = self.call('resume', '--owner-approval', 'owner approves recorded three rounds')['record']
        self.assertEqual(history['covered_triggers']['terra_repair_rounds'], 3)
        # Receipt identity is its hash, even when the path changes.
        same = self.root / 'copy.md'
        same.write_bytes(self.receipt.read_bytes())
        self.call('repair', '--task-id', 'a', '--review-receipt', same)
        self.assertFalse((self.run / 'PAUSED').exists())
        self.enroll('next')
        self.repair()
        self.assertTrue((self.run / 'PAUSED').exists())
        self.assertEqual(len(list(self.episode().glob('repair-*.json'))), 4)
        self.close()
        self.assertEqual(self.call('repair', '--task-id', 'a', '--review-receipt', same)['outcome'], 'already_repaired')

    def test_atomic_publication_failure_leaves_no_partial_final_record(self):
        def fail_link(source, target):
            data = rec.read_json(source)
            self.assertEqual(data['task_id'], 'a')
            self.assertFalse(Path(target).exists())
            raise OSError('injected failure before atomic link')
        with mock.patch.object(rec.os, 'link', side_effect=fail_link):
            self.call(*self.enroll_args(), code=1)
        self.assertFalse((self.episode() / 'enrollment.json').exists())
        self.enroll()
        before = (self.episode() / 'enrollment.json').read_bytes()
        for command in [('review', '--task-id', 'a', '--outcome', 'pass', '--p1', '0', '--p2', '0', '--p3', '0', '--receipt', self.receipt),
                        ('repair', '--task-id', 'a', '--review-receipt', self.receipt),
                        ('close', '--task-id', 'a', '--disposition', 'accepted', '--reason', 'test', '--pre-review-fixes', '0', '--escalations', '0'),
                        ('withdraw', '--task-id', 'a', '--reason', 'test')]:
            with mock.patch.object(rec.os, 'link', side_effect=fail_link):
                self.call(*command, code=1)
        self.assertEqual((self.episode() / 'enrollment.json').read_bytes(), before)
        self.assertEqual(rec.record_files(str(self.episode())), ['enrollment.json'])

    def test_atomic_publication_fsyncs_complete_file_before_link_then_directory(self):
        trace = []
        real_sync, real_link = rec.os.fsync, rec.os.link
        target = self.root / 'published.json'
        data = {'complete': True}
        def sync(fd):
            trace.append('directory' if stat.S_ISDIR(os.fstat(fd).st_mode) else 'file')
            return real_sync(fd)
        def link(source, dest):
            self.assertEqual(trace[-1], 'file')
            self.assertEqual(rec.read_json(source), data)
            trace.append('link')
            return real_link(source, dest)
        with mock.patch.object(rec.os, 'fsync', side_effect=sync), mock.patch.object(rec.os, 'link', side_effect=link):
            rec.atomic_publish(target, data)
        self.assertEqual(trace, ['file', 'link', 'directory'])
        self.assertFalse(list(self.root.glob('.record-*')))
        with self.assertRaises(FileExistsError):
            rec.atomic_publish(target, {'complete': 'different'})
        self.assertEqual(rec.read_json(target), data)

    def test_failure_after_link_keeps_complete_record_and_next_read_recovers(self):
        real_link = rec.os.link
        def link_then_fail(source, target):
            real_link(source, target)
            raise OSError('injected failure after link before checksum publication')
        with mock.patch.object(rec.os, 'link', side_effect=link_then_fail), mock.patch.object(rec.os, 'urandom', return_value=b'\x01'):
            self.call(*self.enroll_args(), code=1)
        self.assertEqual(rec.read_json(self.episode() / 'enrollment.json')['arm'], rec.TERRA)
        self.assertFalse((self.episode() / rec.SUMS).exists())
        out = self.call('status', '--task-id', 'a')
        self.assertEqual(out['episode']['enrollment.json']['arm'], rec.TERRA)
        self.assertFalse(rec.integrity_problems(str(self.episode())))
        self.enroll(outcome='already_enrolled')

    def test_abrupt_exit_at_temporary_creation_is_recoverable(self):
        bootstrap = r'''import importlib.util, os, sys
spec = importlib.util.spec_from_file_location('rec', sys.argv[1])
rec = importlib.util.module_from_spec(spec); spec.loader.exec_module(rec)
real = rec.tempfile.mkstemp
def die(*args, **kwargs):
    fd, path = real(*args, **kwargs)
    os._exit(74)
rec.tempfile.mkstemp = die
sys.argv = [sys.argv[1]] + sys.argv[2:]
rec.main()
'''
        r = subprocess.run([sys.executable, '-I', '-c', bootstrap, SCRIPT, 'enroll', '--run-dir', str(self.run),
                            *self.enroll_args()[1:]], capture_output=True, text=True, timeout=20)
        self.assertEqual(r.returncode, 74, r.stderr)
        self.assertFalse((self.episode() / 'enrollment.json').exists())
        self.enroll(byte=1)
        self.assertIn('verified', self.call('verify')['text'])

    def test_manifest_failure_recovers_before_replay_and_preserves_arm(self):
        with mock.patch.object(rec, 'append_sums', side_effect=OSError('injected manifest failure')):
            with mock.patch.object(rec.os, 'urandom', return_value=b'\x01'):
                self.call(*self.enroll_args(), code=1)
        original = (self.episode() / 'enrollment.json').read_bytes()
        self.assertFalse((self.episode() / rec.SUMS).exists())
        self.assertEqual(self.enroll(outcome='already_enrolled')['arm'], rec.TERRA)
        self.assertEqual((self.episode() / 'enrollment.json').read_bytes(), original)
        self.assertIn('enrollment.json', (self.run / 'recovered.jsonl').read_text())
        self.review()
        self.assertIn('verified', self.call('verify')['text'])

    def test_next_reader_recovers_manifest_for_each_episode_record(self):
        self.enroll()
        # Exercise resubmission, first review and terminal records, not only enrollment.
        actions = [lambda: self.call('repair', '--task-id', 'a', '--review-receipt', self.receipt, code=1),
                   lambda: self.review(code=1), lambda: self.close(code=1)]
        for action in actions:
            with mock.patch.object(rec, 'append_sums', side_effect=OSError('manifest append interrupted')):
                action()
            status = self.call('status')
            self.assertFalse(any('unlisted file' in p for p in status['inconsistencies']))
            self.assertFalse(rec.integrity_problems(str(self.episode())))
        self.enroll('w')
        with mock.patch.object(rec, 'append_sums', side_effect=OSError('manifest append interrupted')):
            self.call('withdraw', '--task-id', 'w', '--reason', 'test', code=1)
        self.call('status', '--task-id', 'w')
        self.assertFalse(rec.integrity_problems(str(self.episode('w'))))

    def test_corrupted_arm_and_other_episode_records_cannot_be_reused(self):
        self.enroll(byte=1)
        p = self.episode() / 'enrollment.json'
        original = p.read_bytes()
        data = rec.read_json(p)
        data['arm'] = rec.SOL
        p.write_text(json.dumps(data))
        with mock.patch.object(rec.os, 'urandom') as draw:
            for task in ('a', 'new'):
                out = self.call(*self.enroll_args(task), code=1)
                self.assertNotIn('arm', out)
                self.assertIn('hash mismatch', out['error'])
            draw.assert_not_called()
        p.write_bytes(original)
        self.repair()
        repair = self.episode() / 'repair-01.json'
        repair.write_text(repair.read_text().replace('resubmitted_at', 'edited_at'))
        with mock.patch.object(rec.os, 'urandom') as draw:
            self.call(*self.enroll_args('new'), code=1)
            draw.assert_not_called()

    def test_torn_screening_refuses_draw_then_preserves_and_trims_only_tail(self):
        self.enroll()
        path = self.run / 'screening.jsonl'
        good = path.read_bytes()
        damaged = good + b'{"event":"screen'
        path.write_bytes(damaged)
        with mock.patch.object(rec.os, 'urandom') as draw:
            self.call(*self.enroll_args('blocked'), code=1)
            draw.assert_not_called()
        self.assertEqual(path.read_bytes(), damaged)
        status = self.call('status')
        self.assertIn('screening.jsonl: malformed line 3', status['inconsistencies'])
        recovered = self.call('recover-screening')
        self.assertEqual(Path(recovered['backup']).read_bytes(), damaged)
        self.assertEqual(path.read_bytes(), good)
        self.enroll('blocked')
        self.assertEqual(self.call('recover-screening')['outcome'], 'no_recovery_needed')

    def test_valid_json_without_line_boundary_requires_recovery(self):
        path = self.run / 'screening.jsonl'
        original = b'{"event":"screened"}\n'
        torn = original + b'{"event":"screened"}'
        path.write_bytes(torn)
        with mock.patch.object(rec.os, 'urandom') as draw:
            self.call(*self.enroll_args(), code=1)
            draw.assert_not_called()
        out = self.call('recover-screening')
        self.assertEqual(Path(out['backup']).read_bytes(), torn)
        self.assertEqual(path.read_bytes(), original)

    def test_malformed_final_line_and_interior_lines_are_reported(self):
        path = self.run / 'screening.jsonl'
        # Keep malformed interior evidence; recovery is only authorized to remove the tail.
        interior = b'{broken interior}\n{"event":"screened"}\n'
        path.write_bytes(interior + b'{broken tail}\n')
        self.assertIn('malformed line 3', ' '.join(self.call('status')['inconsistencies']))
        self.call('recover-screening')
        self.assertEqual(path.read_bytes(), interior)
        self.assertIn('malformed line 1', ' '.join(self.call('status')['inconsistencies']))
        # An empty last line is a torn tail too, and must not delete its predecessor.
        path.write_bytes(interior + b'\n')
        self.call('recover-screening')
        self.assertEqual(path.read_bytes(), interior)

    def test_injected_torn_screening_append_blocks_retry(self):
        real = rec.append_event
        def torn(path, data):
            if str(path).endswith('screening.jsonl'):
                with open(path, 'ab') as fh:
                    fh.write(b'{"event":')
                    fh.flush()
                raise OSError('simulated torn append')
            return real(path, data)
        with mock.patch.object(rec, 'append_event', side_effect=torn), mock.patch.object(rec.os, 'urandom') as draw:
            self.call(*self.enroll_args(), code=1)
            draw.assert_not_called()
        with mock.patch.object(rec.os, 'urandom') as draw:
            self.call(*self.enroll_args(), code=1)
            draw.assert_not_called()
        self.call('recover-screening')
        self.enroll()

    def test_status_reports_withdrawal_plus_assignment(self):
        self.enroll()
        self.call('withdraw', '--task-id', 'a', '--reason', 'test')
        args = self.profile_stub()
        self.call(*args, code=2)
        d = Path(rec.record_dir(str(self.run), 'a@' + 'a' * 16))
        (d / 'assignment.json').write_text('{}')  # Reproduce an old, pre-remediation inconsistency.
        self.assertIn('a: both withdrawal and assignment', self.call('status')['inconsistencies'])

    def test_verify_retains_text_lines_for_profile_and_episode(self):
        self.enroll()
        self.profile_stub()
        d = Path(rec.record_dir(str(self.run), 'a@' + 'a' * 16))
        rec.append_sums(str(d))
        out = self.call('verify')['text']
        self.assertIn('enrollments/a: verified', out.splitlines())
        self.assertIn('a@' + 'a' * 16 + ': verified', out.splitlines())

    def test_both_arms_and_screening_precedes_draw(self):
        for task, byte, arm in [('terra', 255, rec.TERRA), ('sol', 254, rec.SOL)]:
            def draw(n):
                self.assertEqual(n, 1)
                self.assertEqual(self.log()[-1]['event'], 'screened')
                self.assertFalse(self.episode(task).exists())
                return bytes([byte])
            with mock.patch.object(rec.os, 'urandom', side_effect=draw):
                out = self.call('enroll', '--task-id', task, '--task-revision', task + '@' + 'b' * 16,
                                '--eligible', 'yes', '--reason', 'acceptable')
            self.assertEqual(out['arm'], arm)
            e = rec.read_json(self.episode(task) / 'enrollment.json')
            self.assertEqual((e['draw_byte'], e['eligible'], e['task_id']), (byte, True, task))
            self.assertEqual(e['recorder']['sha256'], rec.sha256(SCRIPT))
            self.assertEqual([r['event'] for r in self.log()[-2:]], ['screened', 'result'])
            self.assertEqual(self.log()[-1]['outcome'], 'enrolled')
            self.assertFalse(rec.integrity_problems(str(self.episode(task))))

    def test_reenroll_keeps_arm_even_if_now_ineligible_and_paused(self):
        original = self.enroll(byte=1)
        self.call('pause', '--reason', 'owner pause')
        out = self.enroll(eligible='no', outcome='already_enrolled')
        self.assertEqual(out['arm'], original['arm'])
        self.assertEqual(len(list(self.episode().glob('enrollment*.json'))), 1)

    def test_real_process_concurrent_enrollments_share_one_record(self):
        args = [sys.executable, '-I', SCRIPT, 'enroll', '--run-dir', str(self.run), '--task-id', 'race',
                '--task-revision', 'race@' + 'a' * 16, '--eligible', 'yes', '--reason', 'race test']
        processes = [subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) for _ in range(2)]
        outputs = []
        for process in processes:
            stdout, stderr = process.communicate(timeout=20)
            self.assertEqual(process.returncode, 0, stderr)
            outputs.append(json.loads(stdout))
        self.assertEqual(outputs[0]['arm'], outputs[1]['arm'])
        self.assertEqual({r['outcome'] for r in outputs}, {'enrolled', 'already_enrolled'})
        self.assertEqual(len(list(self.episode('race').glob('enrollment*.json'))), 1)
        self.assertFalse(rec.integrity_problems(str(self.episode('race'))))
        self.assertEqual([r['event'] for r in self.log()], ['screened', 'result', 'screened', 'result'])

    def test_concurrent_different_tasks_do_not_exceed_cap(self):
        self.run = self.root / 'cap-one'
        self.call('init-round', '--cap', '1', '--terra-in-flight', '2', '--terra-repair-budget', '3')
        processes = []
        for task in ('one', 'two'):
            args = [sys.executable, '-I', SCRIPT, 'enroll', '--run-dir', str(self.run), '--task-id', task,
                    '--task-revision', task + '@' + 'a' * 16, '--eligible', 'yes', '--reason', 'cap race']
            processes.append(subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True))
        outcomes = []
        for process in processes:
            stdout, stderr = process.communicate(timeout=20)
            self.assertEqual(process.returncode, 0, stderr)
            outcomes.append(json.loads(stdout)['outcome'])
        self.assertCountEqual(outcomes, ['enrolled', 'cap_reached'])
        self.assertEqual(self.call('status')['enrollments']['total'], 1)

    def test_atomic_publication_loser_reads_winners_arm(self):
        real_link = rec.os.link
        def winner(source, path):
            # The candidate is complete before publication, and a winner is never replaced.
            self.assertEqual(rec.read_json(source)['arm'], rec.SOL)
            data = {'task_id': 'a', 'arm': rec.TERRA, 'reason': 'other caller won'}
            with open(path, 'x') as fh:
                json.dump(data, fh)
            rec.append_sums(os.path.dirname(path))
            return real_link(source, path)  # Really raises FileExistsError.
        with mock.patch.object(rec.os, 'link', side_effect=winner), mock.patch.object(rec.os, 'urandom', return_value=b'\x00'):
            out = self.call('enroll', '--task-id', 'a', '--task-revision', 'a@' + 'a' * 16,
                            '--eligible', 'yes', '--reason', 'loser')
        self.assertEqual((out['outcome'], out['arm']), ('already_enrolled', rec.TERRA))
        self.assertEqual(rec.read_json(self.episode() / 'enrollment.json')['reason'], 'other caller won')

    def test_retry_after_record_written_before_result(self):
        real = rec.screening
        def fail_result(run_dir, data):
            if data['event'] == 'result':
                raise RuntimeError('simulated crash before result and print')
            real(run_dir, data)
        with mock.patch.object(rec, 'screening', side_effect=fail_result), self.assertRaises(RuntimeError):
            self.enroll(byte=1)
        self.assertTrue((self.episode() / 'enrollment.json').is_file())
        self.assertEqual(self.enroll(outcome='already_enrolled')['arm'], rec.TERRA)
        status = self.call('status')
        self.assertTrue(any('no enrolled result' in p for p in status['inconsistencies']))

    def test_not_eligible_draws_nothing_and_creates_no_episode(self):
        self.enroll(eligible='no', outcome='not_eligible')
        self.assertFalse(self.episode().exists())
        self.assertEqual(self.log()[-1]['outcome'], 'not_eligible')

    def test_paused_draws_nothing_and_is_logged(self):
        self.call('pause', '--reason', 'hold')
        self.enroll(outcome='paused')
        self.assertFalse(self.episode().exists())
        self.assertEqual(self.log()[-1]['outcome'], 'paused')

    def test_cap_draws_nothing_and_is_logged(self):
        for i in range(20):
            self.enroll('task' + str(i))
        self.enroll('excess', outcome='cap_reached')
        self.assertFalse(self.episode('excess').exists())
        self.assertEqual(self.log()[-1]['outcome'], 'cap_reached')

    def test_in_flight_blocks_both_arms_before_draw(self):
        self.enroll('t1', byte=1)
        self.enroll('t2', byte=1)
        for byte in (0, 1):
            self.enroll('blocked' + str(byte), byte=byte, outcome='in_flight_limit')
        self.assertEqual(self.log()[-1]['outcome'], 'in_flight_limit')
        self.call('withdraw', '--task-id', 't1', '--reason', 'not dispatched')
        self.enroll('available', byte=0)

    def test_budget_counts_open_and_closed_terra_at_exact_threshold(self):
        self.enroll('t1', byte=1)
        self.enroll('t2', byte=1)
        self.enroll('s', byte=0, outcome='in_flight_limit')
        self.repair('t1')
        self.close('t1')
        self.enroll('s')
        self.repair('s')  # Sol repairs do not consume the Terra budget.
        self.repair('t2')
        self.assertFalse((self.run / 'PAUSED').exists())
        self.assertEqual(self.repair('t2')['terra_repair_rounds'], 3)
        self.assertTrue((self.run / 'PAUSED').exists())
        self.assertEqual(self.call('status')['open_terra'], 1)
        self.enroll('new', outcome='paused')
        self.repair('t2')  # Work in flight continues past the threshold.

    def test_terra_p1_pauses_but_sol_p1_does_not(self):
        self.enroll('s')
        self.review('s', outcome='fail', p1=1)
        self.assertFalse((self.run / 'PAUSED').exists())
        self.enroll('t', byte=1)
        self.review('t', outcome='fail', p1=1)
        self.assertIn("['t']", rec.read_json(self.run / 'PAUSED')['reason'])

    def test_review_validation_and_write_once(self):
        self.enroll()
        self.review(p1=1, code=2)
        self.review(p2=1, code=2)
        self.assertFalse((self.episode() / 'review-first.json').exists())
        result = self.review(p3=2)['record']
        self.assertEqual(result['receipt']['sha256'], rec.sha256(self.receipt))
        self.assertIn('enrollment.json', result['prior_files'])
        self.review(code=2)

    def test_close_counts_repairs_and_first_dispatch_across_revisions(self):
        self.enroll()
        self.repair()
        self.repair()
        for rev, name, stamp in [('a' * 16, 'assignment.json', '2026-10-01T00:01:00Z'),
                                 ('b' * 16, 'assignment.json', '2026-10-01T00:00:00Z'),
                                 ('a' * 16, 'assignment-02.json', '2026-10-01T00:02:00Z')]:
            d = Path(rec.record_dir(str(self.run), 'a@' + rev))
            d.mkdir(parents=True, exist_ok=True)
            (d / name).write_text(json.dumps({'assigned_at': stamp}))
        with mock.patch.object(rec, 'now', return_value='2026-10-01T00:03:00.000000+00:00'):
            out = self.close()['record']
        self.assertEqual(out['repair_rounds'], 2)
        self.assertEqual(out['elapsed_seconds'], 180)
        self.assertEqual(out['first_dispatch_at'], '2026-10-01T00:00:00Z')
        self.assertEqual(len(out['assignments']), 3)
        self.assertEqual(out['artifacts'][0]['sha256'], rec.sha256(self.receipt))
        self.assertEqual(out['pre_review_fixes'], 2)
        self.close(code=2)

    def test_close_without_dispatch_has_null_time_and_zero_repairs(self):
        self.enroll()
        out = self.close(disposition='abandoned')['record']
        self.assertEqual(out['repair_rounds'], 0)
        self.assertIsNone(out['first_dispatch_at'])
        self.assertIsNone(out['elapsed_seconds'])
        self.assertFalse(rec.integrity_problems(str(self.episode())))

    def test_withdraw_refuses_after_any_revision_dispatched(self):
        self.enroll()
        d = Path(rec.record_dir(str(self.run), 'a@' + 'c' * 16))
        d.mkdir(parents=True)
        (d / 'assignment-02.json').write_text('{}')  # Existence alone proves dispatch; do not parse it.
        self.call('withdraw', '--task-id', 'a', '--reason', 'late', code=2)
        self.assertFalse((self.episode() / 'withdraw.json').exists())

    def test_close_and_other_mutations_refuse_after_withdraw(self):
        self.enroll()
        self.call('withdraw', '--task-id', 'a', '--reason', 'owner withdrew')
        self.close(code=2)
        self.review(code=2)
        self.call('repair', '--task-id', 'a', '--review-receipt', self.receipt, code=2)
        self.enroll(outcome='already_enrolled')

    def test_resume_requires_approval_and_preserves_history(self):
        self.call('resume', '--owner-approval', 'approved', code=2)
        first = self.call('pause', '--reason', 'hold')['pause']
        self.assertEqual(self.call('pause', '--reason', 'another')['outcome'], 'already_paused')
        self.call('resume', '--owner-approval', '  ', code=2)
        self.assertTrue((self.run / 'PAUSED').exists())
        resumed = self.call('resume', '--owner-approval', 'owner approved resumption')
        history = rec.read_json(resumed['history'])
        self.assertEqual(history['paused_at'], first['paused_at'])
        self.assertEqual(history['owner_approval'], 'owner approved resumption')
        self.assertIn('resumed_at', history)
        self.assertFalse((self.run / 'PAUSED').exists())
        self.call('pause', '--reason', 'second')
        self.call('resume', '--owner-approval', 'owner approved again')
        self.assertEqual(len(list((self.run / 'pauses').glob('*.json'))), 2)

    def test_status_totals_include_open_closed_withdrawn_and_reviews(self):
        self.enroll('t', byte=1)
        self.enroll('s')
        self.enroll('w')
        self.enroll('no', eligible='no', outcome='not_eligible')
        self.review('t')
        self.review('s', outcome='fail', p2=1)
        self.repair('t')
        self.close('s')
        self.call('withdraw', '--task-id', 'w', '--reason', 'withdrawn')
        out = self.call('status', '--task-id', 't')
        self.assertEqual(out['enrollments'], {'total': 3, 'per_arm': {rec.TERRA: 1, rec.SOL: 2}})
        self.assertEqual((out['open'], out['closed'], out['withdrawn'], out['open_terra']), (1, 1, 1, 1))
        self.assertEqual(out['terra_repair_rounds'], 1)
        self.assertEqual(out['first_reviews'][rec.TERRA]['pass'], 1)
        self.assertEqual(out['first_reviews'][rec.SOL], {'pass': 0, 'fail': 1, 'pending': 1})
        self.assertEqual(out['screening']['outcomes'], {'enrolled': 3, 'not_eligible': 1})
        self.assertFalse(out['inconsistencies'])
        self.assertIn('repair-01.json', out['episode'])
        self.assertEqual(self.call('status', '--task-id', 'no')['episode'], 'no record')

    def test_verify_detects_edit_and_prior_hash_even_with_rewritten_sums(self):
        self.enroll()
        self.repair()
        self.call('verify')
        p = self.episode() / 'enrollment.json'
        e = rec.read_json(p)
        e['reason'] = 'edited'
        p.write_text(json.dumps(e))
        self.call('verify', code=1)
        # Rewriting SHA256SUMS cannot hide the hash in the later repair record.
        (self.episode() / rec.SUMS).unlink()
        rec.append_sums(str(self.episode()))
        self.assertIn('different hash', json.dumps(self.call('verify', code=1)))
        self.call('repair', '--task-id', 'a', '--review-receipt', self.receipt, code=1)

    def test_verify_detects_missing_sums_and_unlisted_files(self):
        self.enroll()
        (self.episode() / 'extra.json').write_text('{}')
        self.assertIn('unlisted', json.dumps(self.call('verify', code=1)))
        (self.episode() / 'extra.json').unlink()
        (self.episode() / rec.SUMS).unlink()
        self.assertIn('verified', self.call('verify')['text'])
        self.assertTrue((self.run / 'recovered.jsonl').exists())

    def test_new_commands_require_round_and_round_is_write_once(self):
        original = (self.run / 'round.json').read_bytes()
        self.call('init-round', '--cap', '1', '--terra-in-flight', '1', '--terra-repair-budget', '1', code=2)
        self.assertEqual((self.run / 'round.json').read_bytes(), original)
        self.run = self.root / 'uninitialized'
        commands = [('enroll', '--task-id', 'a', '--task-revision', 'a@' + 'a' * 16, '--eligible', 'yes', '--reason', 'test'),
                    ('withdraw', '--task-id', 'a', '--reason', 'test'), ('pause', '--reason', 'hold'),
                    ('resume', '--owner-approval', 'approved'), ('status',), ('recover-screening',),
                    ('repair', '--task-id', 'a', '--review-receipt', self.receipt),
                    ('review', '--task-id', 'a', '--outcome', 'pass', '--p1', '0', '--p2', '0', '--p3', '0', '--receipt', self.receipt),
                    ('close', '--task-id', 'a', '--disposition', 'accepted', '--reason', 'test', '--pre-review-fixes', '0', '--escalations', '0')]
        with mock.patch.object(rec.os, 'urandom') as draw:
            for command in commands:
                self.call(*command, code=2)
            draw.assert_not_called()
        self.assertFalse(self.run.exists())

    def test_invalid_task_id_or_revision_cannot_escape_episode_directory(self):
        for task, rev in [('../escape', 'a@' + 'a' * 16), ('a', 'bad'), ('a', 'b@' + 'a' * 16)]:
            self.call('enroll', '--task-id', task, '--task-revision', rev, '--eligible', 'yes', '--reason', 'test', code=2)
        self.assertFalse((self.run / 'enrollments').exists())

    def test_reviewer_leak_check_names_case_insensitive_and_terra_whole_word(self):
        for i, (text, expected) in enumerate([('GPT-6.1-TERRA and GpT-6.1-SoL; TERRA; sOL 6.1',
                                              {'gpt-6.1-terra', 'gpt-6.1-sol', 'Terra', 'Sol 6.1'}),
                                             ('terracotta and subterranean', set())]):
            rev = 'review' + str(i) + '@' + 'b' * 16
            d = Path(rec.record_dir(str(self.run), rev))
            d.mkdir(parents=True)
            rec.write_once(d / 'profile-failed.json', {'stage': 'test'})
            rec.write_once(d / 'overhead.json', {'frozen_at': '2026-01-01T00:00:00Z'})
            packet = self.root / ('packet' + str(i) + '.md')
            packet.write_text(text)
            self.call('assign', '--task-revision', rev, '--role', 'reviewer', '--requested-model', 'review-model',
                      '--requested-effort', 'high', '--harness', 'test', '--run', 'synthetic', '--dispatched-packet', packet,
                      '--child-session', 'fake-session')
            result = rec.read_json(d / 'assignment.json')
            self.assertEqual(set(result['leaked_tokens']), expected)
            self.assertEqual(result['profile_absent_from_dispatch'], not expected)


if __name__ == '__main__':
    unittest.main()
