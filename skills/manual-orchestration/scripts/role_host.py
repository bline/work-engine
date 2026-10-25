#!/usr/bin/env python3
"""Host a role as a Claude or Codex CLI session: launch, status, deliver, retire.

No model judgment here. Every operation either does what it says, refuses, or
reports that it could not establish the outcome. Exit codes match the rotation
gate: 0 ok, 21 defect (a fork or an identity mismatch was observed), 32 not
established, 40 refused because the session is busy, 64 usage.

Host facts this relies on (probe metrics/probes/cli-daemon-host-20261025.json,
claude 2.1.310, codex 0.164.0, n = 1 per cell):

claude
  launch   `claude --bg --name N "<prompt>"`. --bg IGNORES --session-id; the
           daemon assigns the id and prints `backgrounded · <short8> · N`.
  status   `claude agents --json [--all]`, matched by sessionId. A stopped
           background session appears only with --all.
  deliver  Resuming a session that is still loaded does NOT deliver: it starts
           a COPY with a new id under the same name and exits 0. Delivery is
           therefore stop-then-resume of an idle session, and any "started a
           copy" output is a fork, which is contained and reported.
  retire   `claude stop <short>` keeps the conversation.
codex
  launch   `codex exec --json "<prompt>"`; the first event is thread.started
           with the thread id. The process lives for one turn.
  status   the exec process this ledger launched, then thread_turns in
           $CODEX_HOME/thread_history_1.sqlite.
  deliver  `codex exec --json resume <id> "<msg>"` continues the same thread.
           `codex queue` without a live session only deposits text that merges
           into the next turn, so it is never used as delivery here.
  retire   an exec-hosted thread has no process between turns; a thread held
           by an interactive TUI cannot be seen or stopped from here.

Children: every launch with --parent appends to a ledger, so a root's
cross-CLI children are parent-scoped by construction. Children started any
other way (a host's in-process subagents, a shell the root ran itself) are not
in the ledger; `children` says so rather than reporting them as absent.
"""
from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import signal
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

OK, DEFECT, NOT_ESTABLISHED, BUSY, USAGE = 0, 21, 32, 40, 64
UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
DEFAULT_STATE = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "work-engine/role-host"


class HostError(Exception):
    def __init__(self, code: int, status: str, detail: str):
        super().__init__(detail)
        self.code, self.status, self.detail = code, status, detail


def _run(argv, *, cwd=None, timeout=60):
    try:
        return subprocess.run(argv, cwd=cwd, capture_output=True, text=True,
                              timeout=timeout, stdin=subprocess.DEVNULL)
    except (OSError, subprocess.TimeoutExpired) as e:
        raise HostError(NOT_ESTABLISHED, "host_cli_failed", f"{argv[0]}: {e}")


# ---------------------------------------------------------------- ledger ----

class Ledger:
    """Append-only JSON lines: one row per launch or delivery this tool made."""

    def __init__(self, state: Path):
        self.path = state / "ledger.jsonl"
        state.mkdir(parents=True, exist_ok=True)

    def append(self, row: dict) -> None:
        row = {"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **row}
        with open(self.path, "a") as fh:
            fcntl.flock(fh, fcntl.LOCK_EX)
            fh.write(json.dumps(row, sort_keys=True) + "\n")

    def rows(self) -> list[dict]:
        if not self.path.exists():
            return []
        out = []
        with open(self.path) as fh:
            fcntl.flock(fh, fcntl.LOCK_SH)
            for n, line in enumerate(fh, 1):
                try:
                    out.append(json.loads(line))
                except ValueError:
                    raise HostError(NOT_ESTABLISHED, "ledger_unreadable", f"{self.path}:{n}")
        return out

    def latest(self, session_id: str) -> dict | None:
        rows = [r for r in self.rows() if r.get("session_id") == session_id]
        return rows[-1] if rows else None


# ---------------------------------------------------------------- claude ----

class Claude:
    name = "claude"
    LAUNCHED = re.compile(r"backgrounded · ([0-9a-f]{8})")
    COPY = re.compile(r"started a copy as ([0-9a-f]{8})")

    def __init__(self, exe="claude"):
        self.exe = exe

    def _agents(self, all_rows: bool) -> list[dict]:
        r = _run([self.exe, "agents", "--json"] + (["--all"] if all_rows else []))
        if r.returncode != 0:
            raise HostError(NOT_ESTABLISHED, "status_not_established",
                            f"claude agents exit {r.returncode}: {r.stderr.strip()[:200]}")
        try:
            rows = json.loads(r.stdout)
        except ValueError:
            raise HostError(NOT_ESTABLISHED, "status_not_established", "claude agents output is not JSON")
        if not isinstance(rows, list):
            raise HostError(NOT_ESTABLISHED, "status_not_established", "claude agents output is not a list")
        return rows

    def _row(self, sid, all_rows=False):
        hits = [r for r in self._agents(all_rows) if r.get("sessionId") == sid]
        if len(hits) > 1:
            raise HostError(NOT_ESTABLISHED, "status_not_established", f"{len(hits)} rows carry {sid}")
        return hits[0] if hits else None

    def status(self, sid, ledger=None) -> dict:
        row = self._row(sid)
        if row is None:
            stopped = self._row(sid, all_rows=True)
            return {"state": "stopped" if stopped else "absent", "kind": (stopped or {}).get("kind")}
        busy = row.get("state") == "working" or row.get("status") == "busy"
        return {"state": "running" if busy else "idle", "kind": row.get("kind"),
                "short_id": row.get("id"), "name": row.get("name"), "cwd": row.get("cwd")}

    def _resolve_short(self, short):
        rows = [r for r in self._agents(False) if r.get("id") == short]
        if len(rows) != 1:
            raise HostError(NOT_ESTABLISHED, "identity_not_established",
                            f"{len(rows)} active sessions have short id {short}")
        sid = rows[0].get("sessionId", "")
        if not UUID.match(sid) or not sid.startswith(short):
            raise HostError(NOT_ESTABLISHED, "identity_not_established", f"short {short} -> {sid!r}")
        return sid

    def _contain_copy(self, out, ledger, origin):
        m = self.COPY.search(out)
        if not m:
            return
        copy = m.group(1)
        r = _run([self.exe, "stop", copy])
        ledger.append({"host": self.name, "event": "fork_contained", "copy_short_id": copy,
                       "origin_session_id": origin, "stop_exit": r.returncode})
        raise HostError(DEFECT, "fork_detected",
                        f"host started a copy {copy} instead of delivering to {origin}; "
                        f"copy stop exit {r.returncode}")

    def launch(self, *, cwd, name, prompt, host_args, ledger, parent=None) -> dict:
        r = _run([self.exe, "--bg", "--name", name, *host_args, prompt], cwd=cwd, timeout=60)
        out = r.stdout + r.stderr
        if r.returncode != 0:
            raise HostError(NOT_ESTABLISHED, "fail_launch", out.strip()[:300])
        self._contain_copy(out, ledger, None)
        m = self.LAUNCHED.search(out)
        if not m:
            raise HostError(NOT_ESTABLISHED, "identity_not_established", "no backgrounded line in launch output")
        sid = self._resolve_short(m.group(1))
        row = {"host": self.name, "event": "launch", "session_id": sid, "name": name,
               "cwd": str(cwd), "parent_session_id": parent}
        ledger.append(row)
        return row

    def deliver(self, sid, *, message, cwd, ledger, stop_wait=15) -> dict:
        st = self.status(sid)
        if st["state"] == "running":
            raise HostError(BUSY, "refused_busy", f"{sid} is running; delivering now would interrupt it")
        if st.get("kind") == "interactive":
            raise HostError(BUSY, "refused_interactive",
                            f"{sid} is an interactive session; it must collect its own messages")
        if st["state"] == "idle":
            r = _run([self.exe, "stop", st["short_id"]])
            if r.returncode != 0:
                raise HostError(NOT_ESTABLISHED, "stop_failed", r.stderr.strip()[:200])
            deadline = time.time() + stop_wait
            while self.status(sid)["state"] == "idle":
                if time.time() > deadline:
                    raise HostError(NOT_ESTABLISHED, "stop_not_established", f"{sid} still loaded after stop")
                time.sleep(0.5)
        r = _run([self.exe, "--bg", "--resume", sid, message], cwd=cwd, timeout=60)
        out = r.stdout + r.stderr
        if r.returncode != 0:
            raise HostError(NOT_ESTABLISHED, "deliver_failed", out.strip()[:300])
        self._contain_copy(out, ledger, sid)
        m = self.LAUNCHED.search(out)
        if not m or not sid.startswith(m.group(1)):
            raise HostError(DEFECT, "identity_mismatch",
                            f"resume of {sid} reported {m.group(1) if m else 'no id'}")
        ledger.append({"host": self.name, "event": "deliver", "session_id": sid})
        return {"session_id": sid, "delivered": True}

    def retire(self, sid, *, ledger, force=False) -> dict:
        st = self.status(sid)
        if st["state"] in ("stopped", "absent"):
            return {"session_id": sid, "retired": "already_not_loaded", "state": st["state"]}
        if st.get("kind") == "interactive":
            raise HostError(NOT_ESTABLISHED, "retire_not_established",
                            "an interactive session cannot be stopped from here; its user closes it")
        if st["state"] == "running" and not force:
            raise HostError(BUSY, "refused_busy", f"{sid} is running")
        r = _run([self.exe, "stop", st["short_id"]])
        if r.returncode != 0:
            raise HostError(NOT_ESTABLISHED, "retire_not_established", r.stderr.strip()[:200])
        ledger.append({"host": self.name, "event": "retire", "session_id": sid})
        return {"session_id": sid, "retired": "stopped"}


# ----------------------------------------------------------------- codex ----

class Codex:
    name = "codex"

    def __init__(self, exe="codex", codex_home=None):
        self.exe = exe
        self.home = Path(codex_home or os.environ.get("CODEX_HOME") or Path.home() / ".codex")

    def _turn_status(self, sid):
        db = self.home / "thread_history_1.sqlite"
        if not db.exists():
            raise HostError(NOT_ESTABLISHED, "status_not_established", f"{db} missing")
        try:
            con = sqlite3.connect(f"file:{db}?mode=ro", uri=True, timeout=5)
            try:
                row = con.execute("select status, count(*) over () from thread_turns where thread_id=? "
                                  "order by rollout_ordinal desc limit 1", (sid,)).fetchone()
            finally:
                con.close()
        except sqlite3.Error as e:
            raise HostError(NOT_ESTABLISHED, "status_not_established", f"thread history: {e}")
        return (row[0], row[1]) if row else (None, 0)

    @staticmethod
    def _alive(pid):
        if not pid:
            return False
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return True
        # A zombie is not a live turn.
        try:
            return Path(f"/proc/{pid}/stat").read_text().split()[2] != "Z"
        except OSError:
            return False

    def status(self, sid, ledger=None) -> dict:
        last = ledger.latest(sid) if ledger else None
        pid = (last or {}).get("pid")
        if self._alive(pid):
            return {"state": "running", "kind": "exec", "pid": pid}
        status, turns = self._turn_status(sid)
        if status is None:
            return {"state": "absent", "kind": None, "turns": 0}
        if status == "inProgress":
            # In progress with no live process this ledger owns: either another
            # client (a TUI) holds it, or a turn died. Neither is idle.
            return {"state": "running", "kind": "unknown_holder", "turns": turns}
        return {"state": "idle", "kind": "exec" if last else "unknown_holder",
                "turns": turns, "last_turn": status}

    def _spawn(self, argv, cwd, out_path, ledger_state):
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with open(out_path, "w") as out, open(str(out_path) + ".err", "w") as err:
            p = subprocess.Popen(argv, cwd=cwd, stdout=out, stderr=err, stdin=subprocess.DEVNULL,
                                 start_new_session=True)
        return p

    def _thread_started(self, path, proc, timeout):
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                for line in Path(path).read_text().splitlines():
                    try:
                        ev = json.loads(line)
                    except ValueError:
                        continue
                    if ev.get("type") == "thread.started":
                        return ev.get("thread_id")
            except OSError:
                pass
            if proc.poll() is not None:
                break
            time.sleep(0.2)
        return None

    def launch(self, *, cwd, name, prompt, host_args, ledger, parent=None, start_timeout=60) -> dict:
        out = ledger.path.parent / "codex" / f"launch-{int(time.time() * 1000)}.jsonl"
        p = self._spawn([self.exe, "exec", "--json", *host_args, prompt], cwd, out, ledger)
        sid = self._thread_started(out, p, start_timeout)
        if not sid or not UUID.match(sid):
            raise HostError(NOT_ESTABLISHED, "identity_not_established",
                            f"no thread.started within {start_timeout}s (exit {p.poll()}); see {out}")
        row = {"host": self.name, "event": "launch", "session_id": sid, "name": name, "cwd": str(cwd),
               "parent_session_id": parent, "pid": p.pid, "events": str(out)}
        ledger.append(row)
        return row

    def deliver(self, sid, *, message, cwd, ledger, host_args=(), start_timeout=60) -> dict:
        st = self.status(sid, ledger)
        if st["state"] == "running":
            raise HostError(BUSY, "refused_busy", f"{sid} has a turn in progress ({st['kind']})")
        if st["state"] == "absent":
            raise HostError(NOT_ESTABLISHED, "deliver_failed", f"thread {sid} is unknown to this host")
        out = ledger.path.parent / "codex" / f"deliver-{int(time.time() * 1000)}.jsonl"
        p = self._spawn([self.exe, "exec", "--json", *host_args, "resume", sid, message], cwd, out, ledger)
        got = self._thread_started(out, p, start_timeout)
        if got != sid:
            if self._alive(p.pid):
                os.killpg(p.pid, signal.SIGTERM)
            raise HostError(DEFECT if got else NOT_ESTABLISHED,
                            "identity_mismatch" if got else "deliver_not_established",
                            f"resume of {sid} started {got or 'nothing'}; see {out}")
        ledger.append({"host": self.name, "event": "deliver", "session_id": sid, "pid": p.pid,
                       "events": str(out)})
        return {"session_id": sid, "delivered": True}

    def retire(self, sid, *, ledger, force=False) -> dict:
        st = self.status(sid, ledger)
        if st["state"] == "running":
            if st["kind"] != "exec" or not force:
                raise HostError(BUSY, "refused_busy", f"{sid} has a turn in progress ({st['kind']})")
            os.killpg(st["pid"], signal.SIGTERM)
            ledger.append({"host": self.name, "event": "retire", "session_id": sid, "killed_pid": st["pid"]})
            return {"session_id": sid, "retired": "turn_terminated"}
        if st["kind"] == "unknown_holder":
            # This tool did not launch it; a TUI may hold it and nothing here sees that.
            raise HostError(NOT_ESTABLISHED, "retire_not_established",
                            "thread not launched by this ledger; an interactive holder cannot be seen")
        ledger.append({"host": self.name, "event": "retire", "session_id": sid})
        return {"session_id": sid, "retired": "no_live_process"}


HOSTS = {"claude": Claude, "codex": Codex}


def host(name):
    if name not in HOSTS:
        raise HostError(USAGE, "usage", f"unknown host {name!r}")
    exe = os.environ.get(f"ROLE_HOST_{name.upper()}_EXE", name)
    return HOSTS[name](exe)


def children(parent, ledger) -> dict:
    """Ledger children of `parent`, each with its current status."""
    launched = [r for r in ledger.rows() if r.get("event") == "launch" and r.get("parent_session_id") == parent]
    rows, unknown = [], 0
    for r in launched:
        try:
            st = host(r["host"]).status(r["session_id"], ledger)
        except HostError as e:
            st, unknown = {"state": "not_established", "detail": e.detail}, unknown + 1
        rows.append({"host": r["host"], "session_id": r["session_id"], "name": r.get("name"), **st})
    return {"parent_session_id": parent, "children": rows,
            "running": sum(1 for r in rows if r["state"] == "running"),
            "idle_loaded": sum(1 for r in rows if r["state"] == "idle"),
            "not_established": unknown,
            "coverage": "children launched through role_host with --parent only; in-process "
                        "subagents and processes the parent started directly are not covered"}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--state", type=Path, default=DEFAULT_STATE)
    sub = ap.add_subparsers(dest="cmd", required=True)
    la = sub.add_parser("launch")
    la.add_argument("host"); la.add_argument("--cwd", required=True); la.add_argument("--name", required=True)
    la.add_argument("--prompt-file", required=True, type=Path)
    la.add_argument("--parent"); la.add_argument("--host-arg", action="append", default=[])
    for verb in ("status", "retire"):
        s = sub.add_parser(verb); s.add_argument("host"); s.add_argument("session_id")
        if verb == "retire":
            s.add_argument("--force", action="store_true")
    de = sub.add_parser("deliver")
    de.add_argument("host"); de.add_argument("session_id"); de.add_argument("--cwd", required=True)
    de.add_argument("--message-file", required=True, type=Path)
    de.add_argument("--host-arg", action="append", default=[])
    ch = sub.add_parser("children"); ch.add_argument("parent")
    sub.add_parser("whoami", help="print this process's own session id from the host's environment")
    a = ap.parse_args(argv)
    try:
        if a.cmd == "whoami":
            for var, h in (("CLAUDE_CODE_SESSION_ID", "claude"), ("CODEX_THREAD_ID", "codex")):
                v = os.environ.get(var, "")
                if UUID.match(v):
                    print(json.dumps({"host": h, "session_id": v, "source": var})); return OK
            raise HostError(NOT_ESTABLISHED, "identity_not_established", "no host session id in environment")
        ledger = Ledger(a.state)
        if a.cmd == "children":
            res = children(a.parent, ledger)
        else:
            sid = getattr(a, "session_id", None)
            if sid is not None and not UUID.match(sid):
                raise HostError(USAGE, "usage", f"session id {sid!r} is not a lowercase canonical uuid")
            h = host(a.host)
            if a.cmd == "launch":
                res = h.launch(cwd=a.cwd, name=a.name, prompt=a.prompt_file.read_text(),
                               host_args=a.host_arg, ledger=ledger, parent=a.parent)
            elif a.cmd == "status":
                res = {"session_id": sid, "host": a.host, **h.status(sid, ledger)}
            elif a.cmd == "deliver":
                kw = {"host_args": a.host_arg} if a.host == "codex" else {}
                res = h.deliver(sid, message=a.message_file.read_text(), cwd=a.cwd, ledger=ledger, **kw)
            else:
                res = h.retire(sid, ledger=ledger, force=a.force)
        print(json.dumps(res, sort_keys=True))
        return OK
    except HostError as e:
        print(json.dumps({"status": e.status, "detail": e.detail}), file=sys.stderr)
        return e.code


if __name__ == "__main__":
    sys.exit(main())
