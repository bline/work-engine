"""Tests for the read-only `admission` gate of task_profile_record.py.

Every run directory is a temporary synthetic one. The gate is run as a real process
so its exit code is the one a supervisor sees. Fixtures are built with the existing
round commands; cases that need a state the commands refuse to leave (an unmarked
stop, an episode that needs manifest recovery) edit the temporary directory directly.
"""
import ast
import builtins
import contextlib
import errno
import glob
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import task_profile_record as rec

SCRIPT = str(Path(rec.__file__).resolve())
EXIT = {"active": 0, "paused": 10, "stop_reached": 11, "unknown": 12}


def snapshot(path):
    """Recursive (path, type, size, mtime_ns, sha256) of everything under path, without following links."""
    out = {}

    def entry(p, rel):
        try:
            st = os.lstat(p)
        except OSError as e:  # An entry of a directory without search permission cannot be examined.
            out[rel] = ("error", type(e).__name__, None, None)
            return
        kind = ("symlink" if stat.S_ISLNK(st.st_mode) else "dir" if stat.S_ISDIR(st.st_mode)
                else "file" if stat.S_ISREG(st.st_mode) else "other")
        digest = None
        if kind == "file":
            try:
                digest = hashlib.sha256(Path(p).read_bytes()).hexdigest()
            except OSError:
                digest = "unreadable"
        elif kind == "symlink":
            digest = os.readlink(p)
        out[rel] = (kind, st.st_size, st.st_mtime_ns, digest)

    root = str(path)
    if not os.path.lexists(root):
        return {"<absent>": None}
    entry(root, ".")
    if os.path.isdir(root) and not os.path.islink(root):
        for base, dirs, files in os.walk(root):
            for name in dirs + files:
                p = os.path.join(base, name)
                entry(p, os.path.relpath(p, root))
    return out


class AdmissionBase(unittest.TestCase):
    ADMISSION_TIMEOUT = 15  # A gate that hangs is a failure: no state, no exit code.

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.receipt = self.root / "receipt.md"
        self.receipt.write_text("synthetic receipt\n")
        self.receipts = 0
        self.snapshots_compared = 0

    # --- fixtures built with the existing commands -------------------------------------------------
    def rcall(self, run, command, *args, code=0):
        out = io.StringIO()
        argv = [SCRIPT, command, "--run-dir", str(run), *map(str, args)]
        with mock.patch.object(sys, "argv", argv), contextlib.redirect_stdout(out):
            try:
                rec.main()
                actual = 0
            except SystemExit as e:
                actual = e.code
        self.assertEqual(actual, code, out.getvalue())
        # Round commands leave their lock file behind. Remove it so that a lock file present when
        # admission has run could only have been created by admission.
        lock = Path(run) / ".round.lock"
        if lock.exists():
            lock.unlink()
        return json.loads(out.getvalue())

    def new_run(self, name="run", cap=20, in_flight=2, budget=3):
        run = self.root / name
        self.rcall(run, "init-round", "--cap", cap, "--terra-in-flight", in_flight, "--terra-repair-budget", budget)
        return run

    def enroll(self, run, task, byte=0, outcome="enrolled"):
        with mock.patch.object(rec.os, "urandom", return_value=bytes([byte])):
            out = self.rcall(run, "enroll", "--task-id", task, "--task-revision", task + "@" + "a" * 16,
                             "--eligible", "yes", "--reason", "synthetic")
        self.assertEqual(out["outcome"], outcome)

    def review(self, run, task, p1=0):
        verdict = "changes_required" if p1 else "accept"
        self.rcall(run, "review", "--task-id", task, "--verdict", verdict, "--reviewer-session", "r",
                   "--p1", p1, "--p2", 0, "--p3", 0, "--receipt", self.receipt)

    def repair(self, run, task):
        self.receipts += 1
        receipt = self.root / f"repair-{self.receipts}.md"
        receipt.write_text(f"synthetic repair review {self.receipts}")
        self.rcall(run, "repair", "--task-id", task, "--review-receipt", receipt)

    def resume(self, run):
        self.rcall(run, "resume", "--owner-approval", "synthetic owner approval")

    def unmark(self, run):
        """Remove PAUSED, leaving the facts that wrote it: the unmarked stop."""
        self.assertTrue((run / "PAUSED").exists())
        (run / "PAUSED").unlink()

    # --- the gate under test ------------------------------------------------------------------------
    def admit(self, run, state, code=None, *extra):
        """Run admission as a process, assert its state and exit code, and that it changed nothing."""
        before = snapshot(run)
        proc = subprocess.run([sys.executable, "-I", SCRIPT, "admission", "--run-dir", str(run), *extra],
                              capture_output=True, text=True, timeout=self.ADMISSION_TIMEOUT)
        after = snapshot(run)
        self.assertEqual(before, after, "admission changed the run directory")
        self.snapshots_compared += 1
        self.assertFalse(os.path.lexists(os.path.join(str(run), ".round.lock")), "admission created a lock file")
        out = json.loads(proc.stdout)
        self.assertEqual(out["state"], state, proc.stdout)
        self.assertEqual(proc.returncode, EXIT[state], proc.stderr)
        if code is not None:
            self.assertEqual(out["reason"]["code"], code, proc.stdout)
        return out

    def copy_of(self, run, name):
        dest = self.root / name
        shutil.copytree(run, dest, symlinks=True)
        return dest


    # --- helpers for the class-repair fixtures ---------------------------------------------------------
    def rechain(self, run, task, edits):
        """Rewrite an episode's JSON records in order with edits[name](data) applied, keeping every seal,
        backward link and manifest line consistent. Only the content the edit chose is wrong afterwards."""
        d = Path(run) / "enrollments" / task
        order = [line.split("  ", 1)[1] for line in (d / "SHA256SUMS").read_text().splitlines()]
        for name in order:
            if not name.endswith(".json"):
                continue
            data = json.loads((d / name).read_text())
            data.pop("content_sha256", None)
            if name in edits:
                edits[name](data)
            if "prior_files" in data:
                data["prior_files"] = {pn: rec.sha256(str(d / pn)) for pn in data["prior_files"]}
            (d / name).write_text(json.dumps(rec.seal_episode_record(data)))
        self.write_manifest(d, order)

    def write_manifest(self, d, names):
        Path(d, "SHA256SUMS").write_text("".join(f"{rec.sha256(str(Path(d) / n))}  {n}\n" for n in names))

    def add_record_copy(self, run, task, source, new_name):
        """A byte copy of a record under another name, listed in the manifest (hash-consistent, so only
        its name or its content relative to its siblings is wrong)."""
        d = Path(run) / "enrollments" / task
        shutil.copyfile(d / source, d / new_name)
        names = [line.split("  ", 1)[1] for line in (d / "SHA256SUMS").read_text().splitlines()]
        self.write_manifest(d, names + [new_name])

    def replace_with_fifo(self, path):
        os.unlink(path)
        os.mkfifo(path)

    def replace_with_dangling_symlink(self, path):
        os.unlink(path)
        os.symlink(str(self.root / "nowhere"), str(path))

    def terra_with_review(self, name, p1=0, budget=3):
        run = self.new_run(name, budget=budget)
        self.enroll(run, "t", byte=1)
        self.review(run, "t", p1=p1)
        if (run / "PAUSED").exists():
            self.unmark(run)
        return run

    def terra_with_repair(self, name, budget=3):
        run = self.new_run(name, budget=budget)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        if (run / "PAUSED").exists():
            self.unmark(run)
        return run

    # --- fail-closed fixtures (shared by AdmissionFailsClosed and the case-11 no-write test) ---------------
    def chmod_for_test(self, path, mode):
        """chmod, restoring access before temporary-directory cleanup (cleanups run last-in first-out)."""
        self.addCleanup(os.chmod, path, 0o755)
        os.chmod(path, mode)

    def p1_unmarked(self, name):
        """A Terra episode with a P1 review and no PAUSED marker: an unmarked stop, until covered."""
        run = self.new_run(name)
        self.enroll(run, "task-p1", byte=1)
        self.review(run, "task-p1", p1=1)
        self.unmark(run)
        return run

    def rounds_unmarked(self, name):
        """One Terra repair round against a budget of one, no PAUSED marker."""
        run = self.new_run(name, budget=1)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        self.unmark(run)
        return run

    def write_pause_history(self, run, document, name="x.json"):
        (run / "pauses").mkdir(exist_ok=True)
        text = document if isinstance(document, str) else json.dumps(document)
        (run / "pauses" / name).write_text(text)

    def with_enrollment(self, name):
        run = self.new_run(name)
        self.enroll(run, "t", byte=1)
        return run

    def fail_closed_cases(self):
        """name -> (build(name) -> run, expected reason code). Each is a state that must not be admitted."""
        def history(builder, document):
            def build(name):
                run = builder(name)
                self.write_pause_history(run, document)
                return run
            return build

        def enrollments_unreadable(name):
            run = self.with_enrollment(name)
            self.chmod_for_test(run / "enrollments", 0o000)
            return run

        def enrollments_not_searchable(name):
            run = self.with_enrollment(name)
            self.chmod_for_test(run / "enrollments", 0o444)  # Names list; no entry can be examined.
            return run

        def enrollments_regular_file(name):
            run = self.new_run(name)
            (run / "enrollments").write_text("not a directory")
            return run

        def enrollments_replaced_by_file(name):
            run = self.with_enrollment(name)
            shutil.rmtree(run / "enrollments")
            (run / "enrollments").write_text("not a directory")
            return run

        def enrollments_dangling_symlink(name):
            run = self.new_run(name)
            os.symlink(str(self.root / "nowhere"), str(run / "enrollments"))
            return run

        def enrollment_entry_symlink_loop(name):
            run = self.new_run(name)
            (run / "enrollments").mkdir()
            os.symlink("loop", str(run / "enrollments" / "loop"))
            return run

        def episode_unreadable(name):
            run = self.with_enrollment(name)
            self.chmod_for_test(run / "enrollments" / "t", 0o000)
            return run

        def episode_not_searchable(name):
            run = self.with_enrollment(name)
            self.chmod_for_test(run / "enrollments" / "t", 0o444)  # Names list; no record can be examined.
            return run

        def episode_not_listable(name):
            run = self.with_enrollment(name)
            self.chmod_for_test(run / "enrollments" / "t", 0o311)  # Searchable; names cannot be listed.
            return run

        def pauses_unreadable(name):
            run = self.p1_unmarked(name)
            self.write_pause_history(run, {"covered_triggers": {"p1_episodes": ["task-p1"]}})
            self.chmod_for_test(run / "pauses", 0o000)
            return run

        def pauses_regular_file(name):
            run = self.p1_unmarked(name)
            (run / "pauses").write_text("not a directory")
            return run

        def pauses_dangling_symlink(name):
            run = self.p1_unmarked(name)
            os.symlink(str(self.root / "nowhere"), str(run / "pauses"))
            return run

        def pause_entry_is_a_directory(name):
            run = self.p1_unmarked(name)
            (run / "pauses").mkdir()
            (run / "pauses" / "x.json").mkdir()
            return run

        def run_dir_not_searchable(name):
            run = self.with_enrollment(name)
            self.chmod_for_test(run, 0o644)  # round.json, PAUSED and the rest cannot be examined.
            return run

        def one_bad_among_good(name):
            run = history(self.p1_unmarked, {"covered_triggers": {"p1_episodes": ["task-p1"]}})(name)
            self.write_pause_history(run, {"covered_triggers": {"p1_episodes": {"task-p1": 0}}}, "y.json")
            return run

        # --- class repair: every input of the admission decision (see input-inventory.md) ----------------
        missing = object()

        def p1_value(value):
            """A Terra episode whose sealed first review carries `value` as p1 (the review is the only record
            after the enrollment, so the chain stays consistent). With p1 = 1 the stop is real."""
            def build(name):
                run = self.terra_with_review(name, p1=1)

                def edit(data):
                    if value is missing:
                        data.pop("p1", None)
                    else:
                        data["p1"] = value
                self.rechain(run, "t", {"review-first.json": edit})
                return run
            return build

        def special_entry(record, how, builder=None):
            def build(name):
                run = (builder or self.terra_with_review)(name)
                path = run / "enrollments" / "t" / record
                (self.replace_with_fifo if how == "fifo" else self.replace_with_dangling_symlink)(path)
                return run
            return build

        def special_enrollments_entry(name):
            run = self.new_run(name)
            (run / "enrollments").mkdir()
            os.symlink(str(self.root / "nowhere"), str(run / "enrollments" / "ghost"))
            return run

        def fifo_pause_history(name):
            run = self.p1_unmarked(name)
            (run / "pauses").mkdir()
            os.mkfifo(run / "pauses" / "x.json")
            return run

        def dangling_pause_history(name):
            run = self.p1_unmarked(name)
            (run / "pauses").mkdir()
            os.symlink(str(self.root / "nowhere"), str(run / "pauses" / "x.json"))
            return run

        def fifo_round_json(name):
            run = self.new_run(name)
            self.replace_with_fifo(run / "round.json")
            return run

        def fifo_manifest(name):
            run = self.with_enrollment(name)
            self.replace_with_fifo(run / "enrollments" / "t" / "SHA256SUMS")
            return run

        def round_text(text):
            def build(name):
                run = self.new_run(name)
                (run / "round.json").write_text(text)
                return run
            return build

        def repair_edit(edit):
            def build(name):
                run = self.terra_with_repair(name, budget=1)
                self.rechain(run, "t", {"repair-01.json": edit})
                return run
            return build

        def repair_extra_name(new_name):
            def build(name):
                run = self.terra_with_repair(name, budget=1)
                self.add_record_copy(run, "t", "repair-01.json", new_name)
                return run
            return build

        def repair_lone_number_two(name):
            """The only repair record is numbered 2 (round 2): the recorder numbers a record highest + 1, so it
            asserts a round 1 that is absent."""
            run = self.terra_with_repair(name, budget=2)  # counted 1 of 2: the old gate says active, the numbering says 2
            self.rechain(run, "t", {"repair-01.json": lambda data: data.update(round=2)})
            d = run / "enrollments" / "t"
            os.rename(d / "repair-01.json", d / "repair-02.json")
            names = [line.split("  ", 1)[1] for line in (d / "SHA256SUMS").read_text().splitlines()]
            self.write_manifest(d, ["repair-02.json" if n == "repair-01.json" else n for n in names])
            return run

        def repair_gap_one_and_three(name):
            run = self.terra_with_repair(name, budget=1)
            self.add_record_copy(run, "t", "repair-01.json", "repair-03.json")
            self.rechain(run, "t", {"repair-03.json": lambda data: data.update(round=3)})
            return run

        def enrollment_task_id(name):
            run = self.terra_with_review(name, p1=1)
            self.rechain(run, "t", {"enrollment.json": lambda data: data.update(task_id="other")})
            return run

        def enrollment_task_id_masks_a_stop(name):
            """The enrollment names another task and the history covers that name: the old gate reads the P1 as
            covered."""
            run = enrollment_task_id(name)
            self.write_pause_history(run, {"covered_triggers": {"p1_episodes": ["other"]}})
            return run

        def review_task_id(name):
            run = self.terra_with_review(name, p1=1)
            self.rechain(run, "t", {"review-first.json": lambda data: data.update(task_id="other")})
            return run

        def duplicate_manifest_line(name):
            run = self.with_enrollment(name)
            sums = run / "enrollments" / "t" / "SHA256SUMS"
            sums.write_bytes(sums.read_bytes() + sums.read_bytes().splitlines(keepends=True)[0])
            return run

        def history_text(text, builder=None):
            def build(name):
                run = (builder or self.p1_unmarked)(name)
                self.write_pause_history(run, text)
                return run
            return build

        def terra_dir_special(record_dir_name):
            def build(name):
                run = self.terra_with_review(name)
                os.mkdir(run / "enrollments" / "t" / record_dir_name)
                return run
            return build

        p1_values = {"nan": float("nan"), "negative": -1, "minus_infinity": float("-inf"), "infinity": float("inf"),
                     "bool_true": True, "bool_false": False, "float_zero": 0.0, "float_one": 1.0, "string": "1",
                     "null": None, "list": [1], "missing": missing}

        p1 = self.p1_unmarked
        rounds = self.rounds_unmarked
        cases = {
            "enrollments_unreadable": (enrollments_unreadable, "enrollments_unlistable"),
            "enrollments_not_searchable": (enrollments_not_searchable, "enrollments_unlistable"),
            "enrollments_regular_file": (enrollments_regular_file, "enrollments_unlistable"),
            "enrollments_replaced_by_file": (enrollments_replaced_by_file, "enrollments_unlistable"),
            "enrollments_dangling_symlink": (enrollments_dangling_symlink, "enrollments_unlistable"),
            "enrollment_entry_symlink_loop": (enrollment_entry_symlink_loop, "enrollments_unlistable"),
            "episode_unreadable": (episode_unreadable, "episode_unlistable"),
            "episode_not_searchable": (episode_not_searchable, "episode_unlistable"),
            "episode_not_listable": (episode_not_listable, "episode_unlistable"),
            "pauses_unreadable": (pauses_unreadable, "pause_history_unlistable"),
            "pauses_regular_file": (pauses_regular_file, "pause_history_unlistable"),
            "pauses_dangling_symlink": (pauses_dangling_symlink, "pause_history_unlistable"),
            "pause_entry_is_a_directory": (pause_entry_is_a_directory, "trigger_inputs_unreadable"),
            "run_dir_not_searchable": (run_dir_not_searchable, "pause_marker_unreadable"),
            "p1_episodes_dict_suppresses_a_stop": (history(p1, {"covered_triggers": {"p1_episodes": {"task-p1": False}}}),
                                                   "pause_history_malformed"),
            "p1_episodes_list_with_non_string": (history(p1, {"covered_triggers": {"p1_episodes": ["task-p1", 1]}}),
                                                 "pause_history_malformed"),
            "p1_episodes_only_non_string": (history(p1, {"covered_triggers": {"p1_episodes": [1]}}),
                                            "pause_history_malformed"),
            "p1_episodes_string": (history(p1, {"covered_triggers": {"p1_episodes": "task-p1"}}), "pause_history_malformed"),
            "p1_episodes_null": (history(p1, {"covered_triggers": {"p1_episodes": None}}), "pause_history_malformed"),
            "rounds_string": (history(rounds, {"covered_triggers": {"terra_repair_rounds": "many"}}),
                              "pause_history_malformed"),
            "rounds_float_suppresses_a_stop": (history(rounds, {"covered_triggers": {"terra_repair_rounds": 5.0}}),
                                               "pause_history_malformed"),
            "rounds_bool_suppresses_a_stop": (history(rounds, {"covered_triggers": {"terra_repair_rounds": True}}),
                                              "pause_history_malformed"),
            "rounds_null": (history(rounds, {"covered_triggers": {"terra_repair_rounds": None}}), "pause_history_malformed"),
            "covered_triggers_list": (history(p1, {"covered_triggers": []}), "pause_history_malformed"),
            "covered_triggers_string": (history(p1, {"covered_triggers": "task-p1"}), "pause_history_malformed"),
            "covered_triggers_null": (history(p1, {"covered_triggers": None}), "pause_history_malformed"),
            "document_is_a_list": (history(p1, ["task-p1"]), "pause_history_malformed"),
            "document_is_null": (history(p1, "null"), "pause_history_malformed"),
            "one_bad_file_among_good_ones": (one_bad_among_good, "pause_history_malformed"),
            # A FIFO or a dangling link where a record or history file is expected: unknown, never a hang.
            "fifo_pause_history_entry": (fifo_pause_history, "trigger_inputs_unreadable"),
            "dangling_pause_history_entry": (dangling_pause_history, "trigger_inputs_unreadable"),
            "fifo_round_json": (fifo_round_json, "round_file_missing"),
            "fifo_manifest": (fifo_manifest, "episode_manifest_unreadable"),
            "fifo_review_record": (special_entry("review-first.json", "fifo"), "episode_unlistable"),
            "dangling_review_record": (special_entry("review-first.json", "dangling"), "episode_unlistable"),
            "fifo_enrollment_record": (special_entry("enrollment.json", "fifo", self.with_enrollment), "episode_unlistable"),
            "dangling_enrollment_record": (special_entry("enrollment.json", "dangling", self.with_enrollment),
                                           "episode_unlistable"),
            "dangling_enrollments_entry": (special_enrollments_entry, "enrollments_unlistable"),
            "repair_record_is_a_directory": (terra_dir_special("repair-02.json"), "episode_unlistable"),
            "close_record_is_a_directory": (terra_dir_special("close.json"), "episode_unlistable"),
            "repair_fifo": (lambda name: (lambda run: (os.mkfifo(run / "enrollments" / "t" / "repair-02.json"), run)[1])(
                self.terra_with_repair(name, budget=1)), "episode_unlistable"),
            "repair_dangling": (lambda name: (lambda run: (os.symlink(str(self.root / "nowhere"),
                                                                       str(run / "enrollments" / "t" / "repair-09.json")), run)[1])(
                self.terra_with_repair(name, budget=1)), "episode_unlistable"),
            # Counts and consistency of the evidence the stop rule reads.
            **{f"review_p1_{label}": (p1_value(value), "review_record_invalid") for label, value in p1_values.items()},
            "review_task_id_differs": (review_task_id, "review_record_invalid"),
            "enrollment_task_id_differs_from_directory": (enrollment_task_id, "enrollment_record_invalid"),
            "repair_round_differs_from_name": (repair_edit(lambda data: data.update(round=7)), "repair_record_invalid"),
            "repair_round_is_a_bool": (repair_edit(lambda data: data.update(round=True)), "repair_record_invalid"),
            "repair_task_id_differs": (repair_edit(lambda data: data.update(task_id="other")), "repair_record_invalid"),
            "repair_duplicate_number": (repair_extra_name("repair-001.json"), "repair_record_invalid"),
            "repair_name_not_numbered_as_the_recorder_writes": (repair_extra_name("repair-1.json"), "repair_record_invalid"),
            "repair_name_non_numeric": (repair_extra_name("repair-x.json"), "repair_record_invalid"),
            "repair_numbering_starts_at_two": (repair_lone_number_two, "repair_record_invalid"),
            "repair_numbering_skips_a_number": (repair_gap_one_and_three, "repair_record_invalid"),
            "manifest_duplicate_name": (duplicate_manifest_line, "episode_integrity_problem"),
            # Hand-edited unsealed files.
            "round_duplicate_keys": (round_text('{"cap": 1, "cap": 20, "terra_in_flight": 2, "terra_repair_budget": 3}'),
                                     "round_file_unreadable"),
            "round_nan_literal": (round_text('{"cap": NaN, "terra_in_flight": 2, "terra_repair_budget": 3}'),
                                  "round_file_unreadable"),
            "round_infinity_literal": (round_text('{"cap": Infinity, "terra_in_flight": 2, "terra_repair_budget": 3}'),
                                       "round_file_unreadable"),
            "round_negative_cap": (round_text('{"cap": -1, "terra_in_flight": 2, "terra_repair_budget": 3}'),
                                   "round_file_invalid"),
            "round_negative_budget": (round_text('{"cap": 20, "terra_in_flight": 2, "terra_repair_budget": -1}'),
                                      "round_file_invalid"),
            "round_negative_in_flight": (round_text('{"cap": 20, "terra_in_flight": -2, "terra_repair_budget": 3}'),
                                         "round_file_invalid"),
            "round_not_utf8": (lambda name: (lambda run: ((run / "round.json").write_bytes(b'{"cap": "\xff"}'), run)[1])(
                self.new_run(name)), "round_file_unreadable"),
            "pause_duplicate_keys": (history_text('{"covered_triggers": {"p1_episodes": ["task-p1"]}, "covered_triggers": {}}'),
                                     "trigger_inputs_unreadable"),
            "pause_duplicate_keys_last_one_covers": (
                history_text('{"covered_triggers": {}, "covered_triggers": {"p1_episodes": ["task-p1"]}}'),
                "trigger_inputs_unreadable"),
            "enrollment_task_id_masks_a_stop": (enrollment_task_id_masks_a_stop, "enrollment_record_invalid"),
            "pause_nan_rounds": (history_text('{"covered_triggers": {"terra_repair_rounds": NaN}}'),
                                 "trigger_inputs_unreadable"),
            "pause_infinity_rounds": (history_text('{"covered_triggers": {"terra_repair_rounds": Infinity}}', rounds),
                                      "trigger_inputs_unreadable"),
            "pause_negative_rounds": (history_text('{"covered_triggers": {"terra_repair_rounds": -5}}', rounds),
                                      "pause_history_malformed"),
            "pause_negative_rounds_minus_one": (history_text('{"covered_triggers": {"terra_repair_rounds": -1}}'),
                                                "pause_history_malformed"),
            "pause_not_utf8": (lambda name: (lambda run: (self.write_pause_history(run, "{}"),
                                                          (run / "pauses" / "x.json").write_bytes(b'{"a": "\xff"}'), run)[2])(
                self.p1_unmarked(name)), "trigger_inputs_unreadable"),
        }
        return cases


class AdmissionStates(AdmissionBase):
    def test_1_active(self):
        run = self.new_run()
        self.enroll(run, "a", byte=1)
        out = self.admit(run, "active", "admitted")
        self.assertEqual(out["facts"]["enrollments"]["count"], 1)
        self.assertEqual(out["facts"]["pause_marker"], {"evaluated": True, "present": False})
        self.assertEqual(out["facts"]["trigger"]["terra_repair_rounds"], 0)

    def test_1_active_with_no_enrollments_yet(self):
        self.admit(self.new_run(), "active")

    def test_2_paused_valid_marker(self):
        run = self.new_run()
        self.rcall(run, "pause", "--reason", "synthetic owner pause")
        out = self.admit(run, "paused", "pause_marker_present")
        marker = out["facts"]["pause_marker"]
        self.assertTrue(marker["valid"])
        self.assertEqual(marker["reason"], "synthetic owner pause")
        self.assertEqual(out["facts"]["round"], {"evaluated": False})

    def test_3_paused_broken_markers(self):
        marker_kinds = {}

        def malformed(p):
            p.write_text("{not json")

        def empty(p):
            p.write_text("")

        def not_object(p):
            p.write_text("[1]")

        def unreadable(p):
            p.write_text("{}")
            p.chmod(0)

        def directory(p):
            p.mkdir()

        def broken_symlink(p):
            p.symlink_to(self.root / "no-such-target")

        def fifo(p):
            os.mkfifo(p)

        for name, make in (("malformed", malformed), ("empty", empty), ("not_object", not_object),
                           ("unreadable", unreadable), ("directory", directory),
                           ("broken_symlink", broken_symlink), ("fifo", fifo)):
            with self.subTest(name):
                run = self.new_run("run-" + name)
                make(run / "PAUSED")
                out = self.admit(run, "paused", "pause_marker_present")
                marker_kinds[name] = out["facts"]["pause_marker"]["kind"]
        self.assertEqual(marker_kinds["broken_symlink"], "broken_symlink")
        self.assertEqual(marker_kinds["directory"], "directory")
        self.assertEqual(marker_kinds["fifo"], "other")

    def test_3_broken_symlink_is_paused_here_but_enroll_does_not_see_it(self):
        run = self.new_run()
        (run / "PAUSED").symlink_to(self.root / "no-such-target")
        self.assertFalse(os.path.exists(run / "PAUSED"))
        self.assertTrue(os.path.lexists(run / "PAUSED"))
        self.admit(run, "paused")

    def test_4_missing_run_directory(self):
        self.admit(self.root / "absent", "unknown", "run_dir_missing")
        file_in_place = self.root / "plain-file"
        file_in_place.write_text("not a directory")
        self.admit(file_in_place, "unknown", "run_dir_missing")

    def test_5_bad_round_file(self):
        cases = {
            "missing": (None, "round_file_missing"),
            "malformed": ("{", "round_file_unreadable"),
            "not_object": ("[1, 2, 3]", "round_file_invalid"),
            "no_cap": ('{"terra_in_flight": 1, "terra_repair_budget": 1}', "round_file_invalid"),
            "no_in_flight": ('{"cap": 1, "terra_repair_budget": 1}', "round_file_invalid"),
            "no_budget": ('{"cap": 1, "terra_in_flight": 1}', "round_file_invalid"),
            "string_cap": ('{"cap": "20", "terra_in_flight": 1, "terra_repair_budget": 1}', "round_file_invalid"),
            "bool_cap": ('{"cap": true, "terra_in_flight": 1, "terra_repair_budget": 1}', "round_file_invalid"),
        }
        for name, (content, code) in cases.items():
            with self.subTest(name):
                run = self.new_run("run-" + name)
                if content is None:
                    (run / "round.json").unlink()
                else:
                    (run / "round.json").write_text(content)
                self.admit(run, "unknown", code)
        run = self.new_run("round-is-a-directory")
        (run / "round.json").unlink()
        (run / "round.json").mkdir()
        self.admit(run, "unknown", "round_file_missing")

    def test_6_unmarked_stop_repair_rounds(self):
        run = self.new_run(budget=2)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        self.admit(run, "active")  # One round, below the budget.
        self.repair(run, "t")      # Second round reaches the budget; the command writes PAUSED.
        self.unmark(run)
        out = self.admit(run, "stop_reached", "pause_trigger_unmarked")
        self.assertTrue(out["facts"]["trigger"]["repair_rounds_over_budget"])
        self.assertEqual(out["facts"]["trigger"]["terra_repair_rounds"], 2)

    def test_6_unmarked_stop_new_p1_episode(self):
        run = self.new_run()
        self.enroll(run, "t", byte=1)
        self.review(run, "t", p1=1)
        self.unmark(run)
        out = self.admit(run, "stop_reached", "pause_trigger_unmarked")
        self.assertEqual(out["facts"]["trigger"]["uncovered_p1_episodes"], ["t"])

    def test_6_sol_p1_is_not_a_trigger(self):
        run = self.new_run()
        self.enroll(run, "s", byte=0)
        self.review(run, "s", p1=1)
        self.assertFalse((run / "PAUSED").exists())
        self.admit(run, "active")

    def test_6_covered_facts_are_active_and_a_new_fact_is_not(self):
        run = self.new_run(budget=2)
        self.enroll(run, "t", byte=1)
        self.review(run, "t", p1=1)
        self.repair(run, "t")
        self.repair(run, "t")
        self.assertTrue((run / "PAUSED").exists())
        self.resume(run)  # The pause history now covers P1 ['t'] and 2 repair rounds.
        self.admit(run, "active")
        self.assertEqual(self.admit(run, "active")["facts"]["trigger"]["covered_repair_rounds"], 2)
        self.repair(run, "t")  # A third round is above the covered count and at or above the budget.
        self.unmark(run)
        self.admit(run, "stop_reached", "pause_trigger_unmarked")
        self.resume(run)
        self.enroll(run, "u", byte=1)
        self.review(run, "u", p1=1)  # A new uncovered P1 episode.
        self.unmark(run)
        out = self.admit(run, "stop_reached", "pause_trigger_unmarked")
        self.assertEqual(out["facts"]["trigger"]["uncovered_p1_episodes"], ["u"])

    def test_7_cap_reached(self):
        run = self.new_run(cap=2, in_flight=5)
        self.enroll(run, "a", byte=0)
        self.admit(run, "active")
        self.enroll(run, "b", byte=0)
        out = self.admit(run, "stop_reached", "cap_reached")
        self.assertEqual(out["facts"]["enrollments"], {"evaluated": True, "count": 2, "cap": 2})

    def test_8_in_flight_limit_is_a_fact_not_a_stop(self):
        run = self.new_run(in_flight=1)
        self.enroll(run, "t", byte=1)
        out = self.admit(run, "active")
        self.assertEqual(out["facts"]["terra_in_flight"], {"evaluated": True, "open_terra": 1, "limit": 1,
                                                           "limit_reached": True, "gates_admission": False})
        self.rcall(run, "withdraw", "--task-id", "t", "--reason", "not dispatched")
        self.assertFalse(self.admit(run, "active")["facts"]["terra_in_flight"]["limit_reached"])

    def test_9_unreadable_inputs(self):
        def corrupt_pause_history(run):
            (run / "pauses").mkdir()
            (run / "pauses" / "x.json").write_text("{")

        def pause_history_not_object(run):
            (run / "pauses").mkdir()
            (run / "pauses" / "x.json").write_text("[1]")

        def pause_history_wrong_shape(run):
            (run / "pauses").mkdir()
            (run / "pauses" / "x.json").write_text('{"covered_triggers": {"terra_repair_rounds": "many"}}')

        def edited_enrollment(run):
            path = run / "enrollments" / "t" / "enrollment.json"
            path.write_text(path.read_text().replace('"arm"', '"arm "'))

        def unlisted_record(run):
            (run / "enrollments" / "t" / "close.json").write_text("{}")

        def missing_manifest(run):
            (run / "enrollments" / "t" / "SHA256SUMS").unlink()

        def listed_but_missing(run):
            (run / "enrollments" / "t" / "SHA256SUMS").write_text("0" * 64 + "  repair-01.json\n")

        def torn_manifest(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes() + b"abc")

        def manifest_without_newline(run):
            # The last complete entry parses and verifies; repair_manifest_tail still treats the file as torn.
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes().rstrip(b"\n"))

        def manifest_with_crlf(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes().replace(b"\n", b"\r\n"))

        def manifest_with_bare_cr(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes().replace(b"\n", b"\r"))

        def manifest_entries_joined_by_bare_cr(run):
            # The manifest still ends in \n, so only the per-line test can see this one.
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes().replace(b"\n", b"\r", 1))

        def manifest_entries_joined_by_formfeed(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes().replace(b"\n", b"\f", 1))

        def manifest_entries_joined_by_line_separator(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.write_bytes(path.read_bytes().replace(b"\n", "\u2028".encode(), 1))

        def manifest_listing_vanished_records(run):
            # A directory whose manifest lists a record that is gone and that holds no record file.
            ghost = run / "enrollments" / "ghost"
            ghost.mkdir()
            (ghost / "SHA256SUMS").write_text("0" * 64 + "  enrollment.json\n")

        def manifest_torn_with_no_records(run):
            ghost = run / "enrollments" / "ghost"
            ghost.mkdir()
            (ghost / "SHA256SUMS").write_bytes(b"abc")

        def manifest_is_a_directory(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.unlink()
            path.mkdir()

        def corrupt_review(run):
            path = run / "enrollments" / "t" / "review-first.json"
            path.write_text("{")

        def review_resealed_without_p1(run):
            path = run / "enrollments" / "t" / "review-first.json"
            data = json.loads(path.read_text())
            del data["p1"]
            path.write_text(json.dumps(rec.seal_episode_record(data)))
            sums = path.parent / "SHA256SUMS"  # Keep the manifest honest: only the record's content is wrong.
            lines = [line.split("  ", 1) for line in sums.read_text().splitlines()]
            sums.write_text("".join(f"{rec.sha256(str(path)) if name == path.name else digest}  {name}\n"
                                    for digest, name in lines))

        def orphan_episode(run):
            # A self-consistent directory holding a record but no enrollment: enrollments() would not count it.
            orphan = run / "enrollments" / "orphan"
            orphan.mkdir()
            record = orphan / "review-first.json"
            record.write_text(json.dumps(rec.seal_episode_record({"p1": 0})))
            (orphan / "SHA256SUMS").write_text(f"{rec.sha256(str(record))}  review-first.json\n")

        cases = {
            "corrupt_pause_history": (corrupt_pause_history, "trigger_inputs_unreadable", False),
            "pause_history_not_object": (pause_history_not_object, "pause_history_malformed", False),
            "pause_history_wrong_shape": (pause_history_wrong_shape, "pause_history_malformed", False),
            "edited_enrollment": (edited_enrollment, "episode_integrity_problem", False),
            "unlisted_record": (unlisted_record, "episode_integrity_problem", False),
            "missing_manifest": (missing_manifest, "episode_integrity_problem", False),
            "listed_but_missing": (listed_but_missing, "episode_integrity_problem", False),
            "torn_manifest": (torn_manifest, "episode_integrity_problem", False),
            "manifest_without_newline": (manifest_without_newline, "episode_manifest_needs_repair", False),
            "manifest_with_crlf": (manifest_with_crlf, "episode_manifest_needs_repair", True),
            "manifest_with_bare_cr": (manifest_with_bare_cr, "episode_manifest_needs_repair", True),
            "manifest_entries_joined_by_bare_cr": (manifest_entries_joined_by_bare_cr, "episode_manifest_needs_repair", True),
            "manifest_entries_joined_by_formfeed": (manifest_entries_joined_by_formfeed, "episode_manifest_needs_repair", True),
            "manifest_entries_joined_by_line_separator": (manifest_entries_joined_by_line_separator, "episode_manifest_needs_repair", True),
            "manifest_listing_vanished_records": (manifest_listing_vanished_records, "episode_manifest_without_records", False),
            "manifest_torn_with_no_records": (manifest_torn_with_no_records, "episode_manifest_without_records", False),
            "manifest_is_a_directory": (manifest_is_a_directory, "episode_manifest_unreadable", False),
            "corrupt_review": (corrupt_review, "episode_integrity_problem", True),
            "review_resealed_without_p1": (review_resealed_without_p1, "review_record_invalid", True),
            "orphan_episode": (orphan_episode, "episode_without_enrollment", False),
        }
        for name, (damage, code, with_review) in cases.items():
            with self.subTest(name):
                run = self.new_run("run-" + name)
                self.enroll(run, "t", byte=1)
                if with_review:
                    self.review(run, "t", p1=0)
                damage(run)
                self.admit(run, "unknown", code)

    def test_9_integrity_problem_in_a_sol_episode_is_unknown_too(self):
        run = self.new_run()
        self.enroll(run, "s", byte=0)
        (run / "enrollments" / "s" / "SHA256SUMS").unlink()
        self.admit(run, "unknown", "episode_integrity_problem")

    def test_9_unexpected_exception_is_unknown_never_a_traceback(self):
        run = self.new_run()
        with mock.patch.object(rec, "read_round_config", side_effect=RuntimeError("synthetic")):
            out = rec.admission_decision(str(run))
        self.assertEqual((out["state"], out["reason"]["code"]), ("unknown", "unexpected_exception"))

    def test_10_plan_says_active_but_paused_marker_wins(self):
        plan = self.root / "plan.md"
        plan.write_text("Status (2026-10-28): **round 2 is active**. Limits: a cap of 20 enrolled episodes.\n")
        run = self.new_run()
        self.rcall(run, "pause", "--reason", "synthetic")
        self.admit(run, "paused")
        # The gate has no plan input: offering one is a usage error, exit 2, and not an admission.
        proc = subprocess.run([sys.executable, "-I", SCRIPT, "admission", "--run-dir", str(run), "--plan", str(plan)],
                              capture_output=True, text=True, timeout=60)
        self.assertEqual(proc.returncode, 2)
        self.assertNotIn("plan", subprocess.run([sys.executable, "-I", SCRIPT, "admission", "--help"],
                                                capture_output=True, text=True).stdout.split("Read-only gate")[0])

    def test_help_documents_exit_codes_and_the_symlink_rule(self):
        text = subprocess.run([sys.executable, "-I", SCRIPT, "admission", "--help"],
                              capture_output=True, text=True, timeout=60).stdout
        for needle in ("0   active", "10  paused", "11  stop_reached", "12  unknown", "broken symlink",
                       "os.path.lexists", "os.path.exists", "pause_trigger_unmarked", "cap_reached"):
            self.assertIn(needle, text)


class AdmissionWritesNothing(AdmissionBase):
    """Case 11. Each fixture is one where enroll or status would write, and admission must not."""

    def test_11_would_write_cases_are_untouched_and_the_old_path_does_write(self):
        # Unmarked stop: enforce_triggers (status) would write PAUSED.
        run = self.new_run("stop", budget=1)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        self.unmark(run)
        self.admit(run, "stop_reached")
        probe = self.copy_of(run, "stop-probe")
        self.rcall(probe, "status")
        self.assertTrue((probe / "PAUSED").exists(), "fixture is not a would-write case")

        # Missing manifest: recover_episode would relist and rewrite SHA256SUMS.
        run = self.new_run("relist")
        self.enroll(run, "t", byte=1)
        (run / "enrollments" / "t" / "SHA256SUMS").unlink()
        self.admit(run, "unknown")
        probe = self.copy_of(run, "relist-probe")
        self.rcall(probe, "status")
        self.assertTrue((probe / "enrollments" / "t" / "SHA256SUMS").exists(), "fixture is not a would-write case")

        # Torn manifest tail: repair_manifest_tail would archive a damaged copy and truncate.
        run = self.new_run("torn")
        self.enroll(run, "t", byte=1)
        sums = run / "enrollments" / "t" / "SHA256SUMS"
        sums.write_bytes(sums.read_bytes() + b"abc")
        self.admit(run, "unknown")
        probe = self.copy_of(run, "torn-probe")
        self.rcall(probe, "status")
        self.assertTrue(list((probe / "enrollments" / "t").glob("SHA256SUMS.damaged-*")), "fixture is not a would-write case")

    def test_11_manifest_conditions_where_recovery_acts_are_unknown_and_untouched(self):
        """The conditions the first review found, plus their neighbours. On a copy the old path acts."""
        def no_newline(run):
            sums = run / "enrollments" / "t" / "SHA256SUMS"
            sums.write_bytes(sums.read_bytes().rstrip(b"\n"))

        def vanished(run):
            ghost = run / "enrollments" / "ghost"
            ghost.mkdir()
            (ghost / "SHA256SUMS").write_text("0" * 64 + "  enrollment.json\n")

        def torn_only(run):
            ghost = run / "enrollments" / "ghost"
            ghost.mkdir()
            (ghost / "SHA256SUMS").write_bytes(b"abc")

        def sums_directory(run):
            path = run / "enrollments" / "t" / "SHA256SUMS"
            path.unlink()
            path.mkdir()

        # str.splitlines() in listed() breaks on these; bytes.splitlines() in repair_manifest_tail does not
        # (or keeps the separator), so the old path raises on a manifest the first parser accepts.
        def replaced(old, new, count=-1):
            def damage(run):
                sums = run / "enrollments" / "t" / "SHA256SUMS"
                sums.write_bytes(sums.read_bytes().replace(old, new, count))
            return damage

        # name: (damage, code, how recovery acts on the copy: "writes" a damaged copy, or "raises" exit 1)
        cases = {
            "no_newline": (no_newline, "episode_manifest_needs_repair", "writes"),
            "vanished": (vanished, "episode_manifest_without_records", "raises"),
            "torn_only": (torn_only, "episode_manifest_without_records", "writes"),
            "sums_directory": (sums_directory, "episode_manifest_unreadable", "raises"),
            "crlf": (replaced(b"\n", b"\r\n"), "episode_manifest_needs_repair", "raises"),
            "bare_cr": (replaced(b"\n", b"\r"), "episode_manifest_needs_repair", "raises"),
            "bare_cr_join": (replaced(b"\n", b"\r", 1), "episode_manifest_needs_repair", "raises"),
            "formfeed_join": (replaced(b"\n", b"\f", 1), "episode_manifest_needs_repair", "raises"),
            "line_separator_join": (replaced(b"\n", "\u2028".encode(), 1), "episode_manifest_needs_repair", "raises"),
        }
        two_entries = {"crlf", "bare_cr", "bare_cr_join", "formfeed_join", "line_separator_join"}  # Needs a review record: 2 lines.
        for name, (damage, code, acts) in cases.items():
            with self.subTest(name):
                run = self.new_run("run-" + name)
                self.enroll(run, "t", byte=1)
                if name in two_entries:
                    self.review(run, "t", p1=0)
                    self.assertEqual((run / "enrollments" / "t" / "SHA256SUMS").read_bytes().count(b"\n"), 2)
                damage(run)
                self.admit(run, "unknown", code)
                probe = self.copy_of(run, "probe-" + name)
                before = snapshot(probe)
                if acts == "writes":
                    self.rcall(probe, "status")
                    self.assertTrue(list(probe.glob("enrollments/*/SHA256SUMS.damaged-*")), "recovery did not write")
                    self.assertNotEqual(before, snapshot(probe))
                else:
                    with self.assertRaises((rec.IntegrityFailure, OSError)):
                        rec.enrollments(str(probe))

    def test_11_the_two_reviewed_conditions_make_enroll_act_too(self):
        """enroll takes the same path as status: it repairs the no-newline manifest and raises on vanished records."""
        run = self.new_run("nonewline")
        self.enroll(run, "t", byte=1)
        sums = run / "enrollments" / "t" / "SHA256SUMS"
        sums.write_bytes(sums.read_bytes().rstrip(b"\n"))
        probe = self.copy_of(run, "nonewline-probe")
        self.enroll(probe, "u", byte=2)
        self.assertTrue(list(probe.glob("enrollments/t/SHA256SUMS.damaged-*")), "enroll did not repair")
        self.assertTrue((probe / "enrollments" / "t" / "SHA256SUMS").read_bytes().endswith(b"\n"))

        run = self.new_run("vanished")
        self.enroll(run, "t", byte=1)
        ghost = run / "enrollments" / "ghost"
        ghost.mkdir()
        (ghost / "SHA256SUMS").write_text("0" * 64 + "  enrollment.json\n")
        probe = self.copy_of(run, "vanished-probe")
        with self.assertRaises(rec.IntegrityFailure):
            rec.enrollments(str(probe))
        self.rcall(probe, "status", code=1)

    def test_11_directories_recovery_neither_writes_nor_raises_stay_skipped(self):
        run = self.new_run("skipped")
        self.enroll(run, "t", byte=1)
        (run / "enrollments" / "empty").mkdir()
        silent = run / "enrollments" / "silent"
        silent.mkdir()
        (silent / "SHA256SUMS").write_bytes(b"")
        (silent / "subdir").mkdir()
        self.admit(run, "active")
        probe = self.copy_of(run, "skipped-probe")
        before = snapshot(probe)
        self.assertEqual([os.path.basename(d) for d, _ in rec.enrollments(str(probe))], ["t"])
        self.assertEqual(before, snapshot(probe), "recovery acted on a directory the gate skips")

    def test_11_manifest_truncation_sweep_gate_is_unknown_exactly_where_recovery_acts(self):
        """Truncate a real manifest at every byte offset and compare the gate with the old path on a copy."""
        run = self.new_run("sweep")
        self.enroll(run, "t", byte=1)
        self.review(run, "t", p1=0)
        raw = (run / "enrollments" / "t" / "SHA256SUMS").read_bytes()
        acts_count = quiet_count = 0
        for k in range(len(raw) + 1):
            probe = self.copy_of(run, f"sweep-{k}")
            (probe / "enrollments" / "t" / "SHA256SUMS").write_bytes(raw[:k])
            gate = rec.admission_decision(str(probe))  # Read-only, so run it before the old path touches the copy.
            before = snapshot(probe)
            try:
                rec.enrollments(str(probe))
                acted = before != snapshot(probe)
            except rec.IntegrityFailure:
                acted = True
            self.assertEqual(gate["state"] == "unknown", acted, (k, gate["reason"], acted))
            acts_count += acted
            quiet_count += not acted
            shutil.rmtree(probe)
        self.assertEqual(quiet_count, 1)  # Only the whole manifest needs no recovery.
        self.assertEqual(acts_count, len(raw))

    def test_11_every_state_leaves_a_run_directory_with_no_lock_file(self):
        for name, build in {
            "active": lambda r: self.enroll(r, "a", 1),
            "paused": lambda r: self.rcall(r, "pause", "--reason", "x"),
            "cap": lambda r: (self.enroll(r, "a"), self.enroll(r, "b")),
        }.items():
            with self.subTest(name):
                run = self.new_run("run-" + name, cap=2 if name == "cap" else 20)
                build(run)
                self.assertFalse((run / ".round.lock").exists())
                self.admit(run, {"active": "active", "paused": "paused", "cap": "stop_reached"}[name])
                self.assertFalse((run / ".round.lock").exists())

    def test_11_readonly_run_directory_still_decides(self):
        run = self.new_run()
        self.enroll(run, "t", byte=1)
        shutil.copytree(run, self.root / "ro")
        ro = self.root / "ro"
        for base, dirs, files in os.walk(ro):
            for name in files:
                os.chmod(os.path.join(base, name), 0o444)
        for base, dirs, files in os.walk(ro, topdown=False):
            os.chmod(base, 0o555)
        self.addCleanup(lambda: [os.chmod(b, 0o755) for b, _, _ in os.walk(ro)])
        self.admit(ro, "active")

    def test_11_call_path_reaches_no_forbidden_name_and_no_write(self):
        source = Path(SCRIPT).read_text()
        tree = ast.parse(source)
        functions = {n.name: n for n in tree.body if isinstance(n, ast.FunctionDef)}
        reached, pending = set(), ["cmd_admission"]
        while pending:
            name = pending.pop()
            if name in reached:
                continue
            reached.add(name)
            for node in ast.walk(functions[name]):
                if isinstance(node, ast.Name) and node.id in functions:
                    pending.append(node.id)
        forbidden_functions = {
            "enrollments", "recover_episode", "repair_manifest_tail", "durable_sums", "trigger_facts", "terra_repairs",
            "enforce_triggers", "pause_round", "atomic_publish", "fsync_directory", "episode_write", "write_once",
            "append_event", "append_sums", "screening", "round_lock", "run_round_command", "cmd_recover_screening",
            "cmd_init_round", "cmd_enroll", "cmd_withdraw", "cmd_review", "cmd_repair", "cmd_close", "cmd_pause",
            "cmd_resume", "cmd_status", "cmd_profile", "cmd_assign", "cmd_attribute", "cmd_verify"}
        forbidden_attributes = {"makedirs", "mkdir", "unlink", "remove", "rename", "replace", "fsync", "write_text",
                                "write_bytes", "mkstemp", "link", "symlink", "truncate", "flock", "fdopen", "rmtree",
                                "copy", "copyfile", "move", "write", "writelines", "touch"}
        self.assertEqual(reached & forbidden_functions, set())
        self.assertIn("terra_stop_rule", reached)  # The stop rule stays the one shared function.
        # The lenient shared readers swallow listing errors and accept any shape: admission has strict
        # equivalents and reaches none of them (their differential tests are in AdmissionClassRepair).
        lenient = {"episode_trigger_facts", "pause_coverage", "terminal"}
        self.assertEqual(reached & lenient, set())
        self.assertIn("admission_trigger_facts", reached)
        self.assertIn("admission_pause_coverage", reached)
        for name in reached:
            for node in ast.walk(functions[name]):
                if isinstance(node, ast.Attribute):
                    self.assertNotIn(node.attr, forbidden_attributes, f"{name}: .{node.attr}")
                    if isinstance(node.value, ast.Name):  # glob swallows OSError.
                        self.assertNotEqual(node.value.id, "glob", f"{name}: glob")
                if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "open":
                    mode = node.args[1] if len(node.args) > 1 else next((k.value for k in node.keywords if k.arg == "mode"), None)
                    if mode is not None:
                        self.assertIsInstance(mode, ast.Constant)
                        self.assertEqual(set(mode.value) & set("wax+"), set(), f"{name}: open mode {mode.value!r}")
                if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "open" \
                        and isinstance(node.func.value, ast.Name) and node.func.value.id == "os":
                    flags = {n.attr for n in ast.walk(node.args[1]) if isinstance(n, ast.Attribute)} if len(node.args) > 1 else set()
                    self.assertTrue(flags and flags <= {"O_RDONLY", "O_NONBLOCK", "O_CLOEXEC", "O_DIRECTORY"},
                                    f"{name}: os.open flags {flags}")
        self.assertNotIn("admission", ROUND_DISPATCH_NAMES(tree))

    def test_11_fail_closed_states_are_untouched(self):
        """Every unlistable or malformed state of AdmissionFailsClosed is decided without writing, and is unknown."""
        for name, (build, code) in self.fail_closed_cases().items():
            with self.subTest(name):
                run = build(name)
                self.admit(run, "unknown", code)  # admit() compares full snapshots and checks for a lock file.

    def test_11_snapshot_helper_detects_a_change(self):
        run = self.new_run()
        before = snapshot(run)
        (run / "extra").write_text("x")
        self.assertNotEqual(before, snapshot(run))
        before = snapshot(run)
        os.utime(run / "round.json", ns=(1, 1))
        self.assertNotEqual(before, snapshot(run))


def ROUND_DISPATCH_NAMES(tree):
    """Names of commands dispatched through run_round_command: the ROUND_COMMANDS set."""
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "ROUND_COMMANDS" for t in node.targets):
            return {e.value for e in node.value.elts}
    raise AssertionError("ROUND_COMMANDS not found")


class AdmissionFailsClosed(AdmissionBase):
    """AR-1 defects: an unlistable input and a malformed pause history must be unknown, never active.

    The two reproduced fail-open defects are the first and the dict/bool/float cases here; the others harden
    the same rule on every listing or existence test the gate relies on.
    """

    def test_every_unlistable_or_malformed_state_is_unknown_with_its_reason_code(self):
        for name, (build, code) in self.fail_closed_cases().items():
            with self.subTest(name):
                self.admit(build(name), "unknown", code)

    def test_unreadable_enrollments_directory_is_not_active(self):
        run = self.with_enrollment("unreadable")
        self.chmod_for_test(run / "enrollments", 0o000)
        out = self.admit(run, "unknown", "enrollments_unlistable")
        self.assertFalse(out["facts"]["enrollments"]["evaluated"])

    def test_enrollments_as_a_regular_file_is_unknown(self):
        run = self.new_run()
        (run / "enrollments").write_text("x")
        self.admit(run, "unknown", "enrollments_unlistable")

    def test_absent_enrollments_directory_still_means_no_enrollments(self):
        run = self.new_run()
        self.assertFalse((run / "enrollments").exists())
        out = self.admit(run, "active", "admitted")
        self.assertEqual(out["facts"]["enrollments"], {"evaluated": True, "count": 0, "cap": 20})
        # The same with an empty directory, and with a hidden entry that enroll ignores as well.
        (run / "enrollments").mkdir()
        (run / "enrollments" / ".hidden").mkdir()
        self.assertEqual(self.admit(run, "active")["facts"]["enrollments"]["count"], 0)

    def test_absent_pause_history_still_means_nothing_is_covered(self):
        run = self.p1_unmarked("no-history")
        self.assertFalse((run / "pauses").exists())
        out = self.admit(run, "stop_reached", "pause_trigger_unmarked")
        self.assertEqual(out["facts"]["trigger"]["covered_p1_episodes"], [])
        # An empty directory, a non-JSON file and a hidden JSON file are not history, as pause_coverage's glob sees it.
        (run / "pauses").mkdir()
        (run / "pauses" / "notes.txt").write_text("not json")
        (run / "pauses" / ".hidden.json").write_text("[")
        self.admit(run, "stop_reached", "pause_trigger_unmarked")

    def test_a_malformed_coverage_would_have_suppressed_a_real_stop_on_the_old_path(self):
        """The reproduced defect, on a synthetic copy: pause_coverage covers task-p1 from a dict of falses."""
        run = self.p1_unmarked("suppressed")
        self.admit(run, "stop_reached", "pause_trigger_unmarked")  # The stop is real without any history.
        self.write_pause_history(run, {"covered_triggers": {"p1_episodes": {"task-p1": False}}})
        covered, _ = rec.pause_coverage(str(run))
        self.assertEqual(covered, {"task-p1"})  # What the unchanged consumer does with it.
        out = self.admit(run, "unknown", "pause_history_malformed")
        self.assertIn("x.json", out["reason"]["text"])
        self.assertFalse(out["facts"]["trigger"]["evaluated"])

    def test_bool_and_float_rounds_would_have_suppressed_a_repair_round_stop(self):
        for value in (True, 5.0):
            with self.subTest(repr(value)):
                run = self.rounds_unmarked(f"rounds-{value}")
                self.admit(run, "stop_reached", "pause_trigger_unmarked")
                self.write_pause_history(run, {"covered_triggers": {"terra_repair_rounds": value}})
                _, covered_rounds = rec.pause_coverage(str(run))
                self.assertEqual(covered_rounds, value)  # The consumer accepts it and the stop is suppressed.
                self.admit(run, "unknown", "pause_history_malformed")

    def test_well_formed_histories_keep_their_meaning(self):
        run = self.p1_unmarked("good")
        for document in ({}, {"covered_triggers": {}}, {"reason": "x"}, {"covered_triggers": {"p1_episodes": []}},
                         {"covered_triggers": {"terra_repair_rounds": 0}},
                         {"covered_triggers": {"p1_episodes": ["other"], "terra_repair_rounds": 0, "extra": [1, {}]}}):
            with self.subTest(document):
                self.write_pause_history(run, document)
                self.admit(run, "stop_reached", "pause_trigger_unmarked")
        self.write_pause_history(run, {"covered_triggers": {"p1_episodes": ["task-p1"], "terra_repair_rounds": 0}})
        out = self.admit(run, "active")
        self.assertEqual(out["facts"]["trigger"]["covered_p1_episodes"], ["task-p1"])

    def test_real_resume_history_validates_and_agrees_with_enroll(self):
        run = self.new_run(budget=2)
        self.enroll(run, "t", byte=1)
        self.review(run, "t", p1=1)
        self.repair(run, "t")
        self.repair(run, "t")
        self.resume(run)  # Writes a real pauses/*.json with covered_triggers from enforce_triggers.
        self.assertTrue(list((run / "pauses").glob("*.json")))
        self.admit(run, "active")

    def test_the_gate_names_the_file_it_rejected(self):
        run = self.p1_unmarked("named")
        self.write_pause_history(run, {"covered_triggers": []}, "2026-11-07T00-00-00Z.json")
        out = self.admit(run, "unknown", "pause_history_malformed")
        self.assertIn("2026-11-07T00-00-00Z.json", out["reason"]["text"])

    def test_enroll_and_enforcement_semantics_are_unchanged(self):
        """The repair is on admission's path only: the shared consumers still behave as before."""
        run = self.p1_unmarked("unchanged")
        self.write_pause_history(run, {"covered_triggers": {"p1_episodes": {"task-p1": False}}})
        probe = self.copy_of(run, "unchanged-probe")
        out = self.rcall(probe, "status")  # enforce_triggers still sees the dict as coverage.
        self.assertFalse(out["paused"])
        self.assertEqual(rec.enrollment_dirs(str(self.root / "no-such-run")), [])
        os.chmod(run / "enrollments", 0o000)
        try:
            self.assertEqual(rec.enrollment_dirs(str(run)), [])  # enrollment_dirs still swallows the error.
        finally:
            os.chmod(run / "enrollments", 0o755)


class AdmissionAgreesWithEnroll(AdmissionBase):
    """Case 12. On copies of the case 1, 6 and 7 fixtures, enroll --eligible yes agrees."""

    def enroll_copy(self, run, name):
        probe = self.copy_of(run, name)
        with mock.patch.object(rec.os, "urandom", return_value=bytes([0])):
            return probe, self.rcall(probe, "enroll", "--task-id", "next", "--task-revision", "next@" + "a" * 16,
                                     "--eligible", "yes", "--reason", "agreement probe")

    def test_12_active_agrees_with_enrolled(self):
        run = self.new_run()
        self.enroll(run, "a", byte=1)
        self.admit(run, "active")
        _, out = self.enroll_copy(run, "probe")
        self.assertEqual(out["outcome"], "enrolled")

    def test_12_unmarked_stop_agrees_with_paused_after_enroll_writes_the_marker(self):
        run = self.new_run(budget=1)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        self.unmark(run)
        self.admit(run, "stop_reached", "pause_trigger_unmarked")
        probe = self.copy_of(run, "probe")
        self.assertFalse((probe / "PAUSED").exists())
        with mock.patch.object(rec.os, "urandom", return_value=bytes([0])):
            out = self.rcall(probe, "enroll", "--task-id", "next", "--task-revision", "next@" + "a" * 16,
                             "--eligible", "yes", "--reason", "agreement probe")
        self.assertEqual(out["outcome"], "paused")
        self.assertTrue((probe / "PAUSED").exists())
        self.admit(probe, "paused")  # The same facts, once enroll has marked them.

    def test_12_p1_unmarked_stop_agrees_with_paused(self):
        run = self.new_run()
        self.enroll(run, "t", byte=1)
        self.review(run, "t", p1=1)
        self.unmark(run)
        self.admit(run, "stop_reached")
        _, out = self.enroll_copy(run, "probe")
        self.assertEqual(out["outcome"], "paused")

    def test_12_cap_agrees_with_cap_reached(self):
        run = self.new_run(cap=2, in_flight=5)
        self.enroll(run, "a")
        self.enroll(run, "b")
        self.admit(run, "stop_reached", "cap_reached")
        _, out = self.enroll_copy(run, "probe")
        self.assertEqual(out["outcome"], "cap_reached")

    def test_12_covered_facts_agree_with_enrolled(self):
        run = self.new_run(budget=1)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        self.resume(run)
        self.admit(run, "active")
        _, out = self.enroll_copy(run, "probe")
        self.assertEqual(out["outcome"], "enrolled")


class SharedRule(AdmissionBase):
    """The stop rule and trigger facts have one definition that enforce_triggers and admission share."""

    def test_rule_is_pure_and_exact_at_the_boundaries(self):
        facts = {"p1_episodes": ["a", "b"], "terra_repair_rounds": 3}
        self.assertEqual(rec.terra_stop_rule(facts, {"a", "b"}, 3, 3), ([], False))
        self.assertEqual(rec.terra_stop_rule(facts, {"a"}, 3, 3), (["b"], False))
        self.assertEqual(rec.terra_stop_rule(facts, {"a", "b"}, 2, 3), ([], True))   # At budget, above covered.
        self.assertEqual(rec.terra_stop_rule(facts, {"a", "b"}, 2, 4), ([], False))  # Below budget.
        self.assertEqual(rec.terra_stop_rule(facts, {"a", "b"}, -1, 3), ([], True))  # Nothing covered yet.

    def test_enforce_triggers_and_admission_agree_on_a_sweep_of_states(self):
        for budget in (1, 2, 3):
            for rounds in (0, 1, 2, 3):
                for p1 in (0, 1):
                    for covered in ("none", "all"):
                        name = f"b{budget}r{rounds}p{p1}{covered}"
                        with self.subTest(name):
                            run = self.new_run(name, budget=100)  # Build the facts without triggering.
                            self.enroll(run, "t", byte=1)
                            if p1:
                                self.review(run, "t", p1=1)
                            for _ in range(rounds):
                                self.repair(run, "t")
                            if (run / "PAUSED").exists():
                                (run / "PAUSED").unlink()  # The P1 review wrote it; the sweep wants the unmarked state.
                            config = json.loads((run / "round.json").read_text())
                            config["terra_repair_budget"] = budget
                            (run / "round.json").write_text(json.dumps(config))
                            if covered == "all":
                                (run / "pauses").mkdir()
                                (run / "pauses" / "c.json").write_text(json.dumps({"covered_triggers": rec.trigger_facts(str(run))}))
                            out = rec.admission_decision(str(run))
                            probe = self.copy_of(run, name + "-probe")
                            rec.enforce_triggers(str(probe))
                            marked = (probe / "PAUSED").exists()
                            self.assertEqual(out["state"] == "stop_reached", marked, out)
                            self.assertEqual(out["facts"]["trigger"]["p1_episodes"], rec.trigger_facts(str(run))["p1_episodes"])
                            self.assertEqual(out["facts"]["trigger"]["terra_repair_rounds"], rec.terra_repairs(str(run)))

    def test_trigger_facts_and_episode_trigger_facts_do_not_drift(self):
        run = self.new_run(budget=100, in_flight=5)
        self.enroll(run, "t", byte=1)
        self.enroll(run, "u", byte=1)
        self.enroll(run, "s", byte=0)
        self.review(run, "t", p1=1)
        self.review(run, "s", p1=1)
        self.repair(run, "u")
        self.repair(run, "s")
        episodes = rec.enrollments(str(run))
        self.assertEqual(rec.trigger_facts(str(run)), rec.episode_trigger_facts(episodes))
        self.assertEqual(rec.episode_trigger_facts(episodes), {"p1_episodes": ["t"], "terra_repair_rounds": 1})
        self.assertEqual(rec.terra_repairs(str(run)), 1)


class AdmissionClassRepair(AdmissionBase):
    """The whole fail-open class: the gate returns active only when every fact it needs was read and valid.

    Black-box: each test runs `admission` (as a process, or in process with injected faults) on a fixture
    directory, so the same tests run against any version of the recorder. See input-inventory.md for the
    row each test covers (IDs in the docstrings).
    """

    # --- fault injection ---------------------------------------------------------------------------------
    @contextlib.contextmanager
    def fs_fault(self, index=None):
        """Count the filesystem calls of the block; raise OSError(EIO) on call number `index` when given.

        Patches the low-level calls, so the os.path predicates and glob, which swallow OSError, see the fault
        exactly as they would a real I/O error, and also the predicates and glob themselves, so a fault raised
        at that level is covered too (a call is counted at each level it passes). Yields (call, path) seen."""
        calls = []

        def wrap(name, original):
            def call(*args, **kwargs):
                n = len(calls)
                calls.append((name, args[0] if args else None))
                if n == index:
                    raise OSError(errno.EIO, "injected I/O error", str(args[0]) if args else None)
                return original(*args, **kwargs)
            return call

        with contextlib.ExitStack() as stack:
            for owner, name in ((os, "stat"), (os, "lstat"), (os, "listdir"), (os, "scandir"), (os, "open"),
                                (os, "fstat"), (os, "read"), (builtins, "open"), (os.path, "exists"),
                                (os.path, "isdir"), (os.path, "isfile"), (os.path, "islink"), (os.path, "lexists"),
                                (glob, "glob")):
                stack.enter_context(mock.patch.object(owner, name, wrap(name, getattr(owner, name))))
            yield calls

    def decide(self, run, index=None):
        with self.fs_fault(index) as calls:
            out = rec.admission_decision(str(run))
        return out, calls

    def eio_sweep(self, run, label):
        """Inject EIO into each filesystem call of one decision in turn. No run may be active (unless the
        un-injected decision is active and the call is one the decision does not need: there are none),
        and none may write."""
        base, calls = self.decide(run)
        before = snapshot(run)
        self.assertGreater(len(calls), 5, label)
        for index, (name, path) in enumerate(calls):
            out, _ = self.decide(run, index)
            self.assertNotEqual(out["state"], "active",
                                f"{label}: EIO in call {index} ({name} {path}) still gave active ({out['reason']})")
        self.assertEqual(snapshot(run), before, f"{label}: a faulted decision wrote to the run directory")
        return base

    def sweep_fixtures(self):
        def active_full(name):
            run = self.new_run(name, budget=3, in_flight=5)
            self.enroll(run, "t", byte=1)
            self.review(run, "t", p1=0)
            self.repair(run, "t")
            self.enroll(run, "s", byte=0)
            self.review(run, "s", p1=0)
            self.write_pause_history(run, {"covered_triggers": {"p1_episodes": ["old"], "terra_repair_rounds": 0}})
            return run

        def covered_p1(name):
            run = self.p1_unmarked(name)
            self.write_pause_history(run, {"covered_triggers": {"p1_episodes": ["task-p1"], "terra_repair_rounds": 0}})
            return run

        def paused(name):
            run = self.with_enrollment(name)
            self.rcall(run, "pause", "--reason", "synthetic")
            return run

        def capped(name):
            run = self.new_run(name, cap=1)
            self.enroll(run, "t", byte=1)
            return run

        return {"active_full": (active_full, "active"), "covered_p1": (covered_p1, "active"),
                "p1_stop": (self.p1_unmarked, "stop_reached"), "repair_stop": (self.rounds_unmarked, "stop_reached"),
                "paused": (paused, "paused"), "cap": (capped, "stop_reached")}

    # --- D1: the repair scan ------------------------------------------------------------------------------
    def test_d1_eio_in_the_repair_scan_is_not_active(self):
        """B3/A13. A real stop (one repair round, budget one). The episode directory is listed once without
        error; every later scan (glob uses os.scandir) fails. The old gate counted zero repairs: active."""
        run = self.rounds_unmarked("d1")
        self.assertEqual(self.decide(run)[0]["state"], "stop_reached")
        episode = str(run / "enrollments" / "t")
        real = os.scandir

        def failing(path, *args, **kwargs):
            if str(path) == episode:
                raise OSError(errno.EIO, "injected", episode)
            return real(path, *args, **kwargs)

        with mock.patch.object(os, "scandir", failing):
            out = rec.admission_decision(str(run))
        self.assertNotEqual(out["state"], "active", out)

    def test_d1_eio_on_any_call_never_turns_a_stop_or_a_pause_into_active(self):
        for label, (build, expected) in self.sweep_fixtures().items():
            with self.subTest(label):
                run = build("sweep-" + label)
                base = self.eio_sweep(run, label)
                self.assertEqual(base["state"], expected, base)

    # --- D2: counts ---------------------------------------------------------------------------------------
    def test_d2_p1_must_be_a_non_negative_int(self):
        """A12. Sealed, hash-consistent first reviews with an invalid p1 on a Terra episode: unknown."""
        cases = self.fail_closed_cases()
        names = [n for n in cases if n.startswith("review_p1_")]
        self.assertEqual(len(names), 12)
        for name in names:
            with self.subTest(name):
                build, code = cases[name]
                self.admit(build(name), "unknown", code)

    def test_d2_valid_p1_counts_keep_their_meaning(self):
        for p1, state in ((0, "active"), (1, "stop_reached"), (2, "stop_reached"), (10 ** 30, "stop_reached")):
            with self.subTest(p1):
                run = self.terra_with_review(f"p1-{p1}", p1=p1)
                self.admit(run, state)

    def test_d2_a_malformed_p1_on_a_sol_episode_is_not_consumed(self):
        """Sol first reviews are not read by the stop rule, so they add no rule here (they are still sealed)."""
        run = self.new_run()
        self.enroll(run, "s", byte=0)
        self.review(run, "s", p1=1)
        self.rechain(run, "s", {"review-first.json": lambda data: data.update(p1=-1)})
        self.admit(run, "active")

    # --- D3: non-regular files ----------------------------------------------------------------------------
    def test_d3_a_fifo_or_dangling_link_where_a_record_is_expected_is_unknown_without_a_hang(self):
        wanted = ("fifo_pause_history_entry", "dangling_pause_history_entry", "fifo_round_json", "fifo_manifest",
                  "fifo_review_record", "dangling_review_record", "fifo_enrollment_record",
                  "dangling_enrollment_record", "dangling_enrollments_entry", "repair_record_is_a_directory",
                  "close_record_is_a_directory", "repair_fifo", "repair_dangling")
        cases = self.fail_closed_cases()
        for name in wanted:
            with self.subTest(name):
                build, code = cases[name]
                self.admit(build(name), "unknown", code)

    def test_d3_a_fifo_paused_marker_is_paused_and_is_never_opened(self):
        run = self.new_run()
        os.mkfifo(run / "PAUSED")
        self.admit(run, "paused", "pause_marker_present")

    # --- consistency of the evidence ----------------------------------------------------------------------
    def test_conflicting_or_duplicated_records_are_unknown(self):
        cases = self.fail_closed_cases()
        wanted = [n for n in cases if n.startswith(("repair_", "manifest_duplicate", "enrollment_task_id",
                                                    "review_task_id")) and not n.startswith(("repair_fifo", "repair_dangling",
                                                                                              "repair_record_is"))]
        self.assertGreaterEqual(len(wanted), 9)
        for name in wanted:
            with self.subTest(name):
                build, code = cases[name]
                self.admit(build(name), "unknown", code)

    def test_repair_records_numbered_other_than_one_to_n_are_unknown_naming_the_episode(self):
        cases = self.fail_closed_cases()
        for name in ("repair_numbering_starts_at_two", "repair_numbering_skips_a_number"):
            with self.subTest(name):
                build, code = cases[name]
                out = self.admit(build(name), "unknown", code)
                self.assertIn("t", out["reason"]["text"])
                self.assertIn("repair", out["reason"]["text"])

    def test_contiguous_repair_records_still_agree_with_the_shared_rule(self):
        run = self.new_run("contiguous", budget=100, in_flight=9)
        self.enroll(run, "t", byte=1)
        self.repair(run, "t")
        self.repair(run, "t")
        self.repair(run, "t")
        (run / "PAUSED").unlink(missing_ok=True)
        out = rec.admission_decision(str(run))
        self.assertIn(out["state"], ("stop_reached", "active"), out)
        self.assertEqual(out["facts"]["trigger"]["terra_repair_rounds"], rec.trigger_facts(str(run))["terra_repair_rounds"])
        self.assertEqual(out["facts"]["trigger"]["terra_repair_rounds"], 3)

    def test_unsealed_files_with_ambiguous_or_invalid_values_are_unknown(self):
        cases = self.fail_closed_cases()
        wanted = [n for n in cases if n.startswith(("round_", "pause_"))
                  and n not in ("pause_entry_is_a_directory", "pauses_unreadable", "pauses_regular_file",
                                "pauses_dangling_symlink", "pause_history_not_object", "pause_history_wrong_shape")]
        self.assertGreaterEqual(len(wanted), 12)
        for name in wanted:
            with self.subTest(name):
                build, code = cases[name]
                self.admit(build(name), "unknown", code)

    # --- mutation of every consumed JSON field ----------------------------------------------------------
    BAD = [None, "x", "", -1, -2 ** 70, True, False, 1.5, 0.0, 1.0, float("nan"), float("inf"), float("-inf"), [], [1], {}, {"a": 1}]

    def mutate_and_decide(self, baseline, label, edit_file, field, values, valid, expected_baseline, absent_is_valid=False):
        """For each value (and the absent field) not in `valid`, copy the baseline, write the value through
        edit_file(copy, field, value) and require unknown."""
        base = rec.admission_decision(str(baseline))
        self.assertEqual(base["state"], expected_baseline, f"{label}: fixture baseline {base}")
        missing = object()
        for index, value in enumerate(list(values) + [missing]):
            if (value is missing and absent_is_valid) or (value is not missing and valid(value)):
                continue
            copy = self.copy_of(baseline, f"{label}-{index}")
            edit_file(copy, field, value, missing)
            out = rec.admission_decision(str(copy))
            self.assertEqual(out["state"], "unknown", f"{label}: {field}={value!r} gave {out['state']} ({out['reason']})")

    def test_every_consumed_json_field_rejects_malformed_values(self):
        def is_count(value):
            return type(value) is int and value >= 0

        def json_edit(name):
            def edit(run, field, value, missing):
                path = run / name
                data = json.loads(path.read_text())
                if value is missing:
                    data.pop(field, None)
                else:
                    data[field] = value
                path.write_text(json.dumps(data))
            return edit

        def pause_edit(inner):
            def edit(run, field, value, missing):
                path = run / "pauses" / "x.json"
                data = json.loads(path.read_text())
                if inner:
                    target = data.setdefault("covered_triggers", {})
                    if value is missing:
                        target.pop(field, None)
                    else:
                        target[field] = value
                elif value is missing:
                    data.pop(field, None)
                else:
                    data[field] = value
                path.write_text(json.dumps(data))
            return edit

        def record_edit(task, name):
            def edit(run, field, value, missing):
                def apply(data):
                    if value is missing:
                        data.pop(field, None)
                    else:
                        data[field] = value
                self.rechain(run, task, {name: apply})
            return edit

        sweeps = self.sweep_fixtures()
        active = sweeps["active_full"][0]("mut-active")
        covered = sweeps["covered_p1"][0]("mut-covered")
        p1_stop = self.p1_unmarked("mut-p1")
        repair_stop = self.rounds_unmarked("mut-rep")
        sol = self.new_run("mut-sol")
        self.enroll(sol, "s", byte=0)
        for field in ("cap", "terra_in_flight", "terra_repair_budget"):
            self.mutate_and_decide(active, f"round-{field}", json_edit("round.json"), field, self.BAD, is_count, "active")
        # Replacing a count with a valid int is the owner's edit; only the invalid values are the class.
        self.mutate_and_decide(p1_stop, "review-p1", record_edit("task-p1", "review-first.json"), "p1", self.BAD, is_count,
                               "stop_reached")
        self.mutate_and_decide(p1_stop, "review-task", record_edit("task-p1", "review-first.json"), "task_id",
                               self.BAD + ["other"], lambda v: v == "task-p1", "stop_reached")
        self.mutate_and_decide(p1_stop, "enrollment-arm", record_edit("task-p1", "enrollment.json"), "arm", self.BAD + ["other"],
                               lambda v: v in (rec.TERRA, rec.SOL), "stop_reached")
        self.mutate_and_decide(p1_stop, "enrollment-task", record_edit("task-p1", "enrollment.json"), "task_id",
                               self.BAD + ["other"], lambda v: v == "task-p1", "stop_reached")
        self.mutate_and_decide(repair_stop, "repair-round", record_edit("t", "repair-01.json"), "round", self.BAD + [2, 0],
                               lambda v: type(v) is int and v == 1, "stop_reached")
        self.mutate_and_decide(repair_stop, "repair-task", record_edit("t", "repair-01.json"), "task_id", self.BAD + ["other"],
                               lambda v: v == "t", "stop_reached")
        self.mutate_and_decide(covered, "pause-covered", pause_edit(False), "covered_triggers", self.BAD,
                               lambda v: isinstance(v, dict), "active", absent_is_valid=True)
        self.mutate_and_decide(covered, "pause-p1", pause_edit(True), "p1_episodes", self.BAD,
                               lambda v: isinstance(v, list) and all(isinstance(i, str) for i in v), "active", absent_is_valid=True)
        self.mutate_and_decide(covered, "pause-rounds", pause_edit(True), "terra_repair_rounds", self.BAD, is_count, "active", absent_is_valid=True)

    # --- the strict readers give the shared readers' facts on valid input ----------------------------------
    def test_differential_facts_equal_the_shared_readers_on_valid_fixtures(self):
        fixtures = []
        run = self.new_run("diff-a", budget=100, in_flight=9)
        for task, byte in (("t", 1), ("u", 1), ("v", 1), ("s", 0)):
            self.enroll(run, task, byte=byte)
        self.review(run, "t", p1=1)
        self.review(run, "u", p1=0)
        self.review(run, "s", p1=3)
        self.repair(run, "t")
        self.repair(run, "t")
        self.repair(run, "u")
        self.repair(run, "s")
        (run / "PAUSED").unlink()
        fixtures.append(run)
        resumed = self.copy_of(run, "diff-b")
        (resumed / "pauses").mkdir()
        (resumed / "pauses" / "a.json").write_text(json.dumps({"covered_triggers": {"p1_episodes": ["t"], "terra_repair_rounds": 2}}))
        (resumed / "pauses" / "b.json").write_text(json.dumps({"covered_triggers": {"p1_episodes": ["x"], "terra_repair_rounds": 0}}))
        (resumed / "pauses" / "ignored.txt").write_text("[")
        fixtures.append(resumed)
        linked = self.copy_of(run, "diff-c")  # A symlinked review is followed, as before.
        d = linked / "enrollments" / "t"
        shutil.copyfile(d / "review-first.json", self.root / "review-elsewhere.json")
        os.unlink(d / "review-first.json")
        os.symlink(str(self.root / "review-elsewhere.json"), str(d / "review-first.json"))
        fixtures.append(linked)
        for fixture in fixtures:
            with self.subTest(fixture.name):
                out = rec.admission_decision(str(fixture))
                self.assertIn(out["state"], ("stop_reached", "active"), out)
                facts = out["facts"]["trigger"]
                shared = rec.trigger_facts(str(fixture))
                self.assertEqual(facts["p1_episodes"], shared["p1_episodes"])
                self.assertEqual(facts["terra_repair_rounds"], shared["terra_repair_rounds"])
                covered, rounds = rec.pause_coverage(str(fixture))
                self.assertEqual(facts["covered_p1_episodes"], sorted(covered))
                self.assertEqual(facts["covered_repair_rounds"], rounds)
                self.assertEqual(facts["terra_repair_rounds"], rec.terra_repairs(str(fixture)))
                numbered = sorted(n for n in os.listdir(fixture / "enrollments" / "t") if n.startswith("repair-"))
                self.assertEqual(numbered, ["repair-01.json", "repair-02.json"])  # contiguous 1..n is valid input
                self.assertEqual(out["facts"]["enrollments"]["count"], len(rec.enrollment_dirs(str(fixture))))
                new_p1, over = rec.terra_stop_rule(shared, covered, rounds, out["facts"]["round"]["terra_repair_budget"])
                self.assertEqual(out["state"] == "stop_reached" and out["reason"]["code"] == "pause_trigger_unmarked",
                                 bool(new_p1 or over))

    def test_open_terra_fact_counts_closed_and_withdrawn_episodes_as_before(self):
        run = self.new_run(in_flight=9)
        self.enroll(run, "t", byte=1)
        self.enroll(run, "u", byte=1)
        self.enroll(run, "v", byte=1)
        self.rcall(run, "withdraw", "--task-id", "u", "--reason", "synthetic")
        out = self.admit(run, "active")
        episodes = rec.enrollments(str(run))
        expected = sum(e["arm"] == rec.TERRA and not rec.terminal(d) for d, e in episodes)
        self.assertEqual(out["facts"]["terra_in_flight"]["open_terra"], expected)
        self.assertEqual(expected, 2)

    def test_gate_reads_each_regular_input_through_a_non_blocking_checked_open(self):
        """A FIFO is not opened blocking even if it replaces a file between the listing and the read."""
        run = self.with_enrollment("race")
        real_open = os.open
        seen = []

        def spying(path, flags, *args, **kwargs):
            seen.append((str(path), flags))
            return real_open(path, flags, *args, **kwargs)

        with mock.patch.object(os, "open", spying):
            out = rec.admission_decision(str(run))
        self.assertEqual(out["state"], "active")
        gate_reads = [flags for path, flags in seen if path.startswith(str(run))]
        self.assertTrue(gate_reads)
        self.assertTrue(all(flags & os.O_NONBLOCK for flags in gate_reads), seen)


if __name__ == "__main__":
    unittest.main()
