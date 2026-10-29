#!/usr/bin/env python3
"""Record build and test processes, and freeze subjects, without hand-written custody.

`run` starts each command in its own process group, waits for it and writes
NAME.json beside NAME.stdout and NAME.stderr: argv, working directory,
environment, wrapper and child PIDs, start and end times, return code or
signal, output and input digests, and any member of the process group still
alive after the wait. It prints one line per step. A `--plan` runs several
steps (for example a build and its tests) in one process and writes
summary.json. A record file is never overwritten.

`freeze` reads a directory tree once, writes manifest.json (the evidence-battery
entry shape), subject.tar built from the same bytes with fixed metadata, and,
against a baseline, changes.json and subject.diff. It prints one JSON line.

Exit codes: `run` returns 0 when every step exits 0, else 1; `freeze` returns
0; both return 2 for a usage error.
"""

from __future__ import annotations

import argparse
import datetime as dt
import difflib
import fnmatch
import hashlib
import io
import json
import os
import re
import signal
import stat
import subprocess
import sys
import tarfile
import time
from pathlib import Path
from typing import Any

RUN_SCHEMA = "manual-orchestration.custody.run.v1"
SUMMARY_SCHEMA = "manual-orchestration.custody.summary.v1"
FREEZE_SCHEMA = "manual-orchestration.custody.freeze.v1"
NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
EXE_RE = re.compile(r"\{exe:([A-Za-z0-9._-]+)(?::([A-Za-z0-9._-]+))?\}")
STEP_KEYS = {"name", "argv", "cwd", "env", "env_clear", "digest", "cargo_json", "timeout", "kill_leftovers"}
PLAN_KEYS = {"cwd", "env", "env_clear", "steps"}


class UsageError(Exception):
    pass


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z")


def write_json(path: Path, value: Any) -> str:
    data = (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode()
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_bytes(data)
    os.replace(tmp, path)
    return sha256(data)


def file_entry(path: Path, name: str) -> dict[str, Any]:
    """Same shape as evidence-battery entries: path, type, mode, length, sha256."""
    try:
        info = path.lstat()
    except FileNotFoundError:
        return {"path": name, "type": "absent"}
    if stat.S_ISLNK(info.st_mode):
        target = os.readlink(path)
        payload, kind = os.fsencode(target), "symlink"
    elif stat.S_ISREG(info.st_mode):
        payload, kind = path.read_bytes(), "file"
    else:
        raise UsageError(f"unsupported path type: {name}")
    out = {"path": name, "type": kind, "mode": stat.S_IMODE(info.st_mode),
           "length": len(payload), "sha256": sha256(payload)}
    if kind == "symlink":
        out["link_target"] = target
    return out


# ---------------------------------------------------------------- run

def group_members(pgid: int) -> list[dict[str, Any]]:
    """Processes whose process group is pgid, read from /proc."""
    members = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            raw = (entry / "stat").read_text()
        except OSError:
            continue
        comm = raw[raw.find("(") + 1:raw.rfind(")")]
        fields = raw[raw.rfind(")") + 2:].split()
        if len(fields) > 2 and int(fields[2]) == pgid:
            members.append({"pid": int(entry.name), "comm": comm, "state": fields[0]})
    return sorted(members, key=lambda m: m["pid"])


def kill_group(pgid: int) -> None:
    for sig, pause in ((signal.SIGTERM, 2.0), (signal.SIGKILL, 0.5)):
        try:
            os.killpg(pgid, sig)
        except ProcessLookupError:
            return
        deadline = time.monotonic() + pause
        while time.monotonic() < deadline:
            if not [m for m in group_members(pgid) if m["state"] != "Z"]:
                return
            time.sleep(0.05)


def cargo_executables(stdout: Path) -> list[dict[str, Any]]:
    found = []
    for line in stdout.read_text(errors="replace").splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if (isinstance(msg, dict) and msg.get("reason") == "compiler-artifact"
                and msg.get("executable") and (msg.get("profile") or {}).get("test")):
            exe = Path(msg["executable"])
            found.append({"name": (msg.get("target") or {}).get("name"),
                          "kind": (msg.get("target") or {}).get("kind"),
                          "path": str(exe), "sha256": sha256(exe.read_bytes())})
    return found


def resolve_exe(text: str, done: dict[str, dict[str, Any]]) -> str:
    def sub(match: re.Match) -> str:
        step, target = match.group(1), match.group(2)
        if step not in done:
            raise UsageError(f"{match.group(0)}: no earlier step named {step}")
        exes = done[step].get("cargo_executables") or []
        if target:
            exes = [e for e in exes if e["name"] == target]
        if len(exes) != 1:
            names = [e["name"] for e in done[step].get("cargo_executables") or []]
            raise UsageError(f"{match.group(0)}: expected one test executable, found {names}")
        return exes[0]["path"]
    return EXE_RE.sub(sub, text)


def run_step(out: Path, step: dict[str, Any]) -> dict[str, Any]:
    name = step["name"]
    record_path = out / f"{name}.json"
    running_path = out / f"{name}.running.json"
    for path in (record_path, running_path, out / f"{name}.stdout", out / f"{name}.stderr"):
        if path.exists():
            raise UsageError(f"{path} exists; use a new step name")
    cwd = Path(step.get("cwd") or os.getcwd()).resolve()
    env_set = {str(k): str(v) for k, v in (step.get("env") or {}).items()}
    env = dict(env_set) if step.get("env_clear") else {**os.environ, **env_set}
    inputs = [file_entry(Path(p) if Path(p).is_absolute() else cwd / p, p) for p in step.get("digest") or []]
    timeout = step.get("timeout")
    base = {"schema": RUN_SCHEMA, "name": name, "argv": step["argv"], "cwd": str(cwd),
            "env_clear": bool(step.get("env_clear")), "env_set": env_set,
            "env_inherited": not step.get("env_clear"), "wrapper_pid": os.getpid(),
            "inputs": inputs, "timeout_s": timeout}
    stdout_path, stderr_path = out / f"{name}.stdout", out / f"{name}.stderr"
    started_at, t0 = now(), time.monotonic()
    timed_out = False
    with stdout_path.open("wb") as so, stderr_path.open("wb") as se:
        try:
            proc = subprocess.Popen(step["argv"], cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                    stdout=so, stderr=se, start_new_session=True)
        except OSError as exc:
            record = {**base, "started_at": started_at, "ended_at": now(), "pid": None,
                      "launch_error": f"{type(exc).__name__}: {exc}", "returncode": None,
                      "signal": None, "timed_out": False, "leftover_processes": []}
            write_json(record_path, record)
            return record
        write_json(running_path, {**base, "pid": proc.pid, "started_at": started_at})
        try:
            rc = proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            kill_group(proc.pid)
            rc = proc.wait()
    ended_at, duration = now(), round(time.monotonic() - t0, 3)
    leftovers = [m for m in group_members(proc.pid) if m["state"] != "Z"]
    killed = False
    if leftovers and step.get("kill_leftovers"):
        kill_group(proc.pid)
        killed = True
    record = {**base, "pid": proc.pid, "pgid": proc.pid, "started_at": started_at,
              "ended_at": ended_at, "duration_s": duration,
              "returncode": rc if rc >= 0 else None, "signal": -rc if rc < 0 else None,
              "timed_out": timed_out,
              "stdout": {"path": str(stdout_path), **{k: v for k, v in file_entry(stdout_path, "").items() if k in ("length", "sha256")}},
              "stderr": {"path": str(stderr_path), **{k: v for k, v in file_entry(stderr_path, "").items() if k in ("length", "sha256")}},
              "leftover_processes": leftovers, "leftovers_killed": killed,
              "leftovers_after_kill": [m for m in group_members(proc.pid) if m["state"] != "Z"] if killed else None}
    if step.get("cargo_json"):
        record["cargo_executables"] = cargo_executables(stdout_path)
    write_json(record_path, record)
    running_path.unlink()
    return record


def outcome(record: dict[str, Any]) -> str:
    if record.get("launch_error"):
        return "launch-error"
    if record["timed_out"]:
        return "timeout"
    if record["signal"] is not None:
        return f"signal:{record['signal']}"
    return str(record["returncode"])


def ok(record: dict[str, Any]) -> bool:
    return outcome(record) == "0"


def step_line(record: dict[str, Any], out: Path) -> str:
    line = (f"{record['name']} exit={outcome(record)} {record.get('duration_s', 0):.1f}s "
            f"pid={record['pid']} leftovers={len(record['leftover_processes'])} "
            f"record={out / (record['name'] + '.json')}")
    if record.get("launch_error"):
        line += f" error={record['launch_error']}"
    return line


def check_step(step: Any, index: int) -> None:
    if not isinstance(step, dict):
        raise UsageError(f"step {index}: must be an object")
    unknown = set(step) - STEP_KEYS
    if unknown:
        raise UsageError(f"step {index}: unknown keys {sorted(unknown)}")
    if not isinstance(step.get("name"), str) or not NAME_RE.match(step["name"]):
        raise UsageError(f"step {index}: name must match {NAME_RE.pattern}")
    argv = step.get("argv")
    if not isinstance(argv, list) or not argv or not all(isinstance(a, str) for a in argv):
        raise UsageError(f"step {step['name']}: argv must be a nonempty list of strings")


def cmd_run(args: argparse.Namespace) -> int:
    out = Path(args.out).resolve()
    if args.plan:
        if args.argv or args.name:
            raise UsageError("--plan takes no --name or command")
        plan_bytes = Path(args.plan).read_bytes()
        plan = json.loads(plan_bytes)
        if not isinstance(plan, dict) or set(plan) - PLAN_KEYS or not isinstance(plan.get("steps"), list) or not plan["steps"]:
            raise UsageError(f"plan must be an object with keys {sorted(PLAN_KEYS)} and a nonempty steps list")
        steps = []
        for i, raw in enumerate(plan["steps"]):
            check_step(raw, i)
            steps.append({"cwd": plan.get("cwd"), "env_clear": plan.get("env_clear", False),
                          **raw, "env": {**(plan.get("env") or {}), **(raw.get("env") or {})}})
        names = [s["name"] for s in steps]
        if len(set(names)) != len(names):
            raise UsageError("step names must be unique")
    else:
        argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
        if not args.name or not argv:
            raise UsageError("give --name and a command after --, or --plan")
        env = {}
        for item in args.env or []:
            key, sep, value = item.partition("=")
            if not sep or not key:
                raise UsageError(f"--env takes KEY=VALUE, got {item!r}")
            env[key] = value
        steps = [{"name": args.name, "argv": argv, "cwd": args.cwd, "env": env,
                  "env_clear": args.env_clear, "digest": args.digest, "cargo_json": args.cargo_json,
                  "timeout": args.timeout, "kill_leftovers": args.kill_leftovers}]
        check_step(steps[0], 0)
        plan_bytes = None
    out.mkdir(parents=True, exist_ok=True)
    done: dict[str, dict[str, Any]] = {}
    rows, status = [], "passed"
    for step in steps:
        step = {**step, "argv": [resolve_exe(a, done) for a in step["argv"]],
                "digest": [resolve_exe(p, done) for p in step.get("digest") or []]}
        record = run_step(out, step)
        done[step["name"]] = record
        rows.append({"name": step["name"], "outcome": outcome(record),
                     "record": str(out / f"{step['name']}.json"),
                     "record_sha256": sha256((out / f"{step['name']}.json").read_bytes()),
                     "leftovers": len(record["leftover_processes"])})
        print(step_line(record, out), flush=True)
        if not ok(record):
            status = "failed"
            if not args.keep_going:
                break
    if plan_bytes is not None:
        skipped = [s["name"] for s in steps if s["name"] not in done]
        summary = {"schema": SUMMARY_SCHEMA, "plan": str(Path(args.plan).resolve()),
                   "plan_sha256": sha256(plan_bytes), "status": status, "steps": rows,
                   "not_run": skipped}
        summary_path = out / "summary.json"
        if summary_path.exists():
            raise UsageError(f"{summary_path} exists")
        digest = write_json(summary_path, summary)
        passed = sum(1 for r in rows if r["outcome"] == "0")
        print(f"summary {status} {passed}/{len(steps)} not_run={len(skipped)} "
              f"record={summary_path} sha256={digest}", flush=True)
    return 0 if status == "passed" else 1


# ---------------------------------------------------------------- freeze

def excluded(rel: str, patterns: list[str]) -> bool:
    base = rel.rsplit("/", 1)[-1]
    return any(fnmatch.fnmatchcase(rel, p) or fnmatch.fnmatchcase(base, p) for p in patterns)


def walk(root: Path, patterns: list[str]) -> list[tuple[str, Path]]:
    found = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        here = Path(dirpath)
        rel_dir = here.relative_to(root).as_posix()
        keep = []
        for d in sorted(dirnames):
            rel = d if rel_dir == "." else f"{rel_dir}/{d}"
            if excluded(rel, patterns):
                continue
            if (here / d).is_symlink():
                found.append((rel, here / d))
            else:
                keep.append(d)
        dirnames[:] = keep
        for f in sorted(filenames):
            rel = f if rel_dir == "." else f"{rel_dir}/{f}"
            if not excluded(rel, patterns):
                found.append((rel, here / f))
    return sorted(found)


def read_tree(root: Path, patterns: list[str]) -> tuple[list[dict[str, Any]], dict[str, bytes]]:
    entries, payloads = [], {}
    for rel, path in walk(root, patterns):
        info = path.lstat()
        if stat.S_ISLNK(info.st_mode):
            target = os.readlink(path)
            entries.append({"path": rel, "type": "symlink", "mode": stat.S_IMODE(info.st_mode),
                            "length": len(os.fsencode(target)), "sha256": sha256(os.fsencode(target)),
                            "link_target": target})
        elif stat.S_ISREG(info.st_mode):
            data = path.read_bytes()
            payloads[rel] = data
            entries.append({"path": rel, "type": "file", "mode": stat.S_IMODE(info.st_mode),
                            "length": len(data), "sha256": sha256(data)})
        else:
            raise UsageError(f"unsupported path type in subject: {rel}")
    return entries, payloads


def cmd_freeze(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    if not root.is_dir():
        raise UsageError(f"--root is not a directory: {root}")
    out = Path(args.out).resolve()
    if out == root or out.is_relative_to(root):
        raise UsageError("--out must be outside --root")
    if out.exists() and any(out.iterdir()):
        raise UsageError(f"{out} is not empty; a frozen subject is never overwritten")
    if args.baseline_root and args.baseline_manifest:
        raise UsageError("give --baseline-root or --baseline-manifest, not both")
    patterns = args.exclude or []
    entries, payloads = read_tree(root, patterns)
    out.mkdir(parents=True, exist_ok=True)

    archive = out / "subject.tar"
    with tarfile.open(archive, "w", format=tarfile.GNU_FORMAT) as tar:
        for e in entries:
            info = tarfile.TarInfo(e["path"])
            info.mode, info.mtime, info.uid, info.gid, info.uname, info.gname = e["mode"], 0, 0, 0, "", ""
            if e["type"] == "symlink":
                info.type, info.linkname = tarfile.SYMTYPE, e["link_target"]
                tar.addfile(info)
            else:
                info.size = e["length"]
                tar.addfile(info, io.BytesIO(payloads[e["path"]]))

    manifest = {"schema": FREEZE_SCHEMA, "label": args.label, "root": str(root),
                "excludes": patterns, "frozen_at": now(), "entry_count": len(entries),
                "entries_sha256": sha256(canonical(entries)), "entries": entries}
    manifest_sha = write_json(out / "manifest.json", manifest)
    result: dict[str, Any] = {
        "manifest": {"path": str(out / "manifest.json"), "sha256": manifest_sha},
        "archive": {"path": str(archive), "sha256": sha256(archive.read_bytes())},
        "entries_sha256": manifest["entries_sha256"], "entry_count": len(entries)}

    baseline = None
    base_payloads: dict[str, bytes] = {}
    if args.baseline_root:
        base_root = Path(args.baseline_root).resolve()
        if not base_root.is_dir():
            raise UsageError(f"--baseline-root is not a directory: {base_root}")
        baseline, base_payloads = read_tree(base_root, patterns)
        base_ref = {"root": str(base_root)}
    elif args.baseline_manifest:
        doc = json.loads(Path(args.baseline_manifest).read_text())
        baseline = doc.get("entries") if isinstance(doc, dict) else None
        if not isinstance(baseline, list) or not all(isinstance(e, dict) and "path" in e and "sha256" in e for e in baseline):
            raise UsageError("--baseline-manifest needs an object with entries [{path, sha256, ...}]")
        base_ref = {"manifest": str(Path(args.baseline_manifest).resolve()),
                    "sha256": sha256(Path(args.baseline_manifest).read_bytes())}
    if baseline is not None:
        old = {e["path"]: e for e in baseline}
        new = {e["path"]: e for e in entries}
        changes = {"baseline": base_ref,
                   "changed": sorted(p for p in new if p in old and (old[p]["sha256"], old[p].get("type")) != (new[p]["sha256"], new[p]["type"])),
                   "added": sorted(set(new) - set(old)), "removed": sorted(set(old) - set(new))}
        changes["unchanged_count"] = len(new) - len(changes["changed"]) - len(changes["added"])
        result["changes"] = {"path": str(out / "changes.json"), "sha256": write_json(out / "changes.json", changes),
                             **{k: len(changes[k]) for k in ("changed", "added", "removed")}}
        if args.baseline_root:
            diff = []
            for p in changes["changed"] + changes["added"] + changes["removed"]:
                a, b = base_payloads.get(p), payloads.get(p)
                try:
                    a_text = a.decode().splitlines(True) if a is not None else []
                    b_text = b.decode().splitlines(True) if b is not None else []
                except UnicodeDecodeError:
                    diff.append(f"Binary files baseline/{p} and subject/{p} differ\n")
                    continue
                diff.extend(difflib.unified_diff(a_text, b_text, f"baseline/{p}", f"subject/{p}"))
            (out / "subject.diff").write_text("".join(diff))
            result["diff"] = {"path": str(out / "subject.diff"), "sha256": sha256((out / "subject.diff").read_bytes())}
    for path in out.iterdir():
        path.chmod(0o444)
    print(json.dumps(result, separators=(",", ":")), flush=True)
    return 0


# ---------------------------------------------------------------- cli

def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="Record build/test processes and freeze subjects.")
    sub = p.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run", help="run one command, or a plan of steps, and record custody")
    r.add_argument("--out", required=True, help="record directory (created if missing)")
    r.add_argument("--name", help="step name for a single command")
    r.add_argument("--plan", help="JSON plan: {cwd?, env?, env_clear?, steps: [{name, argv, ...}]}")
    r.add_argument("--cwd")
    r.add_argument("--env", action="append", metavar="KEY=VALUE")
    r.add_argument("--env-clear", action="store_true", help="start from an empty environment")
    r.add_argument("--digest", action="append", metavar="PATH", help="input file to digest before running")
    r.add_argument("--cargo-json", action="store_true", help="record test executables from cargo --message-format=json")
    r.add_argument("--timeout", type=float, metavar="SECONDS")
    r.add_argument("--kill-leftovers", action="store_true", help="stop process-group members still alive after the wait")
    r.add_argument("--keep-going", action="store_true", help="with --plan, run later steps after a failure")
    r.add_argument("argv", nargs=argparse.REMAINDER, help="-- COMMAND ARGS...")
    f = sub.add_parser("freeze", help="manifest, archive and changes for a directory tree")
    f.add_argument("--root", required=True)
    f.add_argument("--out", required=True, help="new or empty directory outside --root")
    f.add_argument("--exclude", action="append", metavar="GLOB", help="matched against relative path and basename")
    f.add_argument("--baseline-root")
    f.add_argument("--baseline-manifest")
    f.add_argument("--label")
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        return cmd_run(args) if args.command == "run" else cmd_freeze(args)
    except UsageError as exc:
        print(f"custody: {exc}", file=sys.stderr)
        return 2
    except (OSError, ValueError) as exc:
        print(f"custody: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
