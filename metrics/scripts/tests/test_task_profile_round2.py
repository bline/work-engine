"""Round-2 tooling tests. All run directories and evidence are synthetic."""
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import task_profile_record as rec

SCRIPT = str(Path(rec.__file__).resolve())


class Round2Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.run = self.root / 'run'
        self.receipt = self.root / 'review.md'
        self.receipt.write_text('synthetic review\n')
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
        # Every command emits exactly one JSON object.
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
        return self.call('repair', '--task-id', task, '--review-receipt', self.receipt)

    def close(self, task='a', code=0, disposition='accepted'):
        return self.call('close', '--task-id', task, '--disposition', disposition, '--reason', 'test outcome',
                         '--pre-review-fixes', '2', '--escalations', '1', '--review-receipt', self.receipt,
                         '--artifact', self.receipt, code=code)

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

    def test_exclusive_create_loser_reads_winners_arm(self):
        real_open = rec.os.open
        def winner(path, flags, mode=0o777):
            self.assertTrue(flags & os.O_EXCL)
            data = {'task_id': 'a', 'arm': rec.TERRA, 'reason': 'other caller won'}
            with open(path, 'x') as fh:
                json.dump(data, fh)
            rec.append_sums(os.path.dirname(path))
            return real_open(path, flags, mode)  # Really raises FileExistsError.
        with mock.patch.object(rec.os, 'open', side_effect=winner), mock.patch.object(rec.os, 'urandom', return_value=b'\x00'):
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
        self.assertIn('episode t', rec.read_json(self.run / 'PAUSED')['reason'])

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
        (self.episode() / rec.SUMS).unlink()
        self.assertIn('missing or empty', json.dumps(self.call('verify', code=1)))

    def test_new_commands_require_round_and_round_is_write_once(self):
        original = (self.run / 'round.json').read_bytes()
        self.call('init-round', '--cap', '1', '--terra-in-flight', '1', '--terra-repair-budget', '1', code=2)
        self.assertEqual((self.run / 'round.json').read_bytes(), original)
        self.run = self.root / 'uninitialized'
        commands = [('enroll', '--task-id', 'a', '--task-revision', 'a@' + 'a' * 16, '--eligible', 'yes', '--reason', 'test'),
                    ('withdraw', '--task-id', 'a', '--reason', 'test'), ('pause', '--reason', 'hold'),
                    ('resume', '--owner-approval', 'approved'), ('status',),
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
