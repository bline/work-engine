#!/usr/bin/env python3
"""Root rotation gate over CLI-hosted roles (Claude `--bg` or Codex `exec`).

The root prepares its handoff; this gate validates it, establishes quiescence,
stops at the authority gate, and on an authorized commit launches the
successor, posts the handoff to the successor's own board topic, waits for the
successor's typed `adopt`, and then retires the predecessor if asked. No model
call happens before the gate. Nothing here grants transfer authority: `prepare`
reaching the gate means validated and quiescent, not permitted.

Ported from the retired tmux coordinator (metrics/scripts/root_rotation_coordinator.sh,
probe metrics/probes/tmux-root-rotation-20261024.md). Every refusal it had is
kept except those that existed only because of tmux windows. Fail closed: an
outcome that cannot be established is recorded as not established and nothing
proceeds on it.

Exit codes (terminal-status in --state):
   0 committed_rotation_N        10 awaiting_authority        20 fail_cap_reached
  21 fail_validation_<phase>     22 quiescence failures, joined by '+'
  23 fail_launch                 24 fail_predecessor_session_form
  25 fail_role_grammar           26 fail_fork_detected        29 nothing to commit
  30 fail_handoff_mutated        32 *_not_established         33 fail_board_post
  34 fail_no_adoption            38 committed_rotation_N_predecessor_not_retired
  39 fail_role_ambiguous         77 watch_refused_no_authority
  78 watch_expired               79 watch_refused_no_digest   64 usage
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import role_host  # noqa: E402

HERE = Path(__file__).resolve().parent
UUID = role_host.UUID
ROLE_RX = re.compile(r"^[a-z0-9][a-z0-9._-]{0,63}$")
SESSION_PROC = re.compile(r"(^|/|\s)(claude|codex)(\s|$)")


class Stop(Exception):
    def __init__(self, code, status, msg):
        super().__init__(msg)
        self.code, self.status, self.msg = code, status, msg


class Gate:
    def __init__(self, a):
        self.a = a
        self.state = Path(a.state)
        self.state.mkdir(parents=True, exist_ok=True)
        self.ledger = role_host.Ledger(Path(a.host_state))
        self.count_file = self.state / "rotation-count"
        if not self.count_file.exists():
            self.count_file.write_text("0\n")

    # ------------------------------------------------------------ records --
    def log(self, msg):
        line = f"{datetime.datetime.now(datetime.UTC):%Y-%m-%dT%H:%M:%S.%f}Z [gate:{self.a.role}] {msg}"
        with open(self.state / "supervisor.log", "a") as fh:
            fh.write(line + "\n")
        print(line, file=sys.stderr)

    def rec(self, name, value):
        (self.state / name).write_text(value + "\n")

    def read_rec(self, name):
        p = self.state / name
        return p.read_text().strip() if p.exists() else ""

    def count(self):
        return int(self.count_file.read_text().strip() or 0)

    # -------------------------------------------------------------- board --
    def board(self, *args, timeout=120):
        argv = ["python3", self.a.board_cmd, "--repository", self.a.repo, *args]
        try:
            r = subprocess.run(argv, capture_output=True, text=True, timeout=timeout,
                               stdin=subprocess.DEVNULL)
        except (OSError, subprocess.TimeoutExpired) as e:
            return 1, None, str(e)
        doc = None
        if r.stdout.strip():
            try:
                doc = json.loads(r.stdout)
            except ValueError:
                pass
        return r.returncode, doc, r.stderr.strip()

    def cursor(self):
        rc, doc, err = self.board("read", "--limit", "1")
        msgs = (doc or {}).get("messages")
        if rc != 0 or msgs is None:
            raise Stop(32, "board_not_established", f"cannot read the board cursor: {err[:200]}")
        return msgs[-1]["sequence"] if msgs else 0

    # ------------------------------------------------------------ checks --
    def check_inputs(self):
        # adopt REJECTS non-canonical values rather than normalising them, so a
        # bad value surfaces only at the adoption timeout unless refused here.
        if not UUID.match(self.a.pred_session):
            raise Stop(24, "fail_predecessor_session_form",
                       f"pred session {self.a.pred_session!r} is not a lowercase canonical uuid")
        if not ROLE_RX.match(self.a.role):
            raise Stop(25, "fail_role_grammar", f"role {self.a.role!r} breaks the board role grammar")

    def check_role_unambiguous(self):
        # The display name is not an identity: two loaded Claude sessions may
        # share one. A second loaded session already named as the role is the
        # state a failed retirement or a host-level copy leaves behind. Codex
        # threads carry no display name, so there is nothing to check there.
        if "claude" not in (self.a.pred_host, self.a.successor_host):
            return
        try:
            rows = role_host.host("claude")._agents(False)
        except role_host.HostError as e:
            raise Stop(32, "role_not_established", f"cannot list Claude sessions: {e.detail}")
        others = [r.get("sessionId") for r in rows
                  if r.get("name") == self.a.role and r.get("sessionId") != self.a.pred_session]
        if others:
            raise Stop(39, "fail_role_ambiguous",
                       f"{len(others)} other loaded session(s) are named {self.a.role}: {others}")

    def validate(self, handoff, phase):
        argv = shlex.split(self.a.validator) + [str(handoff)]
        with open(self.state / "validator.log", "a") as fh:
            try:
                rc = subprocess.run(argv, stdout=fh, stderr=fh, timeout=300).returncode
            except (OSError, subprocess.TimeoutExpired) as e:
                fh.write(f"validator could not run: {e}\n")
                rc = -1
        if rc == 0:
            self.log(f"validator: pass ({phase})")
            return
        if rc == 21:
            raise Stop(21, f"fail_validation_{phase}", "validator found a defect in the handoff")
        raise Stop(32, f"validation_not_established_{phase}", f"validator exit {rc} is not a pass")

    def _claims(self):
        rc, doc, err = self.board("read", "--active-claims", "--limit", "1")
        claims = (doc or {}).get("claims")
        if rc != 0 or not isinstance(claims, dict):
            raise Stop(32, "quiescence_not_established", f"claims unreadable: {err[:200]}")
        now = datetime.datetime.now(datetime.UTC)
        live = {}
        for res, c in claims.items():
            exp = c.get("expires_at")
            if exp:
                try:
                    if datetime.datetime.fromisoformat(exp.replace("Z", "+00:00")) <= now:
                        continue
                except ValueError:
                    raise Stop(32, "quiescence_not_established", f"unparseable expires_at on {res!r}")
            live[res] = c
        return live

    def _pred_pid(self):
        if self.a.pred_host != "claude":
            return None
        try:
            row = role_host.host("claude")._row(self.a.pred_session)
        except role_host.HostError as e:
            raise Stop(32, "quiescence_not_established", f"predecessor process unreadable: {e.detail}")
        return (row or {}).get("pid")

    def _descendants(self, pid):
        """Session processes beneath the predecessor's own process (upper bound)."""
        r = subprocess.run(["ps", "-eo", "pid=,ppid=,args="], capture_output=True, text=True, timeout=60)
        if r.returncode != 0:
            raise Stop(32, "quiescence_not_established", "ps failed")
        kids, args = {}, {}
        for line in r.stdout.splitlines():
            parts = line.split(None, 2)
            if len(parts) == 3 and parts[0].isdigit() and parts[1].isdigit():
                kids.setdefault(int(parts[1]), []).append(int(parts[0]))
                args[int(parts[0])] = parts[2]
        if pid not in args:
            raise Stop(32, "quiescence_not_established", f"predecessor pid {pid} not in the process table")
        seen, stack = set(), [pid]
        while stack:
            for k in kids.get(stack.pop(), ()):
                if k not in seen:
                    seen.add(k)
                    stack.append(k)
        me = os.getpid()
        return sum(1 for p in seen if p != me and SESSION_PROC.search(args.get(p, "")))

    def quiescent(self):
        qf = self.a.quiescence_file
        if not qf or not Path(qf).is_file() or Path(qf).stat().st_size == 0:
            raise Stop(32, "quiescence_not_established", "predecessor has not asserted quiescence")
        claims = self._claims()
        held = sorted(r for r, c in claims.items() if c.get("session_id") == self.a.pred_session)
        claimed_children = {c.get("session_id") for r, c in claims.items()
                            if c.get("session_id") not in (self.a.pred_session, None)
                            and r.startswith(self.a.resource_prefix)} if self.a.resource_prefix else set()
        kids = role_host.children(self.a.pred_session, self.ledger)
        if kids["not_established"]:
            raise Stop(32, "quiescence_not_established",
                       f"{kids['not_established']} ledger child status(es) unreadable")
        pid = self._pred_pid()
        desc = self._descendants(pid) if pid else 0
        why = []
        if kids["running"] or kids["idle_loaded"]:
            self.log(f"ledger children still loaded: {kids['running']} running, {kids['idle_loaded']} idle")
            why.append("fail_children_live")
        if claimed_children:
            self.log(f"{len(claimed_children)} other session(s) hold claims under {self.a.resource_prefix!r}")
            why.append("fail_claimed_children_live")
        if desc:
            self.log(f"{desc} session process(es) beneath predecessor pid {pid}")
            why.append("fail_descendants_live")
        if held:
            self.log(f"predecessor still holds claims: {held}")
            why.append("fail_claims_held")
        if why:
            raise Stop(22, "+".join(why), "not quiescent")
        self.log(f"quiescence established: 0 ledger children loaded, 0 claims held, "
                 f"{'0 descendants of pid ' + str(pid) if pid else 'no predecessor process to descend from'}")

    # --------------------------------------------------------- operations --
    def prepare(self, handoff):
        self.check_inputs()
        self.check_role_unambiguous()
        if self.count() >= self.a.max_rotations:
            raise Stop(20, "fail_cap_reached", f"cap {self.count()}/{self.a.max_rotations}")
        self.validate(handoff, "prepare")
        dig = sha256(handoff)
        self.quiescent()
        self.rec("validation-record", f"handoff={handoff} sha256={dig} validated_at={now()}")
        if self.a.gate == "require":
            self.rec("awaiting-authority", f"handoff={handoff} sha256={dig}")
            raise Stop(10, "awaiting_authority", "validated and quiescent; awaiting authority. NO successor exists.")
        return self.commit(handoff)

    def successor_prompt(self, handoff, dig, cursor):
        rh, cb, repo = HERE / "role_host.py", self.a.board_cmd, self.a.repo
        tail = ("Then read the handoff file in full and continue as this role, starting from its "
                "first actions for the successor." if self.a.after_adopt == "continue" else
                "Then report what you did in two or three lines and take no further action.")
        return f"""You are the successor root for role {self.a.role}. Your handoff is being posted to the coordination board for you. Do these steps in order.

1. Find your own session id:
   python3 {rh} whoami
   Use its session_id as SID below. If it fails, say so and stop.

2. Collect your handoff (this blocks until it arrives):
   python3 {cb} --repository {repo} wait --since {cursor} --topic session:SID --timeout-seconds 300
   The message on topic session:SID references {handoff}.

3. Verify it:
   sha256sum {handoff}
   The expected digest is {dig}. If it does not match, say so and POST NOTHING TO THE BOARD.

4. Reconcile claims before taking ownership:
   python3 {cb} --repository {repo} read --active-claims --limit 10
   Note any claim that still names session {self.a.pred_session}.

5. Acknowledge adoption with the dedicated command (`post` cannot write an adoption):
   python3 {cb} --repository {repo} adopt --author successor-root --session-id SID --role {self.a.role} --predecessor-session {self.a.pred_session} --handoff-sha256 {dig} --handoff-path {handoff} --body "Adoption acknowledged: digest verified, claims reconciled."
   It re-checks the digest against the file. If it fails, say so and do not retry with `post`.

{tail} You hold no authority beyond the role the handoff describes.
"""

    def commit(self, handoff=None):
        self.check_inputs()
        self.check_role_unambiguous()
        if not handoff:
            m = re.search(r"handoff=(\S+)", self.read_rec("awaiting-authority"))
            handoff = m.group(1) if m else None
        if not handoff or not Path(handoff).is_file():
            raise Stop(29, "nothing_to_commit", "no handoff to commit")
        handoff = str(Path(handoff).resolve())
        self.validate(handoff, "commit")
        dig = sha256(handoff)
        want = self.a.expect_digest or (re.search(r"sha256=([0-9a-f]{64})", self.read_rec("validation-record"))
                                        or [None, None])[1]
        if not want:
            raise Stop(32, "validation_not_established_commit", "no validated digest to compare against")
        if dig != want:
            raise Stop(30, "fail_handoff_mutated", "handoff bytes changed since validation")
        self.quiescent()
        if self.count() >= self.a.max_rotations:
            raise Stop(20, "fail_cap_reached", f"cap {self.count()}/{self.a.max_rotations}")

        cursor = self.cursor()
        succ = role_host.host(self.a.successor_host)
        prompt = self.successor_prompt(handoff, dig, cursor)
        (self.state / "successor-prompt.txt").write_text(prompt)
        try:
            row = succ.launch(cwd=self.a.cwd, name=self.a.role, prompt=prompt,
                              host_args=self.a.host_arg, ledger=self.ledger)
        except role_host.HostError as e:
            if e.status == "fork_detected":
                raise Stop(26, "fail_fork_detected", e.detail)
            raise Stop(23, "fail_launch", f"{e.status}: {e.detail}")
        sid = row["session_id"]
        self.count_file.write_text(f"{self.count() + 1}\n")
        topic = f"session:{sid}"
        self.rec("successor-identity", f"host={self.a.successor_host} session={sid} topic={topic} "
                                       f"digest={dig} cursor={cursor}")
        self.log(f"launched {self.a.successor_host} successor {sid}")

        rc, doc, err = self.board("post", "--author", f"supervisor:{self.a.role}",
                                  "--session-id", self.a.pred_session, "--topic", topic,
                                  "--to", topic, "--ref", handoff,
                                  "--body", f"Root handoff for role {self.a.role}. Verify sha256 {dig}, "
                                            f"reconcile claims, then acknowledge with `adopt` citing it. "
                                            f"If the digest does NOT match, post nothing.")
        seq = ((doc or {}).get("message") or {}).get("sequence")
        if rc != 0 or not seq:
            self.abandon(succ, sid, "board_post_failed")
            raise Stop(33, "fail_board_post", f"board post failed: {err[:200]}")
        self.log(f"posted handoff to {topic} at sequence {seq}")

        ack = self.await_adoption(succ, sid, topic, dig, cursor)
        self.log(f"adoption acknowledged by {sid} (message {ack})")
        self.rec("adoption", f"session={sid} message_id={ack}")
        self.rm_awaiting()
        self.post_ending(handoff, dig)
        return self.retire_predecessor(sid)

    def post_ending(self, handoff, dig):
        # The roles view ends an adoption only on an `availability` row with
        # state `ending` from the SAME session and role. Without it every
        # rotation reads as `overlap` forever. Posted after adoption, so a
        # failed rotation never reports the predecessor as ending. The facts
        # are the ones quiescence just established, not the predecessor's word.
        data = self.state / "ending-data.json"
        data.write_text(json.dumps({"state": "ending", "handoff": {"path": handoff, "sha256": dig},
                                    "claims_released": [], "live_children": 0}))
        rc, doc, err = self.board("post", "--kind", "availability", "--role", self.a.role,
                                  "--author", f"supervisor:{self.a.role}", "--session-id", self.a.pred_session,
                                  "--topic", f"session:{self.a.pred_session}", "--data-file", str(data),
                                  "--body", f"Ending: role {self.a.role} handed off; successor adopted "
                                            f"handoff sha256 {dig}.")
        seq = ((doc or {}).get("message") or {}).get("sequence")
        if rc != 0 or not seq:
            # The transfer already happened; a missing report must not undo it,
            # but it must not be silent either.
            self.rec("ending-report", f"not_posted exit={rc} {err[:200]}")
            self.log(f"WARNING: ending report not posted (exit {rc}); roles will read overlap")
            return
        self.rec("ending-report", f"sequence={seq}")
        self.log(f"posted predecessor ending at sequence {seq}")

    def matching_adoption(self, doc, sid, topic, dig):
        for m in (doc or {}).get("messages", []):
            if m.get("kind") != "adoption" or m.get("session_id") != sid:
                continue
            # S2<->S5 integration condition: since S2, a wait on session:X can
            # return a row addressed `to` session:X on ANOTHER topic. Only a row
            # on the successor's own topic is its adoption.
            if m.get("topic") != topic:
                self.log(f"ignored adoption-shaped row {m.get('sequence')} on topic {m.get('topic')!r}")
                continue
            data = m.get("data") or {}
            if data.get("handoff_sha256") == dig and data.get("predecessor_session") == self.a.pred_session:
                return m.get("message_id") or str(m.get("sequence"))
        return None

    def await_adoption(self, succ, sid, topic, dig, since):
        deadline = time.time() + self.a.adopt_timeout
        seen_running = False
        launched_at = time.time()
        while time.time() < deadline:
            chunk = max(1, min(5, int(deadline - time.time())))
            rc, doc, err = self.board("wait", "--since", str(since), "--topic", topic,
                                      "--timeout-seconds", str(chunk), timeout=chunk + 60)
            if rc == 0:
                ack = self.matching_adoption(doc, sid, topic, dig)
                if ack:
                    return ack
                nxt = (doc or {}).get("next_since")
                if isinstance(nxt, int) and nxt > since:
                    since = nxt
            elif rc != 2:  # 2 is the wait's own timeout; anything else is an error
                self.abandon(succ, sid, "adoption_not_established")
                raise Stop(32, "adoption_not_established", f"board wait exit {rc}: {err[:200]}")
            # A successor that ended its turn without adopting will not adopt.
            # Checked only after it was seen running, or after a grace period, so
            # a slow start is not mistaken for an ended turn.
            try:
                st = succ.status(sid, self.ledger)["state"]
            except role_host.HostError:
                continue
            seen_running = seen_running or st == "running"
            if st != "running" and (seen_running or time.time() - launched_at > 30):
                rc, doc, _ = self.board("wait", "--since", str(since), "--topic", topic,
                                        "--timeout-seconds", "0")
                ack = self.matching_adoption(doc, sid, topic, dig) if rc == 0 else None
                if ack:
                    return ack
                self.abandon(succ, sid, "turn_ended_without_adoption")
                raise Stop(34, "fail_no_adoption", f"successor {sid} is {st} and has not adopted")
        self.abandon(succ, sid, "no_adoption")
        raise Stop(34, "fail_no_adoption", f"no adoption within {self.a.adopt_timeout}s")

    def abandon(self, succ, sid, reason):
        # An unadopted successor owns nothing, but left loaded it is a second
        # candidate root. Name it in STATE either way, and retire it unless kept.
        disp = self.a.orphan
        if disp == "stop":
            try:
                succ.retire(sid, ledger=self.ledger, force=True)
            except role_host.HostError as e:
                disp = f"retire_failed:{e.status}"
        self.rec("orphan-successor", f"host={self.a.successor_host} session={sid} reason={reason} "
                                     f"disposition={disp}")
        self.log(f"unadopted successor {sid}: {reason}, disposition {disp}")

    def retire_predecessor(self, sid):
        n = self.count()
        if self.a.retire == "keep":
            self.rec("predecessor-disposition", "kept")
            self.log("predecessor kept for questions about its own record; it holds no role")
            return Stop(0, f"committed_rotation_{n}", f"committed; successor {sid}")
        pred = role_host.host(self.a.pred_host)
        # The predecessor usually launched this gate and then ended its turn, so
        # give it time to go idle rather than interrupting it.
        deadline = time.time() + self.a.pred_idle_timeout
        try:
            while pred.status(self.a.pred_session, self.ledger)["state"] == "running":
                if time.time() > deadline:
                    raise role_host.HostError(40, "refused_busy", "predecessor still running")
                time.sleep(2)
            res = pred.retire(self.a.pred_session, ledger=self.ledger)
            self.rec("predecessor-disposition", f"retired {res['retired']}")
            return Stop(0, f"committed_rotation_{n}", f"committed; predecessor {res['retired']}")
        except role_host.HostError as e:
            self.rec("predecessor-disposition", f"retire_failed {e.status}: {e.detail}")
            return Stop(38, f"committed_rotation_{n}_predecessor_not_retired", e.detail)

    def rm_awaiting(self):
        p = self.state / "awaiting-authority"
        if p.exists():
            p.unlink()

    def watch(self, handoff):
        auth = {x.strip() for x in (self.a.gate_authority or "").split(",") if x.strip()}
        if not auth:
            raise Stop(77, "watch_refused_no_authority", "no authorized approver named")
        try:
            want = sha256(handoff)
        except OSError:
            raise Stop(79, "watch_refused_no_digest", f"cannot digest {handoff}")
        topic = f"control:rotation:{self.a.role}"
        since = self.cursor()
        deadline = time.time() + self.a.watch_timeout
        self.rec("watch-state", f"topic={topic} since={since} authority={','.join(sorted(auth))} digest={want}")
        self.log(f"watching {topic} from {since}; approvers {sorted(auth)}; must cite sha256 {want[:12]}")
        while time.time() < deadline:
            left = int(deadline - time.time())
            if left <= 0:
                break
            rc, doc, err = self.board("wait", "--since", str(since), "--topic", topic,
                                      "--timeout-seconds", str(left), timeout=left + 60)
            if rc == 2:
                break
            if rc != 0:
                raise Stop(32, "watch_board_error", f"wait exit {rc}: {err[:200]}")
            approved = None
            # Examine EVERY message before the cursor moves, so a rejectable post
            # cannot hide a valid approval behind it in the same batch.
            for m in (doc or {}).get("messages", []):
                body = m.get("body") or ""
                if self.a.approve_token not in body:
                    continue
                ident = f"message_id={m.get('message_id')} author={m.get('author')}"
                if want not in body:
                    self.rec("last-rejected-trigger", f"unbound {ident}")
                elif m.get("author") in auth or m.get("session_id") in auth:
                    approved = approved or ident
                else:
                    self.rec("last-rejected-trigger", f"unauthorized {ident}")
            if approved:
                self.rec("authorized-by", f"{approved} digest={want} rotation={self.count()}")
                self.log(f"approval accepted: {approved}")
                self.a.expect_digest = want
                return self.commit(handoff)
            nxt = (doc or {}).get("next_since")
            if not isinstance(nxt, int) or nxt <= since:
                break
            since = nxt
        raise Stop(78, "watch_expired", "watch window closed with no authorized approval")


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def now():
    return datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def parser():
    repo_default = os.environ.get("REPO") or str(HERE.parents[2])
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("op", choices=["prepare", "commit", "watch"])
    ap.add_argument("handoff", nargs="?")
    ap.add_argument("--role", required=True)
    ap.add_argument("--state", required=True, help="durable evidence directory for this role")
    ap.add_argument("--repo", default=repo_default, help="repository holding the board")
    ap.add_argument("--board-cmd", default=str(HERE.parents[1] / "durable-state/scripts/codex_chatboard.py"))
    ap.add_argument("--host-state", default=str(role_host.DEFAULT_STATE))
    ap.add_argument("--pred-host", choices=sorted(role_host.HOSTS), required=True)
    ap.add_argument("--pred-session", required=True)
    ap.add_argument("--successor-host", choices=sorted(role_host.HOSTS), required=True)
    ap.add_argument("--cwd", help="successor working directory (default: --repo)")
    ap.add_argument("--host-arg", action="append", default=[], help="extra successor launch argument")
    ap.add_argument("--validator", default=f"python3 {HERE / 'handoff_validator.py'}")
    ap.add_argument("--quiescence-file")
    ap.add_argument("--resource-prefix", default="",
                    help="count other sessions' claims under this prefix as this root's children")
    ap.add_argument("--max-rotations", type=int, default=3)
    ap.add_argument("--gate", choices=["require", "open"], default="require")
    ap.add_argument("--adopt-timeout", type=int, default=300)
    ap.add_argument("--retire", choices=["keep", "stop"], default="keep")
    ap.add_argument("--pred-idle-timeout", type=int, default=600)
    ap.add_argument("--orphan", choices=["stop", "keep"], default="stop")
    ap.add_argument("--after-adopt", choices=["continue", "stop"], default="continue")
    ap.add_argument("--gate-authority", default="")
    ap.add_argument("--approve-token", default="approve-rotation")
    ap.add_argument("--watch-timeout", type=int, default=3000)
    ap.add_argument("--expect-digest", default="", help=argparse.SUPPRESS)
    ap.add_argument("--detach", action="store_true",
                    help="run commit/watch in a new session and return at once, so the predecessor can "
                         "end its turn and be retired without killing the gate")
    return ap


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    a = parser().parse_args(argv)
    a.cwd = a.cwd or a.repo
    if a.handoff:
        a.handoff = str(Path(a.handoff).resolve())
    if a.op in ("prepare", "watch") and not a.handoff:
        print("prepare and watch need a handoff", file=sys.stderr)
        return 64
    gate = Gate(a)
    if a.detach and a.op in ("commit", "watch"):
        child = [sys.executable, str(Path(__file__).resolve())] + [x for x in argv if x != "--detach"]
        with open(gate.state / "detached.log", "a") as fh:
            p = subprocess.Popen(child, stdout=fh, stderr=fh, stdin=subprocess.DEVNULL, start_new_session=True)
        gate.rec("detached-pid", str(p.pid))
        print(json.dumps({"status": f"{a.op}_detached", "pid": p.pid, "state": str(gate.state)}))
        return 0
    os.environ.setdefault("BASH_MAX_TIMEOUT_MS", "3900000")
    try:
        if a.op == "prepare":
            result = gate.prepare(a.handoff)
        elif a.op == "commit":
            result = gate.commit(a.handoff)
        else:
            result = gate.watch(a.handoff)
    except Stop as s:
        result = s
    gate.rec("terminal-status", result.status)
    gate.log(f"{result.status}: {result.msg}")
    return result.code


if __name__ == "__main__":
    sys.exit(main())
