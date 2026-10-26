"""Rotation gate and role host against a REAL board and fake host CLIs.

The board is the real chatboard adapter on a scratch repository. The fake
`claude` CLI reproduces the host behaviour measured in
metrics/probes/cli-daemon-host-20261025.json, the fake Codex app-server daemon
the behaviour in metrics/probes/codex-daemon-thread-host-20261026.json, and the
fake successor follows
the gate's own successor prompt, so a test cannot pass on a capability the
real successor lacks.
"""
from __future__ import annotations

import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import uuid
from pathlib import Path

ROOT = Path(__file__).parents[1]
REPO_ROOT = ROOT.parents[1]
FAKES = Path(__file__).parent / "role_host_fakes"
GATE = ROOT / "scripts" / "root_rotation.py"
HOST = ROOT / "scripts" / "role_host.py"
BOARD = REPO_ROOT / "skills" / "durable-state" / "scripts" / "codex_chatboard.py"
PASS = f"{sys.executable} -c 'import sys; sys.exit(0)'"


def role_host_rows(state):
    return [json.loads(l) for l in (Path(state) / "ledger.jsonl").read_text().splitlines()]


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="rotation-test-"))
        self.repo = self.tmp / "repo"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        self.state = self.tmp / "gate"
        self.host_state = self.tmp / "host"
        self.codex_home = self.tmp / "codex-home"
        self.codex_home.mkdir()
        self.env = dict(os.environ,
                        PATH=f"{FAKES}:{os.environ['PATH']}",
                        FAKE_CLAUDE_STATE=str(self.tmp / "claude-state.json"),
                        CODEX_HOME=str(self.codex_home),
                        ROLE_HOST_CODEX_SOCKET=str(self.tmp / "daemon.sock"),
                        FAKE_DAEMON_MODE=str(self.tmp / "daemon-mode.json"))
        for k in ("CLAUDE_CODE_SESSION_ID", "CODEX_THREAD_ID", "FAKE_SUCCESSOR", "FAKE_TURN"):
            self.env.pop(k, None)
        # The gate is normally run from inside a host session, so the caller's
        # identity is in the environment. The fakes pass it through, as real
        # processes do; a successor must still learn its OWN id.
        self.caller = str(uuid.uuid4())
        self.env["CLAUDE_CODE_SESSION_ID"] = self.caller
        self.handoff = self.tmp / "handoff.md"
        self.handoff.write_text("# handoff\n\nContinue the role.\n")
        self.quiescent = self.tmp / "quiescent"
        self.quiescent.write_text("no children, no claims\n")
        self.pred = str(uuid.uuid4())
        self.pred_proc = subprocess.Popen(["sleep", "300"])
        self.add_claude_session(self.pred, "operator-shell", kind="interactive", pid=self.pred_proc.pid,
                                status="busy", state="working")
        self.procs = [self.pred_proc]
        # The shared Codex daemon runs independently of any caller session.
        daemon_env = {k: v for k, v in self.env.items() if k != "CLAUDE_CODE_SESSION_ID"}
        self.daemon = subprocess.Popen([sys.executable, str(FAKES / "fake_app_server.py"),
                                        self.env["ROLE_HOST_CODEX_SOCKET"]], env=daemon_env)
        self.procs.append(self.daemon)
        assert self.wait_for(lambda: Path(self.env["ROLE_HOST_CODEX_SOCKET"]).exists(), 10)

    def daemon_mode(self, env):
        mode = {k: v for k, v in (env or {}).items() if k.startswith("FAKE_") and k != "FAKE_CLAUDE_STATE"}
        Path(self.env["FAKE_DAEMON_MODE"]).write_text(json.dumps(mode))

    def tearDown(self):
        for p in self.procs:
            p.kill()
            p.wait()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def add_claude_session(self, sid, name, kind="background", pid=1, status="idle", state="done", loaded=True):
        path = Path(self.env["FAKE_CLAUDE_STATE"])
        db = json.loads(path.read_text()) if path.exists() else {"sessions": {}}
        db["sessions"][sid] = {"name": name, "cwd": str(self.repo), "kind": kind, "pid": pid,
                               "status": status, "state": state, "loaded": loaded}
        path.write_text(json.dumps(db))

    def claude_sessions(self):
        return json.loads(Path(self.env["FAKE_CLAUDE_STATE"]).read_text())["sessions"]

    def gate(self, op, *extra, successor="claude", pred_host="claude", env=None, handoff=True):
        argv = [sys.executable, str(GATE), op] + ([str(self.handoff)] if handoff else []) + [
            "--role", "test-root", "--state", str(self.state), "--repo", str(self.repo),
            "--host-state", str(self.host_state), "--pred-host", pred_host,
            "--pred-session", self.pred, "--successor-host", successor,
            "--validator", PASS, "--quiescence-file", str(self.quiescent),
            "--adopt-timeout", "60", "--after-adopt", "stop", *extra]
        self.daemon_mode(env)
        r = subprocess.run(argv, env={**self.env, **(env or {})}, capture_output=True, text=True, timeout=180)
        return r.returncode, (self.state / "terminal-status").read_text().strip() \
            if (self.state / "terminal-status").exists() else "", r.stderr

    def host(self, *args, env=None):
        self.daemon_mode(env)
        r = subprocess.run([sys.executable, str(HOST), "--state", str(self.host_state), *args],
                           env={**self.env, **(env or {})}, capture_output=True, text=True, timeout=120)
        return r.returncode, r.stdout, r.stderr

    def board_rows(self, **filters):
        r = subprocess.run([sys.executable, str(BOARD), "--repository", str(self.repo), "read", "--limit", "50"],
                           capture_output=True, text=True, check=True)
        rows = json.loads(r.stdout)["messages"]
        return [m for m in rows if all(m.get(k) == v for k, v in filters.items())]

    def wait_for(self, pred, timeout=30):
        end = time.time() + timeout
        while time.time() < end:
            if pred():
                return True
            time.sleep(0.2)
        return False


class GateInputs(Base):
    def test_awaiting_authority_launches_nothing(self):
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (10, "awaiting_authority"))
        self.assertEqual(len(self.claude_sessions()), 1)

    def test_noncanonical_predecessor_refused(self):
        self.pred = self.pred.upper()
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (24, "fail_predecessor_session_form"))

    def test_missing_assertion_is_not_established(self):
        self.quiescent.unlink()
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (32, "quiescence_not_established"))

    def test_validator_unusable_exit_is_not_a_pass(self):
        rc, status, _ = self.gate("prepare", "--validator", f"{sys.executable} -c 'import sys; sys.exit(3)'")
        self.assertEqual((rc, status), (32, "validation_not_established_prepare"))

    def test_role_name_already_loaded_is_ambiguous(self):
        self.add_claude_session(str(uuid.uuid4()), "test-root")
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (39, "fail_role_ambiguous"))

    def test_mutated_handoff_refused_at_commit(self):
        self.assertEqual(self.gate("prepare")[0], 10)
        self.handoff.write_text("# handoff\n\nchanged\n")
        rc, status, _ = self.gate("commit")
        self.assertEqual((rc, status), (30, "fail_handoff_mutated"))

    def test_descendant_session_process_blocks(self):
        self.pred_proc.kill(); self.pred_proc.wait()
        self.pred_proc = subprocess.Popen(["bash", "-c", "(exec -a codex sleep 300) & wait"])
        self.procs.append(self.pred_proc)
        self.add_claude_session(self.pred, "operator-shell", kind="interactive", pid=self.pred_proc.pid,
                                status="busy", state="working")
        time.sleep(0.3)
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (22, "fail_descendants_live"))

    def test_gate_run_by_the_predecessor_does_not_count_its_own_shell(self):
        # The gate's calling shell carries "--pred-host claude" in its argv and
        # sits beneath the predecessor's process. It is not a child. A real
        # session process beside it still is.
        reg = self.tmp / "register.py"
        reg.write_text("import json,sys\np=sys.argv[1]; d=json.load(open(p))\n"
                       "d['sessions'][sys.argv[2]]['pid']=int(sys.argv[3]); json.dump(d, open(p,'w'))\n")
        register = shlex.join([sys.executable, str(reg), self.env["FAKE_CLAUDE_STATE"], self.pred]) + " $$"
        gate = shlex.join([sys.executable, str(GATE), "prepare", str(self.handoff), "--role", "test-root",
                           "--state", str(self.state), "--repo", str(self.repo), "--host-state", str(self.host_state),
                           "--pred-host", "claude", "--pred-session", self.pred, "--successor-host", "claude",
                           "--validator", PASS, "--quiescence-file", str(self.quiescent)])
        for extra, want in (("", 10), ("(exec -a claude sleep 30) & sleep 0.3; ", 22)):
            shutil.rmtree(self.state, ignore_errors=True)
            # The predecessor runs the gate through a tool shell of its own, as a
            # host does: predecessor -> shell (argv names claude) -> gate.
            tool_shell = shlex.join(["bash", "-c", gate + " ; exit $?"])
            r = subprocess.run(["bash", "-c", f"{register}; {extra}{tool_shell}; rc=$?; kill %1 2>/dev/null; exit $rc"],
                               env=self.env, capture_output=True, text=True, timeout=120)
            self.assertEqual(r.returncode, want, r.stderr[-600:])

    def test_loaded_ledger_child_blocks(self):
        prompt = self.tmp / "p.txt"; prompt.write_text("work")
        rc, out, err = self.host("launch", "claude", "--cwd", str(self.repo), "--name", "child",
                                 "--prompt-file", str(prompt), "--parent", self.pred,
                                 env={"FAKE_SUCCESSOR": "silent"})
        self.assertEqual(rc, 0, err)
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (22, "fail_children_live"))


class GateCommit(Base):
    def commit(self, successor="claude", **env):
        self.assertEqual(self.gate("prepare", successor=successor)[0], 10)
        return self.gate("commit", successor=successor, env=env)

    def test_claude_successor_adopts_and_commits(self):
        rc, status, err = self.commit()
        self.assertEqual((rc, status), (0, "committed_rotation_1"), err)
        sid = (self.state / "adoption").read_text().split()[0].split("=")[1]
        rows = self.board_rows(kind="adoption", session_id=sid)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["topic"], f"session:{sid}")
        self.assertEqual(rows[0]["data"]["predecessor_session"], self.pred)
        ending = self.board_rows(kind="availability", session_id=self.pred, role="test-root")
        self.assertEqual(len(ending), 1)
        self.assertEqual(ending[0]["data"]["state"], "ending")
        r = subprocess.run([sys.executable, str(BOARD), "--repository", str(self.repo), "roles"],
                           capture_output=True, text=True, check=True)
        role = [x for x in json.loads(r.stdout)["roles"] if x["role"] == "test-root"][0]
        self.assertEqual(role["state"], "single", role)
        self.assertEqual(role["reported_holder"]["session"], sid)

    def test_codex_successor_adopts_and_commits(self):
        rc, status, err = self.commit(successor="codex")
        self.assertEqual((rc, status), (0, "committed_rotation_1"), err)

    def test_adoption_on_another_topic_is_not_accepted(self):
        # S2<->S5 integration condition: a row addressed `to` the successor on
        # another topic must not be read as its adoption.
        rc, status, err = self.commit(FAKE_SUCCESSOR="wrong_topic")
        self.assertEqual((rc, status), (34, "fail_no_adoption"), err)
        self.assertIn("ignored adoption-shaped row", err)
        self.assertEqual(len(self.board_rows(kind="adoption", topic="aspect:handoff")), 1)

    def test_adoption_naming_another_predecessor_is_not_accepted(self):
        rc, status, _ = self.commit(FAKE_SUCCESSOR="wrong_pred")
        self.assertEqual((rc, status), (34, "fail_no_adoption"))
        self.assertEqual(self.board_rows(kind="availability"), [], "a failed rotation reports no ending")

    def test_successor_that_ends_without_adopting_fails_early_and_is_stopped(self):
        t0 = time.time()
        rc, status, _ = self.commit(FAKE_SUCCESSOR="silent")
        self.assertEqual((rc, status), (34, "fail_no_adoption"))
        self.assertLess(time.time() - t0, 50, "should not wait for the full adoption timeout")
        orphan = (self.state / "orphan-successor").read_text()
        self.assertIn("disposition=stop", orphan)
        loaded = [s for s in self.claude_sessions().values() if s["name"] == "test-root" and s["loaded"]]
        self.assertEqual(loaded, [])

    def test_retire_stop_on_interactive_predecessor_is_reported_not_committed_clean(self):
        self.assertEqual(self.gate("prepare")[0], 10)
        self.add_claude_session(self.pred, "operator-shell", kind="interactive", pid=self.pred_proc.pid,
                                status="idle", state="done")
        rc, status, _ = self.gate("commit", "--retire", "stop")
        self.assertEqual((rc, status), (38, "committed_rotation_1_predecessor_not_retired"))

    def test_retire_stop_on_background_predecessor(self):
        self.add_claude_session(self.pred, "operator-shell", kind="background", pid=self.pred_proc.pid,
                                status="idle", state="done")
        self.assertEqual(self.gate("prepare")[0], 10)
        rc, status, err = self.gate("commit", "--retire", "stop")
        self.assertEqual((rc, status), (0, "committed_rotation_1"), err)
        self.assertFalse(self.claude_sessions()[self.pred]["loaded"])

    def test_cap(self):
        (self.state).mkdir(parents=True, exist_ok=True)
        (self.state / "rotation-count").write_text("3\n")
        rc, status, _ = self.gate("prepare")
        self.assertEqual((rc, status), (20, "fail_cap_reached"))


class HostAdapter(Base):
    def launch(self, host="claude", **env):
        prompt = self.tmp / "p.txt"; prompt.write_text("work")
        rc, out, err = self.host("launch", host, "--cwd", str(self.repo), "--name", "worker",
                                 "--prompt-file", str(prompt), env={"FAKE_SUCCESSOR": "silent", **env})
        self.assertEqual(rc, 0, err)
        return json.loads(out)["session_id"]

    def test_claude_deliver_to_idle_loaded_stops_then_resumes_same_id(self):
        sid = self.launch()
        self.assertTrue(self.wait_for(lambda: self.claude_sessions()[sid]["status"] == "idle"))
        msg = self.tmp / "m.txt"; msg.write_text("next")
        rc, out, err = self.host("deliver", "claude", sid, "--cwd", str(self.repo), "--message-file", str(msg),
                                 env={"FAKE_SUCCESSOR": "silent"})
        self.assertEqual(rc, 0, err)
        self.assertEqual(len(self.claude_sessions()), 2, "no copy may be created")

    def test_claude_copy_is_detected_contained_and_reported(self):
        sid = self.launch()
        self.assertTrue(self.wait_for(lambda: self.claude_sessions()[sid]["status"] == "idle"))
        # Simulate a race: the session is loaded when the resume lands. Calling
        # the host directly with --resume reproduces the copy path.
        sys.path.insert(0, str(ROOT / "scripts"))
        import role_host
        os.environ.update(FAKE_CLAUDE_STATE=self.env["FAKE_CLAUDE_STATE"], PATH=self.env["PATH"],
                          FAKE_SUCCESSOR="silent")
        h = role_host.Claude(str(FAKES / "claude"))
        r = subprocess.run([str(FAKES / "claude"), "--bg", "--resume", sid, "x"], env=self.env,
                           capture_output=True, text=True)
        with self.assertRaises(role_host.HostError) as cm:
            h._contain_copy(r.stdout, role_host.Ledger(self.host_state), sid)
        self.assertEqual((cm.exception.code, cm.exception.status), (21, "fork_detected"))
        copies = [k for k, s in self.claude_sessions().items() if k not in (sid, self.pred)]
        self.assertEqual(len(copies), 1)
        self.assertFalse(self.claude_sessions()[copies[0]]["loaded"], "the copy must be stopped")

    def test_claude_deliver_refused_while_running(self):
        sid = self.launch(FAKE_TURN="hang")
        msg = self.tmp / "m.txt"; msg.write_text("next")
        rc, _, err = self.host("deliver", "claude", sid, "--cwd", str(self.repo), "--message-file", str(msg))
        self.assertEqual(rc, 40, err)

    def codex_status(self, sid):
        rc, out, err = self.host("status", "codex", sid)
        self.assertEqual(rc, 0, err)
        return json.loads(out)

    def test_codex_launch_is_a_named_daemon_thread_with_no_host_process(self):
        sid = self.launch("codex")
        row = [r for r in role_host_rows(self.host_state) if r["session_id"] == sid][0]
        self.assertEqual((row["transport"], row.get("pid")), ("app_server", None))
        self.assertEqual(self.codex_status(sid).get("name"), "worker")

    def test_codex_just_started_turn_reads_as_running(self):
        # The daemon hides a new turn from reads for a moment; status must not
        # call the thread idle in that window.
        sid = self.launch("codex", FAKE_TURN="hang")
        st = self.codex_status(sid)
        self.assertEqual(st["state"], "running", st)
        self.assertIn("pending_turn", st)
        time.sleep(0.7)
        st = self.codex_status(sid)
        self.assertEqual((st["state"], st["last_turn"]), ("running", "inProgress"), st)

    def test_codex_deliver_resumes_same_thread(self):
        sid = self.launch("codex")
        self.assertTrue(self.wait_for(lambda: self.codex_status(sid)["state"] == "idle"))
        msg = self.tmp / "m.txt"; msg.write_text("next")
        rc, out, err = self.host("deliver", "codex", sid, "--cwd", str(self.repo), "--message-file", str(msg),
                                 env={"FAKE_SUCCESSOR": "silent"})
        self.assertEqual(rc, 0, err)
        self.assertEqual(json.loads(out)["session_id"], sid)

    def test_codex_deliver_refused_while_running(self):
        sid = self.launch("codex", FAKE_TURN="hang")
        msg = self.tmp / "m.txt"; msg.write_text("next")
        rc, _, err = self.host("deliver", "codex", sid, "--cwd", str(self.repo), "--message-file", str(msg))
        self.assertEqual(rc, 40, err)

    def test_codex_resume_that_returns_another_thread_is_a_defect(self):
        sid = self.launch("codex")
        self.assertTrue(self.wait_for(lambda: self.codex_status(sid)["state"] == "idle"))
        msg = self.tmp / "m.txt"; msg.write_text("next")
        rc, _, err = self.host("deliver", "codex", sid, "--cwd", str(self.repo), "--message-file", str(msg),
                               env={"FAKE_SUCCESSOR": "silent", "FAKE_CODEX_MISMATCH": "1"})
        self.assertEqual(rc, 21, err)

    def test_codex_retire_running_needs_force_and_interrupts(self):
        sid = self.launch("codex", FAKE_TURN="hang")
        self.assertEqual(self.host("retire", "codex", sid)[0], 40)
        rc, out, err = self.host("retire", "codex", sid, "--force")
        self.assertEqual((rc, json.loads(out)["retired"]), (0, "turn_interrupted"), err)
        self.assertTrue(self.wait_for(lambda: self.codex_status(sid)["last_turn"] == "interrupted"))
        self.assertEqual(self.codex_status(sid)["state"], "stopped")

    def test_codex_retired_idle_child_no_longer_blocks_quiescence(self):
        prompt = self.tmp / "p.txt"; prompt.write_text("work")
        rc, out, err = self.host("launch", "codex", "--cwd", str(self.repo), "--name", "child",
                                 "--prompt-file", str(prompt), "--parent", self.pred,
                                 env={"FAKE_SUCCESSOR": "silent"})
        self.assertEqual(rc, 0, err)
        sid = json.loads(out)["session_id"]
        self.assertTrue(self.wait_for(lambda: self.codex_status(sid)["state"] == "idle"))
        self.assertEqual(self.gate("prepare")[:2], (22, "fail_children_live"))
        shutil.rmtree(self.state, ignore_errors=True)
        rc, out, err = self.host("retire", "codex", sid)
        self.assertEqual((rc, json.loads(out)["retired"]), (0, "idle_released"), err)
        self.assertEqual(self.gate("prepare")[:2], (10, "awaiting_authority"))

    def test_codex_unloaded_thread_trusts_disk_history_not_the_daemon(self):
        import sqlite3
        busy, done = str(uuid.uuid4()), str(uuid.uuid4())
        con = sqlite3.connect(self.codex_home / "thread_history_1.sqlite")
        con.execute("create table thread_turns (thread_id text, turn_id text, rollout_ordinal integer, status text)")
        con.executemany("insert into thread_turns values (?,?,?,?)",
                        [(busy, "t1", 1, "inProgress"), (done, "t1", 1, "completed")])
        con.commit(); con.close()
        env = {"FAKE_DISK_THREADS": f"{busy},{done}"}
        rc, out, err = self.host("status", "codex", busy, env=env)
        self.assertEqual((rc, json.loads(out)["state"], json.loads(out)["kind"]), (0, "running", "unknown_holder"), err)
        rc, out, err = self.host("status", "codex", done, env=env)
        self.assertEqual((rc, json.loads(out)["state"]), (0, "stopped"), err)

    def test_codex_unknown_thread_is_absent(self):
        self.assertEqual(self.codex_status(str(uuid.uuid4()))["state"], "absent")

    def test_codex_unmapped_host_arg_is_refused(self):
        prompt = self.tmp / "p.txt"; prompt.write_text("work")
        rc, _, err = self.host("launch", "codex", "--cwd", str(self.repo), "--name", "w", "--prompt-file",
                               str(prompt), "--host-arg=--oss")
        self.assertEqual(rc, 64, err)

    def test_codex_host_args_map_onto_thread_start(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        import role_host
        params, effort = role_host.Codex.thread_options(
            ["--model", "gpt-6-astra", "-c", 'model_reasoning_effort="high"', "-c", "x.y=3", "-s", "read-only"])
        self.assertEqual(params, {"approvalPolicy": "never", "model": "gpt-6-astra", "sandbox": "read-only",
                                  "config": {"model_reasoning_effort": "high", "x.y": 3}})
        self.assertEqual(effort, "high")

    def test_children_lists_only_ledger_children_of_that_parent(self):
        prompt = self.tmp / "p.txt"; prompt.write_text("work")
        for parent in (self.pred, str(uuid.uuid4())):
            self.host("launch", "claude", "--cwd", str(self.repo), "--name", "c", "--prompt-file", str(prompt),
                      "--parent", parent, env={"FAKE_SUCCESSOR": "silent"})
        rc, out, _ = self.host("children", self.pred)
        self.assertEqual(rc, 0)
        self.assertEqual(len(json.loads(out)["children"]), 1)

    def test_whoami(self):
        rc, out, _ = self.host("whoami")
        self.assertEqual((rc, json.loads(out)["session_id"]), (0, self.caller))
        del self.env["CLAUDE_CODE_SESSION_ID"]
        rc, _, _ = self.host("whoami")
        self.assertEqual(rc, 32)

    def test_whoami_refuses_two_host_ids(self):
        rc, _, err = self.host("whoami", env={"CODEX_THREAD_ID": str(uuid.uuid4())})
        self.assertEqual(rc, 32, err)


class GateWatch(Base):
    def post(self, author, body):
        subprocess.run([sys.executable, str(BOARD), "--repository", str(self.repo), "post", "--author", author,
                        "--session-id", str(uuid.uuid4()), "--topic", "control:rotation:test-root", "--body", body],
                       capture_output=True, text=True, check=True)

    def watch(self, *extra, approver="the-user"):
        argv = [sys.executable, str(GATE), "watch", str(self.handoff), "--role", "test-root",
                "--state", str(self.state), "--repo", str(self.repo), "--host-state", str(self.host_state),
                "--pred-host", "claude", "--pred-session", self.pred, "--successor-host", "claude",
                "--validator", PASS, "--quiescence-file", str(self.quiescent), "--adopt-timeout", "60",
                "--after-adopt", "stop", "--watch-timeout", "20", *extra]
        if approver:
            argv += ["--gate-authority", approver]
        return subprocess.Popen(argv, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def status(self):
        return (self.state / "terminal-status").read_text().strip()

    def test_refuses_without_named_approver(self):
        p = self.watch(approver=None)
        self.assertEqual(p.wait(30), 77)
        self.assertEqual(self.status(), "watch_refused_no_authority")

    def test_unbound_and_unauthorized_approvals_are_ignored_and_window_expires(self):
        p = self.watch()
        self.assertTrue(self.wait_for(lambda: (self.state / "watch-state").exists()))
        dig = __import__("hashlib").sha256(self.handoff.read_bytes()).hexdigest()
        self.post("the-user", "approve-rotation")                 # no digest
        self.post("someone-else", f"approve-rotation {dig}")       # not an approver
        self.assertEqual(p.wait(60), 78)
        self.assertEqual(self.status(), "watch_expired")
        self.assertEqual(len([s for s in self.claude_sessions().values() if s["name"] == "test-root"]), 0)

    def test_authorized_approval_behind_a_rejected_one_commits(self):
        p = self.watch()
        self.assertTrue(self.wait_for(lambda: (self.state / "watch-state").exists()))
        dig = __import__("hashlib").sha256(self.handoff.read_bytes()).hexdigest()
        self.post("someone-else", f"approve-rotation {dig}")
        self.post("the-user", f"approve-rotation {dig}")
        rc = p.wait(120)
        self.assertEqual((rc, self.status()), (0, "committed_rotation_1"), p.stderr.read()[-800:])
        self.assertIn("author=the-user", (self.state / "authorized-by").read_text())


if __name__ == "__main__":
    unittest.main()
