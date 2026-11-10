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
codex (probe metrics/probes/codex-daemon-thread-host-20261026.json, n = 1)
  host     the shared local app-server daemon (unix socket, JSON-RPC over
           WebSocket). `codex resume` goes through the same daemon, and the
           daemon rejoins a running thread for a second client (observed for
           a protocol client; a terminal joining mid-turn is not established).
  launch   thread/start (cwd, model, sandbox, approvalPolicy never, -c config),
           thread/name/set to the role name, turn/start with the prompt; then
           disconnect. The turn keeps running on the daemon.
  status   thread/read status plus the latest turn. A turn just started can be
           invisible to both for a moment, so the ledger's turn id counts as
           running until a read shows it. Unloaded threads fall back to
           $CODEX_HOME/thread_history_1.sqlite.
  deliver  thread/resume (identity check) then turn/start on an idle thread.
  retire   turn/interrupt a running turn (with --force); an idle thread is
           released by a ledger row, since the daemon has no unload request.
  legacy   ledger rows from earlier `codex exec` launches carry a pid; a live
           pid still reads as running, and retire --force terminates it.
pi (probe metrics/probes/pi-cli-host-20261109.json, pi 1.4.0, local fake
OpenAI endpoint, n = 1 per cell; no provider was contacted)
  host     no daemon. Each turn is one detached `pi --mode json` process
           (its own process group) whose stdin is the prompt file and whose
           stdout/stderr/exit status land in the role-host state directory.
  launch   `--session-id <uuid>` creates the session under that exact id; the
           first stdout record is the session header carrying it.
  status   a live turn pid (checked against its recorded start time) is
           running; otherwise idle, with the last turn read from its events.
           JSON mode exits 0 even when the response failed, so outcome comes
           from the events, not the exit status.
  deliver  the same `--session-id` on an idle session continues that session
           file with its history; launch settings are reused.
  retire   SIGTERM to a running turn's process group (with --force); the
           session keeps the user message without a reply. Idle is a ledger row.
  auth     every launch names a lane: `api` needs Pi's credential for the
           provider to be an API key, `subscription` an OAuth login. A service
           tier is accepted only on the api lane for provider openai, and is
           sent and observed by scripts/pi_service_tier.ts.

Children: every launch with --parent appends to a ledger, so a root's
cross-CLI children are parent-scoped by construction. Children started any
other way (a host's in-process subagents, a shell the root ran itself) are not
in the ledger; `children` says so rather than reporting them as absent.
"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import re
import signal
import sqlite3
import subprocess
import sys
import time
import tomllib
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import codex_app  # noqa: E402

OK, DEFECT, NOT_ESTABLISHED, BUSY, USAGE = 0, 21, 32, 40, 64
UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
DEFAULT_STATE = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "work-engine/role-host"


class HostError(Exception):
    def __init__(self, code: int, status: str, detail: str):
        super().__init__(detail)
        self.code, self.status, self.detail = code, status, detail


# Variables that name or authenticate the CALLING session. A launched session
# must never inherit them: a Codex successor started from a Claude session
# inherited CLAUDE_CODE_SESSION_ID, `whoami` reported the caller's id, and the
# successor waited on the caller's topic for its whole adoption window
# (observed 2026-10-25). The messaging token is also a credential. Pi 1.4.0's
# bash tool exports its session id, file, provider, model and thinking level to
# every command it runs.
CALLER_IDENTITY = ("CLAUDE_CODE_SESSION_ID", "CLAUDE_PID", "CLAUDE_CODE_MESSAGING_SOCKET",
                   "CLAUDE_CODE_MESSAGING_TOKEN", "CLAUDE_CODE_SESSION_ATTENDED",
                   "CLAUDE_CODE_CHILD_SESSION", "CLAUDECODE", "CODEX_THREAD_ID",
                   "PI_SESSION_ID", "PI_SESSION_FILE", "PI_PROVIDER", "PI_MODEL", "PI_REASONING_LEVEL")


def child_env():
    return {k: v for k, v in os.environ.items() if k not in CALLER_IDENTITY}


def codex_settings(sid: str, rollout: Path, turn_id: str | None = None) -> dict:
    """Read applied settings from this session's rollout, without shared-config inference."""
    latest = None
    active_turn, context_source = None, None
    pending_settings, start_settings, start_source = None, None, None
    selected_settings, selected_contexts = [], []
    try:
        with rollout.open(encoding="utf-8", newline="") as fh:
            first = json.loads(next(fh))
            if not isinstance(first, dict) or not isinstance(first.get("payload"), dict):
                raise ValueError("session_meta must be an object with an object payload")
            if first.get("type") != "session_meta" or first["payload"].get("id") != sid:
                raise HostError(DEFECT, "identity_mismatch", f"{rollout} does not begin with session_meta for {sid}")
            recorded_version = first["payload"].get("cli_version")
            for line_no, raw in enumerate(fh, 2):
                event = json.loads(raw)
                if not isinstance(event, dict):
                    raise ValueError(f"record at line {line_no} must be an object")
                payload = event.get("payload", {})
                if not isinstance(payload, dict):
                    raise ValueError(f"payload at line {line_no} must be an object")
                if event.get("type") == "event_msg" and payload.get("type") == "task_started":
                    active_turn, context_source = payload.get("turn_id"), None
                    start_source = {"line": line_no, "turn_id": active_turn,
                                    "event_sha256": hashlib.sha256(raw.encode()).hexdigest()}
                    start_settings, pending_settings = pending_settings, None
                if event.get("type") == "turn_context":
                    if active_turn != payload.get("turn_id"):
                        start_settings, start_source = None, None
                    active_turn = payload.get("turn_id")
                    pending_settings = None
                    context_source = {"line": line_no, "event_sha256": hashlib.sha256(raw.encode()).hexdigest(),
                                      "turn_id": active_turn, "model": payload.get("model"), "effort": payload.get("effort")}
                    if turn_id is not None and active_turn == turn_id:
                        selected_contexts.append(context_source)
                        # Retained turns can apply settings before task_started,
                        # with no duplicate settings event inside the turn. Bind
                        # only this idle-interval application, explicit start and
                        # matching context; never carry an earlier turn's event.
                        if start_settings is not None and start_source["turn_id"] == turn_id:
                            settings, source = start_settings
                            observation = (settings, {**source, "task_started": start_source,
                                                      "turn_context": context_source})
                            selected_settings.append(observation)
                            latest = observation
                    start_settings = None
                if event.get("type") == "event_msg" and payload.get("type") in ("task_complete", "task_completed"):
                    active_turn, context_source = None, None
                    pending_settings, start_settings, start_source = None, None, None
                if (event.get("type") == "event_msg" and payload.get("type") == "thread_settings_applied"
                        and payload.get("thread_id") == sid):
                    if not isinstance(payload.get("thread_settings"), dict):
                        raise ValueError(f"thread_settings at line {line_no} must be an object")
                    observation = (payload.get("thread_settings") or {}, {
                        "rollout": str(rollout.resolve()), "line": line_no, "timestamp": event.get("timestamp"),
                        "event_sha256": hashlib.sha256(raw.encode()).hexdigest()})
                    if turn_id is None:
                        latest = observation
                    elif active_turn == turn_id and context_source is not None and context_source["turn_id"] == turn_id:
                        observation[1]["turn_context"] = context_source
                        selected_settings.append(observation)
                        latest = observation
                    if active_turn is None:
                        pending_settings = observation
                    elif context_source is None and start_source is not None:
                        start_settings = observation
    except (OSError, ValueError, UnicodeError, StopIteration) as e:
        raise HostError(NOT_ESTABLISHED, "settings_not_established", f"{rollout}: {e}")
    if latest is None:
        raise HostError(NOT_ESTABLISHED, "settings_not_established", f"no matching applied-settings record for {sid}")
    settings, source = latest
    if turn_id is not None:
        if any(value != settings for value, _ in selected_settings) or not selected_contexts or any(
                context["model"] != settings.get("model") or context["effort"] != settings.get("reasoning_effort")
                for context in selected_contexts):
            raise HostError(NOT_ESTABLISHED, "turn_settings_not_established", "selected turn configuration changed or lacks matching attributed context")
    tier = settings.get("service_tier")
    # Native Codex 0.164: config fast is recorded as runtime priority.
    # Future or missing tiers have no established inverse mapping here.
    cli_tier = {"priority": "fast", "default": "default"}.get(tier) if isinstance(tier, str) else None
    model, effort = settings.get("model"), settings.get("reasoning_effort")
    args = None
    if cli_tier is not None and isinstance(model, str) and model and isinstance(effort, str) and effort:
        args = ["-m", model, "-c", "model_reasoning_effort=" + json.dumps(effort),
                "-c", "service_tier=" + json.dumps(cli_tier)]
    return {"session_id": sid, "turn_id": turn_id, "turn_attribution": "exact_turn_interval" if turn_id is not None else "latest_session_observation", "model": model, "reasoning_effort": effort, "service_tier": tier,
            "recorded_cli_version": recorded_version,
            "tier_mapping_basis": "Historical CLI 0.164.0 probe metrics/probes/codex-manual-service-tier-20261028.json; runtime-reported 0.165.0 fixture evidence metrics/probes/outer-context-continuity-20261028.json",
            "source": source, "successor_host_args": args,
            "preservation": "launch_arguments_available" if args is not None else "not_established",
            "limitations": ["Selected-turn settings require an attributed matching turn_context/interval; later session events are excluded." if turn_id is not None else "Latest matching applied record in this bound rollout; later UI changes may not yet be applied.",
                            "Successor settings must be observed after launch and adoption; arguments are a request.",
                            "Runtime configuration does not establish backend fulfillment or billing."]}


def _run(argv, *, cwd=None, timeout=60, env=None):
    try:
        return subprocess.run(argv, cwd=cwd, capture_output=True, text=True,
                              timeout=timeout, stdin=subprocess.DEVNULL, env=env or child_env())
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
    """Codex roles are threads on the shared local app-server daemon.

    A thread started there is one the daemon serves to `codex resume <id>`, with
    no separate process owning its writer. `codex exec`
    launches are no longer made: an exec process owns its thread's writer for the
    whole turn, a root's turn is unbounded, and the daemon then refuses every
    other client (probe metrics/probes/codex-daemon-thread-host-20261026.json).
    Ledger rows written by earlier exec launches carry a pid and are still read.
    """
    name = "codex"
    # exec-style flags accepted as host args, mapped onto thread/start.
    VALUE_FLAGS = {"-m": "model", "--model": "model", "-s": "sandbox", "--sandbox": "sandbox",
                   "-a": "approvalPolicy", "--ask-for-approval": "approvalPolicy",
                   "-c": "config", "--config": "config"}
    PENDING_TURN_GRACE = 30  # seconds a just-started turn may be invisible to reads

    def __init__(self, exe="codex", codex_home=None):
        self.exe = exe
        self.home = Path(codex_home or os.environ.get("CODEX_HOME") or Path.home() / ".codex")
        self.sock = codex_app.default_socket(self.home)

    # ---- transport ----
    def _conn(self):
        try:
            return codex_app.Connection(self.sock, client_name="role-host")
        except codex_app.AppServerError as e:
            raise HostError(NOT_ESTABLISHED, "app_server_unreachable", e.detail)

    def _call(self, conn, method, params):
        try:
            return conn.call(method, params)
        except codex_app.AppServerError as e:
            raise HostError(NOT_ESTABLISHED, "app_server_error", e.detail)

    @classmethod
    def thread_options(cls, host_args) -> tuple[dict, str | None]:
        """Map exec-style host args onto thread/start params and a turn effort."""
        params, config, effort = {"approvalPolicy": "never"}, {}, None
        args = list(host_args)
        while args:
            flag = args.pop(0)
            if flag == "--dangerously-bypass-approvals-and-sandbox":
                params.update(sandbox="danger-full-access", approvalPolicy="never")
                continue
            if flag in ("--json", "--skip-git-repo-check"):
                continue
            key, _, inline = flag.partition("=")
            if key not in cls.VALUE_FLAGS:
                raise HostError(USAGE, "usage", f"codex host arg {flag!r} has no app-server mapping")
            if inline:
                value = inline
            elif args:
                value = args.pop(0)
            else:
                raise HostError(USAGE, "usage", f"codex host arg {flag!r} needs a value")
            field = cls.VALUE_FLAGS[key]
            if field != "config":
                params[field] = value
                continue
            k, eq, raw = value.partition("=")
            if not eq:
                raise HostError(USAGE, "usage", f"-c {value!r} is not key=value")
            try:  # the CLI parses the value as TOML and falls back to the raw string
                v = tomllib.loads(f"v = {raw}")["v"]
            except tomllib.TOMLDecodeError:
                v = raw
            config[k.strip()] = v
            if k.strip() == "model_reasoning_effort":
                effort = str(v)
        if config:
            params["config"] = config
        return params, effort

    @staticmethod
    def _text(message):
        return [{"type": "text", "text": message}]

    # ---- legacy exec rows ----
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

    def _history_status(self, sid):
        """Latest turn status from the on-disk thread history, for unloaded threads."""
        db = self.home / "thread_history_1.sqlite"
        if not db.exists():
            return None
        try:
            con = sqlite3.connect(f"file:{db}?mode=ro", uri=True, timeout=5)
            try:
                row = con.execute("select status from thread_turns where thread_id=? "
                                  "order by rollout_ordinal desc limit 1", (sid,)).fetchone()
            finally:
                con.close()
        except sqlite3.Error as e:
            raise HostError(NOT_ESTABLISHED, "status_not_established", f"thread history: {e}")
        return row[0] if row else None

    # ---- lifecycle ----
    def status(self, sid, ledger=None) -> dict:
        rows = [r for r in ledger.rows() if r.get("session_id") == sid] if ledger else []
        lifecycle = [r for r in rows if r.get("event") not in ("rename", "unarchive")]
        last = lifecycle[-1] if lifecycle else None
        if self._alive((last or {}).get("pid")):
            return {"state": "running", "kind": "exec", "pid": last["pid"]}
        with self._conn() as c:
            try:
                thread = c.call("thread/read", {"threadId": sid})["thread"]
            except codex_app.AppServerError as e:
                if "not loaded" not in e.detail and "not found" not in e.detail:
                    raise HostError(NOT_ESTABLISHED, "status_not_established", e.detail)
                thread = None
            # A thread on disk but not loaded reads as notLoaded, and the
            # daemon then reports its last turn as interrupted even while an
            # exec process is still running it (observed 2026-10-26 for
            # 01a17620). Only the on-disk history is trusted for these.
            if thread is not None and (thread.get("status") or {}).get("type") == "notLoaded":
                thread = None
            if thread is None:
                # Not loaded on the daemon. Another writer (an exec process, or a
                # TUI on its own local runtime) may still hold it.
                hist = self._history_status(sid)
                if hist is None:
                    return {"state": "absent", "kind": None}
                if hist == "inProgress":
                    return {"state": "running", "kind": "unknown_holder"}
                return {"state": "stopped", "kind": "app_server", "last_turn": hist}
            turns = self._call(c, "thread/turns/list", {"threadId": sid, "limit": 1}).get("data", [])
        st = thread.get("status") or {}
        latest = turns[0] if turns else {}
        out = {"kind": "app_server", "thread_status": st.get("type"), "last_turn": latest.get("status"),
               "name": thread.get("name")}
        if st.get("type") == "systemError":
            raise HostError(NOT_ESTABLISHED, "status_not_established", f"thread {sid} is in systemError")
        if st.get("type") == "active" or latest.get("status") == "inProgress":
            return {"state": "running", "active_flags": st.get("activeFlags", []), **out}
        # A turn this ledger just started can be invisible to reads for a moment
        # (measured: thread/read idle and the turn absent 20 ms after turn/start).
        started = [r for r in rows if r.get("turn_id")]
        if started:
            pending = started[-1]
            seen = latest.get("id") == pending["turn_id"] or self._turn_known(sid, pending["turn_id"])
            if not seen and time.time() - pending.get("at_epoch", 0) < self.PENDING_TURN_GRACE:
                return {"state": "running", "pending_turn": pending["turn_id"], **out}
        if last and last.get("event") == "retire":
            return {"state": "stopped", **out}
        return {"state": "idle", **out}

    def _turn_known(self, sid, turn_id):
        with self._conn() as c:
            data = self._call(c, "thread/turns/list", {"threadId": sid, "limit": 20}).get("data", [])
        return any(t.get("id") == turn_id for t in data)

    def _start_turn(self, c, sid, message, effort):
        params = {"threadId": sid, "input": self._text(message)}
        if effort:
            params["effort"] = effort
        turn = self._call(c, "turn/start", params).get("turn") or {}
        if not turn.get("id"):
            raise HostError(NOT_ESTABLISHED, "turn_not_established", f"turn/start on {sid} returned no turn id")
        return turn["id"]

    def launch(self, *, cwd, name, prompt, host_args, ledger, parent=None, start_timeout=60) -> dict:
        params, effort = self.thread_options(host_args)
        with self._conn() as c:
            res = self._call(c, "thread/start", {"cwd": str(Path(cwd).resolve()), **params})
            sid = (res.get("thread") or {}).get("id", "")
            if not UUID.match(sid):
                raise HostError(NOT_ESTABLISHED, "identity_not_established", f"thread/start returned id {sid!r}")
            self._call(c, "thread/name/set", {"threadId": sid, "name": name})
            turn = self._start_turn(c, sid, prompt, effort)
        row = {"host": self.name, "transport": "app_server", "event": "launch", "session_id": sid,
               "name": name, "cwd": str(cwd), "parent_session_id": parent, "turn_id": turn,
               "at_epoch": time.time(), "model": res.get("model"), "effort": effort or res.get("reasoningEffort")}
        ledger.append(row)
        return row

    def prepare(self, *, cwd, name, host_args, ledger, parent=None):
        """Create an idle native thread. The caller journals intent before calling.

        No inference is started here. A lost response still has an unknown
        external effect and must not cause the caller to create a replacement.
        """
        params, effort = self.thread_options(host_args)
        with self._conn() as c:
            response = self._call(c, "thread/start", {"cwd": str(Path(cwd).resolve()), **params})
            thread = response.get("thread") or {}
            sid = thread.get("id", "")
            if not UUID.match(sid):
                raise HostError(NOT_ESTABLISHED, "identity_not_established", "prepare returned no canonical thread id")
            # Keep identity even if naming fails. Naming is cosmetic, not a
            # reason to discard custody of an already created thread.
            naming = None
            try:
                self._call(c, "thread/name/set", {"threadId": sid, "name": name})
            except HostError as exc:
                naming = exc.detail
        row = {"event": "prepare", "host": self.name, "transport": "app_server",
               "session_id": sid, "parent_session_id": parent, "name": name,
               "cwd": str(Path(cwd).resolve()), "effort": effort,
               "requested": params, "realized": response, "naming_error": naming,
               "at_epoch": time.time()}
        ledger.append(row)
        return row

    def start_prepared(self, sid, *, message, ledger, effort=None):
        # The caller has already made the start attempt durable. Do not accept
        # an existing thread with work: the prepared first-turn path is closed.
        with self._conn() as c:
            thread = self._call(c, "thread/read", {"threadId": sid}).get("thread") or {}
            if thread.get("id") != sid or (thread.get("status") or {}).get("type") != "idle":
                raise HostError(NOT_ESTABLISHED, "prepared_identity_not_established", "prepared thread is not verified idle")
            history = self._turns(c, sid)
            if history:
                raise HostError(BUSY, "prepared_thread_has_turns", "prepared thread already has turn history")
            tid = self._start_turn(c, sid, message, effort)
        row = {"event": "start_prepared", "host": self.name, "session_id": sid,
               "turn_id": tid, "at_epoch": time.time()}
        ledger.append(row)
        return row

    def _turns(self, c, sid):
        turns, cursor, seen = [], None, set()
        while True:
            params = {"threadId": sid, "limit": 100, "itemsView": "full"}
            if cursor is not None:
                params["cursor"] = cursor
            page = self._call(c, "thread/turns/list", params)
            data = page.get("data")
            if not isinstance(data, list) or any(not isinstance(t, dict) or not t.get("id") for t in data):
                raise HostError(NOT_ESTABLISHED, "turn_history_not_established", "invalid turn history page")
            turns.extend(data)
            cursor = page.get("nextCursor")
            if not cursor:
                return turns
            if cursor in seen or len(turns) > 100000:
                raise HostError(NOT_ESTABLISHED, "turn_history_not_established", "unbounded/repeating turn history cursor")
            seen.add(cursor)

    def identity(self, sid):
        """Resolve the native rollout by exact session_meta, never environment lineage."""
        with self._conn() as c:
            thread = self._call(c, "thread/read", {"threadId": sid}).get("thread") or {}
        if thread.get("id") != sid:
            raise HostError(NOT_ESTABLISHED, "identity_not_established", "thread/read identity mismatch")
        candidates = []
        if thread.get("path"):
            candidates.append(Path(thread["path"]))
        candidates.extend((self.home / "sessions").glob(f"**/*{sid}*.jsonl"))
        matches = set()
        for path in candidates:
            try:
                with path.open() as fh:
                    meta = json.loads(next(fh))
                if meta.get("type") == "session_meta" and meta.get("payload", {}).get("id") == sid:
                    matches.add(str(path.resolve()))
            except (OSError, ValueError, StopIteration, AttributeError):
                continue
        if len(matches) != 1:
            raise HostError(NOT_ESTABLISHED, "identity_not_established", "no unique verified rollout for native thread")
        return {"session_id": sid, "rollout": matches.pop(), "source": "thread/read plus exact rollout session_meta"}

    def exact_turn(self, sid, turn_id):
        with self._conn() as c:
            thread = self._call(c, "thread/read", {"threadId": sid}).get("thread") or {}
            loading = (thread.get("status") or {}).get("type")
            if thread.get("id") != sid or loading == "systemError":
                raise HostError(NOT_ESTABLISHED, "turn_not_established", "thread identity/read state unavailable")
            # History remains readable when the execution session is unloaded.
            # Both calls are read-only; collection never resumes or starts work.
            turns = self._turns(c, sid)
        matches = [t for t in turns if t["id"] == turn_id]
        if len(matches) != 1:
            return {"session_id": sid, "turn_id": turn_id, "status": "not_established",
                    "reason": "exact turn absent or duplicated", "history_count": len(turns)}
        if loading == "notLoaded" and matches[0].get("status") not in ("completed", "failed", "interrupted"):
            return {"session_id": sid, "turn_id": turn_id, "status": "not_established",
                    "reason": "unloaded history does not establish a current nonterminal turn",
                    "history_count": len(turns), "thread_loading": loading}
        return {"session_id": sid, "turn_id": turn_id, "status": matches[0].get("status"),
                "turn": matches[0], "history_count": len(turns),
                "thread_loading": loading, "source": "thread/read and paged thread/turns/list"}

    def deliver(self, sid, *, message, cwd, ledger, host_args=(), start_timeout=60) -> dict:
        st = self.status(sid, ledger)
        if st["state"] == "running":
            raise HostError(BUSY, "refused_busy", f"{sid} has a turn in progress ({st['kind']})")
        if st["state"] == "absent":
            raise HostError(NOT_ESTABLISHED, "deliver_failed", f"thread {sid} is unknown to this host")
        _, effort = self.thread_options(host_args)
        launched = next((r for r in ledger.rows() if r.get("session_id") == sid and r.get("event") == "launch"), {})
        with self._conn() as c:
            got = (self._call(c, "thread/resume", {"threadId": sid}).get("thread") or {}).get("id")
            if got != sid:
                raise HostError(DEFECT if got else NOT_ESTABLISHED,
                                "identity_mismatch" if got else "deliver_not_established",
                                f"resume of {sid} returned {got or 'no thread'}")
            turn = self._start_turn(c, sid, message, effort or launched.get("effort"))
        ledger.append({"host": self.name, "transport": "app_server", "event": "deliver", "session_id": sid,
                       "turn_id": turn, "at_epoch": time.time()})
        return {"session_id": sid, "delivered": True, "turn_id": turn}

    def retire(self, sid, *, ledger, force=False) -> dict:
        st = self.status(sid, ledger)
        if st["state"] in ("absent", "stopped"):
            return {"session_id": sid, "retired": "already_not_loaded", "state": st["state"]}
        if st["state"] == "running":
            if not force:
                raise HostError(BUSY, "refused_busy", f"{sid} has a turn in progress ({st['kind']})")
            if st["kind"] == "exec":
                os.killpg(st["pid"], signal.SIGTERM)
                ledger.append({"host": self.name, "event": "retire", "session_id": sid, "killed_pid": st["pid"]})
                return {"session_id": sid, "retired": "turn_terminated"}
            if st["kind"] == "unknown_holder":
                raise HostError(NOT_ESTABLISHED, "retire_not_established",
                                "the thread is held outside the app-server daemon; its holder must end it")
            turn_id = st.get("pending_turn")
            with self._conn() as c:
                if not turn_id:
                    data = self._call(c, "thread/turns/list", {"threadId": sid, "limit": 1}).get("data", [])
                    turn_id = data[0]["id"] if data and data[0].get("status") == "inProgress" else None
                if not turn_id:
                    raise HostError(NOT_ESTABLISHED, "retire_not_established", f"no in-progress turn id for {sid}")
                self._call(c, "turn/interrupt", {"threadId": sid, "turnId": turn_id})
            ledger.append({"host": self.name, "event": "retire", "session_id": sid, "interrupted_turn": turn_id})
            return {"session_id": sid, "retired": "turn_interrupted"}
        # Idle: the daemon keeps the thread loaded and has no unload request. The
        # ledger row is what makes status report it stopped from now on; a later
        # deliver gives it work again.
        ledger.append({"host": self.name, "event": "retire", "session_id": sid})
        return {"session_id": sid, "retired": "idle_released"}

    def rename(self, sid, *, name, ledger) -> dict:
        if not isinstance(name, str) or not name.strip():
            raise HostError(USAGE, "usage", "session name must be nonempty")
        with self._conn() as c:
            self._call(c, "thread/name/set", {"threadId": sid, "name": name})
        ledger.append({"host": self.name, "event": "rename", "session_id": sid, "name": name})
        return {"session_id": sid, "name": name}

    def archive(self, sid, *, ledger) -> dict:
        # The server also archives spawned descendants. The caller must collect
        # those first; archiving is not process collection or task completion.
        st = self.status(sid, ledger)
        if st["state"] == "running":
            raise HostError(BUSY, "refused_busy", f"{sid} has a turn in progress")
        if st["state"] == "absent":
            raise HostError(NOT_ESTABLISHED, "archive_not_established", f"unknown thread {sid}")
        with self._conn() as c:
            self._call(c, "thread/archive", {"threadId": sid})
        ledger.append({"host": self.name, "event": "retire", "session_id": sid, "archived": True})
        return {"session_id": sid, "retired": "archived", "descendant_archival": "server_best_effort"}

    def unarchive(self, sid, *, ledger) -> dict:
        with self._conn() as c:
            got = (self._call(c, "thread/unarchive", {"threadId": sid}).get("thread") or {}).get("id")
        if got != sid:
            raise HostError(DEFECT, "identity_mismatch", f"unarchive of {sid} returned {got}")
        ledger.append({"host": self.name, "event": "unarchive", "session_id": sid})
        return {"session_id": sid, "unarchived": True, "resumed": False}


# -------------------------------------------------------------------- pi ----

class Pi:
    """Pi roles are sessions; each turn is one detached `pi --mode json` process.

    The prompt, events, stderr, exit status and service-tier record of turn N
    live in <state>/pi/<session>/turn-N.*. The ledger row carries the turn's
    wrapper pid and start time, so a reused pid never reads as a live turn.
    """
    name = "pi"
    LANES = ("api", "subscription")
    LANE_AUTH = {"api": "api_key", "subscription": "oauth"}
    TIERS = ("auto", "default", "flex", "priority", "scale")
    TIER_PROVIDERS = frozenset({"openai"})
    THINKING = ("off", "minimal", "low", "medium", "high", "xhigh", "max")
    # Pi flags passed through. Session, mode, credential and provider selection
    # are role_host's, so they are not accepted here.
    VALUE_FLAGS = {"--model": "--model", "--thinking": "--thinking", "--tools": "--tools", "-t": "--tools",
                   "--exclude-tools": "--exclude-tools", "-xt": "--exclude-tools", "--skill": "--skill",
                   "--append-system-prompt": "--append-system-prompt", "--extension": "--extension", "-e": "--extension"}
    BOOL_FLAGS = {"--no-mcp", "--no-skills", "-ns", "--no-extensions", "-ne", "--no-context-files", "-nc",
                  "--approve", "-a", "--no-approve", "-na"}
    EXTENSION = Path(__file__).resolve().parent / "pi_service_tier.ts"
    START_TIMEOUT = 30
    STOP_WAIT = 10
    # The wrapper stays the process-group leader until pi exits, then writes
    # pi's exit status atomically. A killed turn leaves no status file.
    WRAPPER = ('prompt=$1 out=$2 err=$3 status=$4; shift 4; '
               '"$@" < "$prompt" > "$out" 2> "$err"; echo $? > "$status.tmp" && mv "$status.tmp" "$status"')
    TURN_FILES = {"prompt": "prompt", "events": "events.jsonl", "stderr": "stderr",
                  "exit": "exit", "tier_record": "tier.jsonl"}

    def __init__(self, exe="pi"):
        self.exe = exe

    # ---- options and credentials ----
    @classmethod
    def options(cls, host_args, lane, tier) -> dict:
        """Validate a launch request; returns the pass-through argv and the selection."""
        if lane not in cls.LANES:
            raise HostError(USAGE, "usage", "a pi launch needs --auth api or --auth subscription")
        argv, model, thinking = [], None, None
        args = list(host_args)
        while args:
            flag = args.pop(0)
            key, eq, inline = flag.partition("=")
            if key in cls.BOOL_FLAGS and not eq:
                argv.append(key)
                continue
            if key not in cls.VALUE_FLAGS:
                raise HostError(USAGE, "usage", f"pi host arg {flag!r} is not accepted here")
            if eq:
                value = inline
            elif args:
                value = args.pop(0)
            else:
                raise HostError(USAGE, "usage", f"pi host arg {flag!r} needs a value")
            canonical = cls.VALUE_FLAGS[key]
            if canonical == "--model":
                if model is not None:
                    raise HostError(USAGE, "usage", "--model given twice")
                model = value
            elif canonical == "--thinking":
                if value not in cls.THINKING:
                    raise HostError(USAGE, "usage", f"--thinking {value!r} is not one of {', '.join(cls.THINKING)}")
                thinking = value
            argv += [canonical, value]
        if not model:
            raise HostError(USAGE, "usage", "a pi launch needs --host-arg=--model --host-arg=<provider>/<model-id>")
        provider, slash, model_id = model.partition("/")
        if not slash or not provider or not model_id or any(c in model for c in "*?, "):
            raise HostError(USAGE, "usage", f"--model {model!r} must be one exact <provider>/<model-id>, not a pattern")
        if ":" in model_id and model_id.rsplit(":", 1)[1] in cls.THINKING:
            raise HostError(USAGE, "usage", "give the thinking level with --thinking, not a :<level> model suffix")
        if tier is not None:
            if tier not in cls.TIERS:
                raise HostError(USAGE, "usage", f"service tier {tier!r} is not one of {', '.join(cls.TIERS)}")
            if lane != "api":
                raise HostError(USAGE, "usage", "a service tier applies only to api-lane requests; "
                                "subscription (session-quota) requests do not take one")
            if provider not in cls.TIER_PROVIDERS:
                raise HostError(USAGE, "usage", f"a service tier is supported here only for provider "
                                f"{', '.join(sorted(cls.TIER_PROVIDERS))}, not {provider!r}")
        return {"argv": argv, "model": model, "provider": provider, "thinking": thinking,
                "lane": lane, "tier": tier}

    def check_lane(self, opts, *, cwd, env) -> dict:
        """Confirm Pi's credential for the provider is the lane's kind. Never refreshes or prints it."""
        provider, lane = opts["provider"], opts["lane"]
        r = _run([self.exe, "auth", "check", "--provider", provider, "--json", "--no-refresh"],
                 cwd=cwd, env=env, timeout=60)
        try:
            got = json.loads(r.stdout.strip().splitlines()[-1])
        except (ValueError, IndexError):
            raise HostError(NOT_ESTABLISHED, "credential_not_established",
                            f"pi auth check exit {r.returncode}: {r.stderr.strip()[:200]}")
        if not isinstance(got, dict) or got.get("status") != "ready":
            raise HostError(NOT_ESTABLISHED, "credential_not_ready",
                            f"pi auth check for {provider}: {got.get('status') if isinstance(got, dict) else got} "
                            f"{got.get('reason', '') if isinstance(got, dict) else ''}".strip())
        want = self.LANE_AUTH[lane]
        if got.get("authType") != want:
            raise HostError(NOT_ESTABLISHED, "lane_mismatch",
                            f"the {lane} lane needs a {want} credential for {provider}; Pi reports {got.get('authType')!r}")
        # Pi 1.4.0 sends a non-"sk-" OPENAI_API_KEY to OpenAI as a Sign in with
        # ChatGPT token (openai-responses.js isChatGPTSignIn): subscription
        # billing under an api label. Only the prefix is read.
        key = env.get("OPENAI_API_KEY")
        if lane == "api" and provider == "openai" and key and not key.startswith("sk-"):
            raise HostError(NOT_ESTABLISHED, "lane_mismatch",
                            "OPENAI_API_KEY is not an sk- API key; Pi would send it as a ChatGPT sign-in token")
        return {"provider": provider, "auth_type": got["authType"]}

    # ---- processes ----
    @staticmethod
    def _start_time(pid):
        try:
            fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        except (OSError, IndexError):
            return None
        return None if fields[0] == "Z" else int(fields[19])

    def _alive(self, row) -> bool:
        pid = row.get("pid")
        return bool(pid) and row.get("pid_start") is not None and self._start_time(pid) == row["pid_start"]

    @staticmethod
    def _group_alive(pgid) -> bool:
        try:
            os.killpg(pgid, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return True
        return True

    @staticmethod
    def _records(path: Path) -> tuple[list[dict], bool]:
        """Complete LF-terminated JSON records; True when a partial last line was left out."""
        try:
            data = path.read_bytes()
        except OSError:
            return [], False
        lines = data.split(b"\n")
        partial = lines.pop() != b""
        out = []
        for n, raw in enumerate(lines, 1):
            if not raw.strip():
                continue
            try:
                value = json.loads(raw)
            except ValueError:
                raise HostError(NOT_ESTABLISHED, "turn_not_established", f"{path}:{n} is not JSON")
            if isinstance(value, dict):
                out.append(value)
        return out, partial

    def _turns(self, sid, ledger):
        return [r for r in ledger.rows() if r.get("session_id") == sid and r.get("host") == self.name]

    def _spawn(self, sid, opts, *, cwd, prompt, ledger, event, name=None, parent=None):
        turn = sum(1 for r in self._turns(sid, ledger) if r.get("turn")) + 1
        folder = ledger.path.parent / "pi" / sid
        folder.mkdir(parents=True, exist_ok=True)
        files = {k: folder / f"turn-{turn}.{suffix}" for k, suffix in self.TURN_FILES.items()}
        try:
            with open(files["prompt"], "x") as fh:
                fh.write(prompt)
        except FileExistsError:
            raise HostError(NOT_ESTABLISHED, "turn_not_established", f"{files['prompt']} already exists")
        argv = [self.exe, "--mode", "json", "--session-id", sid, "-e", str(self.EXTENSION), *opts["argv"]]
        if name:
            argv += ["--name", name]
        env = {**child_env(), "WORK_ENGINE_PI_SERVICE_TIER": opts["tier"] or "",
               "WORK_ENGINE_PI_SERVICE_TIER_MODEL": opts["model"],
               "WORK_ENGINE_PI_SERVICE_TIER_RECORD": str(files["tier_record"])}
        try:
            proc = subprocess.Popen(["/bin/sh", "-c", self.WRAPPER, "sh",
                                     *(str(files[k]) for k in ("prompt", "events", "stderr", "exit")), *argv],
                                    cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                    stderr=subprocess.DEVNULL, start_new_session=True)
        except OSError as e:
            raise HostError(NOT_ESTABLISHED, "fail_launch", f"pi turn: {e}")
        row = {"host": self.name, "event": event, "session_id": sid, "turn": turn, "pid": proc.pid,
               "pid_start": self._start_time(proc.pid), "at_epoch": time.time(),
               "files": {k: str(v) for k, v in files.items()}, "model": opts["model"],
               "thinking": opts["thinking"], "auth_lane": opts["lane"], "service_tier": opts["tier"],
               "host_argv": opts["argv"]}
        if event == "launch":
            row.update(name=name, cwd=str(Path(cwd).resolve()), parent_session_id=parent)
        ledger.append(row)
        self._verify_header(proc, row, files, ledger)
        return row

    def _verify_header(self, proc, row, files, ledger):
        """The turn's first record must be the session header for the requested id."""
        sid, deadline = row["session_id"], time.time() + self.START_TIMEOUT
        while True:
            exited = proc.poll() is not None  # read before the events, so an exit is never ahead of them
            records, _ = self._records(files["events"])
            if records:
                first = records[0]
                if first.get("type") == "session" and first.get("id") == sid:
                    return
                if self._alive(row):
                    os.killpg(row["pid"], signal.SIGTERM)
                ledger.append({"host": self.name, "event": "identity_defect", "session_id": sid,
                               "turn": row["turn"], "first_record": {k: first.get(k) for k in ("type", "id")}})
                raise HostError(DEFECT, "identity_mismatch",
                                f"turn {row['turn']} began with {first.get('type')} {first.get('id')!r}, not session {sid}; turn stopped")
            if exited:
                err = files["stderr"].read_text(errors="replace").strip() if files["stderr"].exists() else ""
                raise HostError(NOT_ESTABLISHED, "fail_launch",
                                f"pi exited {proc.returncode} before a session header: {err[-300:]}")
            if time.time() > deadline:
                raise HostError(NOT_ESTABLISHED, "identity_not_established",
                                f"no session header within {self.START_TIMEOUT}s; turn {row['turn']} is in the ledger "
                                "and may still be running (retire --force stops it)")
            time.sleep(0.1)

    # ---- observation ----
    def session_dir(self) -> Path:
        if os.environ.get("PI_CODING_AGENT_SESSION_DIR"):
            return Path(os.environ["PI_CODING_AGENT_SESSION_DIR"])
        agent = os.environ.get("PI_CODING_AGENT_DIR") or str(Path.home() / ".pi/agent")
        return Path(agent) / "sessions"

    def session_file(self, sid) -> Path | None:
        matches = set()
        for path in self.session_dir().glob(f"*/*{sid}*.jsonl"):
            try:
                with path.open() as fh:
                    head = json.loads(fh.readline())
            except (OSError, ValueError):
                continue
            if isinstance(head, dict) and head.get("type") == "session" and head.get("id") == sid:
                matches.add(path.resolve())
        if len(matches) > 1:
            raise HostError(NOT_ESTABLISHED, "identity_not_established", f"{len(matches)} session files carry {sid}")
        return matches.pop() if matches else None

    def identity(self, sid) -> dict:
        path = self.session_file(sid)
        if path is None:
            raise HostError(NOT_ESTABLISHED, "identity_not_established",
                            f"no session file under {self.session_dir()} begins with session {sid}")
        return {"session_id": sid, "session_file": str(path), "source": "session file header id"}

    def observe_turn(self, row) -> dict:
        """Outcome, model, cost and tier of one turn, from its own files only."""
        files = {k: Path(v) for k, v in row["files"].items()}
        try:
            exit_code = int(files["exit"].read_text().strip())
        except (OSError, ValueError):
            exit_code = None
        records, partial = self._records(files["events"])
        header = records[0] if records and records[0].get("type") == "session" else None
        settled = next((r for r in reversed(records) if r.get("type") == "agent_settled"), None)
        replies = [r["message"] for r in records if r.get("type") == "message_end"
                   and isinstance(r.get("message"), dict) and r["message"].get("role") == "assistant"]
        final = replies[-1] if replies else {}
        if header is None or header.get("id") != row["session_id"]:
            outcome = "not_established"
        elif exit_code is None:
            outcome = "interrupted"
        elif exit_code != 0:
            outcome = "failed"
        elif settled is None:
            outcome = "not_established"
        elif settled.get("aborted"):
            outcome = "aborted"
        elif final.get("stopReason") in ("error", "aborted"):
            outcome = "failed"
        elif final.get("stopReason") == "length":
            outcome = "incomplete"
        elif not replies:
            outcome = "not_established"
        else:
            outcome = "completed"
        served = sorted({f"{m.get('provider')}/{m.get('model')}" for m in replies})
        cost = sum(((m.get("usage") or {}).get("cost") or {}).get("total") or 0 for m in replies)
        tier_rows, _ = self._records(files["tier_record"])
        injected = [r for r in tier_rows if r.get("event") == "injected"]
        observed = [r.get("tier") for r in tier_rows if r.get("event") == "observed"]
        requested = row.get("service_tier")
        if not requested:
            tier_status = "not_requested"
        elif not injected:
            tier_status = "not_applied"
        elif not observed or None in observed:
            tier_status = "not_established"
        elif requested != "auto" and any(t != requested for t in observed):
            tier_status = "mismatch"
        else:
            tier_status = "applied"
        return {
            "turn": row["turn"], "outcome": outcome, "exit_code": exit_code, "partial_last_record": partial,
            "stop_reason": final.get("stopReason"), "error": final.get("errorMessage"),
            "model_requested": row.get("model"), "model_served": served,
            "model_match": (served == [row.get("model")]) if served else None,
            "assistant_messages": len(replies),
            "pi_reported_cost_usd": round(cost, 6),
            "service_tier": {"requested": requested, "status": tier_status, "injected": len(injected),
                             "observed": observed,
                             "skipped": sorted({r.get("reason") for r in tier_rows if r.get("event") == "skipped"})},
            "files": row["files"],
            "limitations": ["Cost is Pi's catalog price times Pi's tier multiplier, not billing.",
                            "The tier is observed from the provider's terminal response event as Pi parsed it."],
        }

    # ---- lifecycle ----
    def status(self, sid, ledger=None) -> dict:
        rows = self._turns(sid, ledger) if ledger else []
        turns = [r for r in rows if r.get("turn")]
        if not turns:
            if self.session_file(sid) is None:
                return {"state": "absent", "kind": None}
            raise HostError(NOT_ESTABLISHED, "status_not_established",
                            f"pi session {sid} exists but role_host started none of its turns; its holder is not observable here")
        last = turns[-1]
        if self._alive(last):
            return {"state": "running", "kind": "pi_turn", "turn": last["turn"], "pid": last["pid"]}
        lifecycle = [r for r in rows if r.get("event") in ("launch", "deliver", "retire")]
        state = "stopped" if lifecycle[-1].get("event") == "retire" else "idle"
        if state == "idle" and self.session_file(sid) is None:
            state = "absent"
        return {"state": state, "kind": "pi_turn", "last_turn": self.observe_turn(last)}

    def launch(self, *, cwd, name, prompt, host_args, ledger, parent=None, lane=None, tier=None) -> dict:
        opts = self.options(host_args, lane, tier)
        auth = self.check_lane(opts, cwd=cwd, env=child_env())
        sid = str(uuid.uuid4())
        row = self._spawn(sid, opts, cwd=cwd, prompt=prompt, ledger=ledger, event="launch", name=name, parent=parent)
        return {**row, "credential": auth, "session_header_verified": True}

    def deliver(self, sid, *, message, cwd, ledger, host_args=()) -> dict:
        if host_args:
            raise HostError(USAGE, "usage", "pi delivery reuses the launch settings; launch a new session to change them")
        st = self.status(sid, ledger)
        if st["state"] == "running":
            raise HostError(BUSY, "refused_busy", f"{sid} has turn {st['turn']} in progress")
        if st["state"] == "absent":
            raise HostError(NOT_ESTABLISHED, "deliver_failed", f"pi session {sid} is unknown to this host")
        launch = next(r for r in self._turns(sid, ledger) if r.get("event") == "launch")
        # Pi resolves --session-id within the working directory's project; from
        # another directory it would silently create a new session.
        if Path(cwd).resolve() != Path(launch["cwd"]):
            raise HostError(USAGE, "usage", f"pi sessions belong to their working directory; deliver with --cwd {launch['cwd']}")
        opts = {"argv": launch["host_argv"], "model": launch["model"], "provider": launch["model"].split("/", 1)[0],
                "thinking": launch["thinking"], "lane": launch["auth_lane"], "tier": launch["service_tier"]}
        auth = self.check_lane(opts, cwd=cwd, env=child_env())
        row = self._spawn(sid, opts, cwd=cwd, prompt=message, ledger=ledger, event="deliver")
        return {"session_id": sid, "delivered": True, "turn": row["turn"], "credential": auth}

    def retire(self, sid, *, ledger, force=False) -> dict:
        st = self.status(sid, ledger)
        if st["state"] in ("absent", "stopped"):
            return {"session_id": sid, "retired": "already_not_loaded", "state": st["state"]}
        if st["state"] == "running":
            if not force:
                raise HostError(BUSY, "refused_busy", f"{sid} has turn {st['turn']} in progress")
            last = [r for r in self._turns(sid, ledger) if r.get("turn")][-1]
            # SIGTERM only: Pi kills the tool process trees it started detached
            # on SIGTERM (utils/shell.js); SIGKILL would leave them running.
            try:
                os.killpg(last["pid"], signal.SIGTERM)
            except ProcessLookupError:
                pass
            deadline = time.time() + self.STOP_WAIT
            while self._group_alive(last["pid"]):
                if time.time() > deadline:
                    raise HostError(NOT_ESTABLISHED, "retire_not_established",
                                    f"turn {last['turn']} process group {last['pid']} still alive {self.STOP_WAIT}s after SIGTERM")
                time.sleep(0.2)
            ledger.append({"host": self.name, "event": "retire", "session_id": sid, "terminated_turn": last["turn"]})
            return {"session_id": sid, "retired": "turn_terminated", "turn": last["turn"]}
        ledger.append({"host": self.name, "event": "retire", "session_id": sid})
        return {"session_id": sid, "retired": "idle_released"}


HOSTS = {"claude": Claude, "codex": Codex, "pi": Pi}


def host(name):
    if name not in HOSTS:
        raise HostError(USAGE, "usage", f"unknown host {name!r}")
    exe = os.environ.get(f"ROLE_HOST_{name.upper()}_EXE", name)
    return HOSTS[name](exe)


def children(parent, ledger) -> dict:
    """Ledger children of `parent`, each with its current status."""
    launched = [r for r in ledger.rows() if r.get("event") in ("launch", "prepare") and r.get("parent_session_id") == parent]
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
    la.add_argument("--auth", choices=Pi.LANES,
                    help="pi only, required: api (API-key billing) or subscription (OAuth session quota)")
    la.add_argument("--service-tier", choices=Pi.TIERS,
                    help="pi only: OpenAI service tier for an api-lane openai model")
    pr = sub.add_parser("prepare", help="create an idle Codex thread, without inference")
    pr.add_argument("host", choices=("codex",)); pr.add_argument("--cwd", required=True)
    pr.add_argument("--name", required=True); pr.add_argument("--parent")
    pr.add_argument("--host-arg", action="append", default=[])
    st = sub.add_parser("start-prepared", help="start the first turn on an idle prepared Codex thread")
    st.add_argument("host", choices=("codex",)); st.add_argument("session_id")
    st.add_argument("--prompt-file", required=True, type=Path); st.add_argument("--effort")
    ident = sub.add_parser("identity", help="resolve a native rollout (Codex) or session file (Pi) by its recorded id")
    ident.add_argument("host", choices=("codex", "pi")); ident.add_argument("session_id")
    ex = sub.add_parser("exact-turn", help="observe one exact native turn, paging history")
    ex.add_argument("host", choices=("codex",)); ex.add_argument("session_id"); ex.add_argument("--turn-id", required=True)
    for verb in ("status", "retire"):
        s = sub.add_parser(verb); s.add_argument("host"); s.add_argument("session_id")
        if verb == "retire":
            s.add_argument("--force", action="store_true")
    settings = sub.add_parser("settings", help="read a Codex session's applied model, effort and tier from its bound rollout")
    settings.add_argument("host"); settings.add_argument("session_id")
    settings.add_argument("--rollout", required=True, type=Path)
    settings.add_argument("--turn-id", help="bind settings to an exact native turn interval")
    rename = sub.add_parser("rename", help="rename a Codex thread without starting a turn")
    rename.add_argument("host"); rename.add_argument("session_id"); rename.add_argument("--name", required=True)
    for verb in ("archive", "unarchive"):
        s = sub.add_parser(verb, help="Codex history management; no model turn")
        s.add_argument("host"); s.add_argument("session_id")
        if verb == "archive":
            s.add_argument("--descendants-collected", action="store_true", required=True,
                           help="caller attests all spawned descendants/results have been collected")
    de = sub.add_parser("deliver")
    de.add_argument("host"); de.add_argument("session_id"); de.add_argument("--cwd", required=True)
    de.add_argument("--message-file", required=True, type=Path)
    de.add_argument("--host-arg", action="append", default=[])
    ch = sub.add_parser("children"); ch.add_argument("parent")
    sub.add_parser("whoami", help="print this process's own session id from the host's environment")
    a = ap.parse_args(argv)
    try:
        if a.cmd == "whoami":
            found = [(h, os.environ[var], var) for var, h in
                     (("CLAUDE_CODE_SESSION_ID", "claude"), ("CODEX_THREAD_ID", "codex"), ("PI_SESSION_ID", "pi"))
                     if UUID.match(os.environ.get(var, ""))]
            # Both set means one was inherited from whoever launched this
            # session, and nothing here can tell which. Refuse rather than pick.
            if len(found) != 1:
                raise HostError(NOT_ESTABLISHED, "identity_not_established",
                                f"{len(found)} host session ids in environment: {[f[2] for f in found]}")
            h, v, var = found[0]
            print(json.dumps({"host": h, "session_id": v, "source": var})); return OK
        ledger = Ledger(a.state)
        if a.cmd == "children":
            res = children(a.parent, ledger)
        else:
            sid = getattr(a, "session_id", None)
            if sid is not None and not UUID.match(sid):
                raise HostError(USAGE, "usage", f"session id {sid!r} is not a lowercase canonical uuid")
            h = host(a.host)
            if a.cmd == "settings":
                if a.host != "codex":
                    raise HostError(USAGE, "unsupported_control", "applied runtime settings reader is Codex-only")
                res = codex_settings(sid, a.rollout, a.turn_id)
            elif a.cmd == "prepare":
                res = h.prepare(cwd=a.cwd, name=a.name, host_args=a.host_arg, ledger=ledger, parent=a.parent)
            elif a.cmd == "start-prepared":
                res = h.start_prepared(sid, message=a.prompt_file.read_text(), ledger=ledger, effort=a.effort)
            elif a.cmd == "identity":
                res = h.identity(sid)
            elif a.cmd == "exact-turn":
                res = h.exact_turn(sid, a.turn_id)
            elif a.cmd == "launch":
                kw = {}
                if a.host == "pi":
                    kw = {"lane": a.auth, "tier": a.service_tier}
                elif a.auth or a.service_tier:
                    raise HostError(USAGE, "usage", "--auth and --service-tier are pi launch options; "
                                    "Codex selects its tier with -c service_tier")
                res = h.launch(cwd=a.cwd, name=a.name, prompt=a.prompt_file.read_text(),
                               host_args=a.host_arg, ledger=ledger, parent=a.parent, **kw)
            elif a.cmd == "status":
                res = {"session_id": sid, "host": a.host, **h.status(sid, ledger)}
            elif a.cmd == "deliver":
                kw = {"host_args": a.host_arg} if a.host in ("codex", "pi") else {}
                res = h.deliver(sid, message=a.message_file.read_text(), cwd=a.cwd, ledger=ledger, **kw)
            elif a.cmd in ("rename", "archive", "unarchive"):
                if a.host == "pi":
                    raise HostError(USAGE, "unsupported_control", "Pi names a session with --name at launch; "
                                    "external rename/archive is not established by this adapter.")
                if a.host != "codex":
                    raise HostError(USAGE, "unsupported_control", "Claude supports --name, current-session /rename, and stop; "
                                    "external rename/archive is not established by this adapter. rm may remove a worktree.")
                kw = {"name": a.name} if a.cmd == "rename" else {}
                res = getattr(h, a.cmd)(sid, ledger=ledger, **kw)
            else:
                res = h.retire(sid, ledger=ledger, force=a.force)
        print(json.dumps(res, sort_keys=True))
        return OK
    except HostError as e:
        print(json.dumps({"status": e.status, "detail": e.detail}), file=sys.stderr)
        return e.code


if __name__ == "__main__":
    sys.exit(main())
