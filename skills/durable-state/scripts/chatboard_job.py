#!/usr/bin/env python3
"""Run a long command detached and announce its completion on the chatboard.

    python3 skills/durable-state/scripts/chatboard_job.py --repository . run \
        --author AUTHOR --session-id SESSION_ID [--name NAME] [--topic TOPIC] \
        [--tail-lines N] -- COMMAND [ARGS...]

`run` returns at once and prints JSON with the job's log paths, its `topic`,
the board cursor `since`, and a base `wait` command. Use your role's usual
blocking wait (no timeout), with its context-pressure and budget binding, and
add `--topic <topic>` and the printed `--since`: the wait then returns on the
job's completion as well as on context or budget events and other selected
topics. The base command alone carries `--session-id` (session control events)
but no context or budget binding. The completion post uses `--wake`, so it
also reaches a waiter that suppresses its own session's posts.

The default topic is `job:<session-id>`. The completion message records the
exit code, duration and log paths. Output is not posted unless --tail-lines
asks for it: board messages are shared and durable, so only include output
that is safe to share.
"""
import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

BOARD = Path(__file__).with_name("codex_chatboard.py")
STATE = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "chatboard-jobs"


def board(repository, *args):
    return subprocess.run(
        [sys.executable, str(BOARD), "--repository", repository, *args],
        capture_output=True, text=True, check=True,
    ).stdout


def latest_sequence(repository):
    messages = json.loads(board(repository, "read", "--limit", "1"))["messages"]
    return messages[-1]["sequence"] if messages else 0


def run(a):
    if not a.command:
        sys.exit("a command is required after --")
    name = a.name or Path(a.command[0]).name
    job = f"{time.strftime('%Y%m%dT%H%M%S')}-{name}-{os.getpid()}"
    directory = STATE / a.session_id / job
    directory.mkdir(parents=True)
    topic = a.topic or f"job:{a.session_id}"
    since = latest_sequence(a.repository)
    record = {
        "job": job, "name": name, "command": a.command, "cwd": os.getcwd(),
        "topic": topic, "since": since, "stdout": str(directory / "stdout.log"),
        "stderr": str(directory / "stderr.log"), "started": time.time(),
    }
    (directory / "job.json").write_text(json.dumps(record, indent=1))
    supervisor = subprocess.Popen(
        [sys.executable, __file__, "--repository", a.repository, "_supervise",
         "--author", a.author, "--session-id", a.session_id,
         "--tail-lines", str(a.tail_lines), str(directory)],
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    record["supervisor_pid"] = supervisor.pid
    record["wait"] = (f"python3 {BOARD.relative_to(Path(a.repository).resolve()) if BOARD.is_relative_to(Path(a.repository).resolve()) else BOARD}"
                      f" --repository {a.repository} wait --since {since} --topic {topic}"
                      f" --session-id {a.session_id}")
    record["note"] = "add your role's context/budget binding flags to this wait"
    print(json.dumps(record, indent=1))


def supervise(a):
    directory = Path(a.directory)
    record = json.loads((directory / "job.json").read_text())
    with open(record["stdout"], "wb") as out, open(record["stderr"], "wb") as err:
        try:
            child = subprocess.Popen(record["command"], cwd=record["cwd"], stdin=subprocess.DEVNULL,
                                     stdout=out, stderr=err, start_new_session=True)
            record["pid"] = child.pid
            (directory / "job.json").write_text(json.dumps(record, indent=1))
            code = child.wait()
        except OSError as e:
            err.write(f"chatboard_job: could not start: {e}\n".encode())
            code = 127
    record["exit"] = code
    record["seconds"] = round(time.time() - record["started"], 1)
    (directory / "job.json").write_text(json.dumps(record, indent=1))
    body = (f"Background job {record['name']} ({record['job']}) finished: exit {code} "
            f"after {record['seconds']} s. stdout {record['stdout']}; stderr {record['stderr']}.")
    if a.tail_lines > 0:
        tail = Path(record["stdout"]).read_text(errors="replace").splitlines()[-a.tail_lines:]
        body += "\nLast stdout lines:\n" + "\n".join(tail)[-2000:]
    try:
        board(a.repository, "post", "--author", a.author, "--session-id", a.session_id,
              "--topic", record["topic"], "--wake", "--ref", str(directory / "job.json"), "--body", body)
        record["posted"] = True
    except subprocess.CalledProcessError as e:
        record["posted"] = False
        record["post_error"] = (e.stderr or "")[-500:]
    (directory / "job.json").write_text(json.dumps(record, indent=1))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--repository", required=True)
    sub = p.add_subparsers(dest="action", required=True)
    r = sub.add_parser("run", help="start COMMAND detached; print the wait command")
    r.add_argument("--author", required=True)
    r.add_argument("--session-id", required=True)
    r.add_argument("--name")
    r.add_argument("--topic", help="default: job:<session-id>")
    r.add_argument("--tail-lines", type=int, default=0, help="post the last N stdout lines (default 0)")
    r.add_argument("command", nargs=argparse.REMAINDER)
    s = sub.add_parser("_supervise")
    s.add_argument("--author", required=True)
    s.add_argument("--session-id", required=True)
    s.add_argument("--tail-lines", type=int, default=0)
    s.add_argument("directory")
    a = p.parse_args()
    if a.action == "run":
        if a.command and a.command[0] == "--":
            a.command = a.command[1:]
        run(a)
    else:
        supervise(a)


if __name__ == "__main__":
    main()
