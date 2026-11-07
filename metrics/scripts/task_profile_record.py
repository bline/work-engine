#!/usr/bin/env python3
"""Shadow-mode task-profile records for Work Engine dispatches
(metrics/dogfood-selection-plan-20261027.md). Called by the coordinator guidance
(skills/manual-orchestration/references/coordinator.md) during the shadow pilot.

Uses the pinned agent-routing-lab release (`taskprofile`) and writes, never overwriting:

  <run_dir>/task-profiles/<task_revision>/
    sidecar-stageB.json         the coordinator's declaration, byte for byte
    profile-stageB.json         taskprofile compute output (profile-failed.json if compute failed)
    validation.json             taskprofile validate --production result
    overhead.json               declaration timing, tool pin, unknown-field count, hashes of the files above
    assignment.json             first assignment; retained continuations add assignment-02.json, ...
    attribution-<assignment>.json  models actually used, resolved from the usage ledger
    SHA256SUMS                  append-only index; a listed file that changes is an integrity failure

Every overhead, assignment and attribution record stores `recorder`, this script's path and
sha256: coordinators run it from their checkout, not from a release.

Every record after the first also stores `prior_files`: the sha256 of each file that existed
when it was written. The run directory is then copied into the data repository, whose git
history is the tamper-evident copy; these local checks catch accidental edits, not an
adversary who rewrites every file.

Install a pinned tool once per commit (outside this script):
  python3 -m venv ~/.local/state/work-engine/tools/agent-routing-lab/<commit>
  ~/.local/state/work-engine/tools/agent-routing-lab/<commit>/bin/pip install \\
      "git+https://github.com/bline/agent-routing-lab@<commit>"

Usage
  task_profile_record.py start      prints the current time; run it before writing the sidecar
  task_profile_record.py check-tool [--tool-dir DIR]
  task_profile_record.py profile   --run-dir R --sidecar S --repo REPO --declare-started-at ISO8601
                                   [--call-id ID ...] [--tool-dir DIR]
  task_profile_record.py assign    --run-dir R (--task-revision ID | --episode TASK_ID) --role builder|reviewer
                                   --requested-model M --requested-effort E --harness H --run NAME
                                   --dispatched-packet PATH --child-session ID [--child-session ID ...]
                                   [--reported-model M] [--reported-effort E] [--selected-at ISO8601] [--retained]
  task_profile_record.py attribute --run-dir R (--task-revision ID | --all) --ledger DIR [--assignment NAME]
                                   [--since ISO8601] [--until ISO8601]
  task_profile_record.py init-round --run-dir R --cap N --terra-in-flight N --terra-repair-budget N
  task_profile_record.py enroll    --run-dir R --task-id T (--task-revision REV | --packet PATH) --eligible yes|no --reason TEXT
  task_profile_record.py withdraw  --run-dir R --task-id T --reason TEXT
  task_profile_record.py review    --run-dir R --task-id T --verdict accept|changes_required|blocked_evidence|blocked_scope --reviewer-session ID --p1 N --p2 N --p3 N --receipt PATH
  task_profile_record.py repair    --run-dir R --task-id T --review-receipt PATH
  task_profile_record.py close     --run-dir R --task-id T --disposition accepted|abandoned --reason TEXT
                                   --pre-review-fixes N --escalations N [--review-receipt PATH ...] [--artifact PATH ...]
  task_profile_record.py pause     --run-dir R --reason TEXT
  task_profile_record.py resume    --run-dir R --owner-approval TEXT
  task_profile_record.py status    --run-dir R [--task-id T]
  task_profile_record.py recover-screening --run-dir R
  task_profile_record.py admission --run-dir R   read-only gate before screening; exit 0 only when active
  task_profile_record.py verify    --run-dir R [--repo REPO] [--no-recompute] [--tools-root DIR]

Round-2 episode records live in <run_dir>/enrollments/<task_id>/ with their own
SHA256SUMS and prior_files. Initialize round.json before using round commands.
Round commands print one JSON object (verify retains text lines); exit 2 is a usage refusal, 1 an integrity
failure, 3 includes profile no-ops and verify with no records, and 0 includes logged screening refusals.
`admission` prints one JSON object and writes nothing: exit 0 active, 10 paused, 11 stop_reached, 12 unknown;
only 0 admits a screening.
Exit 4 is profile_required: an eligible packet can enroll; no screening or enrollment is written. A run-level advisory lock
serializes round commands; episode records are published without replacement by linking a complete, fsynced
temporary file to the final name. Missing manifest entries are recovered on read
and reported in command output. Every episode record carries content_sha256 of
its canonical JSON without that field; every read checks it before manifest
re-listing. Torn manifest tails are archived without changing earlier complete
lines. Resume history acknowledges durable triggers.

--child-session takes the child's Claude session id or Codex thread id; pass "unknown" only when
the runtime gave none, and attribution will then be unresolvable.

Profiling never blocks dispatch. `profile` exits 0 after writing either a profile or a failure
record (missing tool, unreadable or invalid sidecar, compute error, invalid revision), and 3 when
the revision already has a profile; in every case the dispatch may continue. `verify` exits 0
when every record is fully verified, 2 when hashes pass but a recompute was skipped or
unavailable (the pinned tool is not installed), 1 on any problem, including a recompute that
fails or differs, and 3 when there are no records. Recompute uses archived source copies when
available.

Ordering: `profile_frozen_before_assignment_record` compares the freeze time with when the
assignment record was written. Pass --selected-at (when the model was actually chosen) to also
record `profile_frozen_before_selection`. Attribution counts every ledger call of the child
sessions unless --since/--until bound it; a retained or reused thread needs a window.
"""
import argparse
import collections
import contextlib
import fcntl
import datetime as dt
import fnmatch
import glob
import hashlib
import json
import math
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile

PIN = {"repository": "https://github.com/bline/agent-routing-lab", "tag": "v0.2.2",
       "commit": "373c12f301ba2d7859522e67e506940d6a424ef3"}
TOOLS_ROOT = os.path.expanduser("~/.local/state/work-engine/tools/agent-routing-lab")
REVISION = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}@[0-9a-f]{16}\Z")
SUMS = "SHA256SUMS"


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds")


def parse_ts(text):
    return dt.datetime.fromisoformat(text.replace("Z", "+00:00"))


def sha256(path):
    with open(path, "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def recorder():
    path = os.path.realpath(__file__)
    return {"path": path, "sha256": sha256(path)}


def read_json(path):
    with open(path) as fh:
        return json.load(fh)


def read_text(path):
    with open(path, encoding="utf-8", errors="replace") as fh:
        return fh.read()


def write_once(path, data):
    if os.path.exists(path):
        raise SystemExit(f"refusing to overwrite {path}: records are write-once")
    with open(path, "x") as fh:
        if isinstance(data, (dict, list)):
            json.dump(data, fh, indent=1, sort_keys=True)
            fh.write("\n")
        else:
            fh.write(data)


def listed(d):
    path = os.path.join(d, SUMS)
    if not os.path.exists(path):
        return {}
    out = {}
    for line in read_text(path).splitlines():
        digest, name = line.split("  ", 1)
        out[name] = digest
    return out


def record_files(d):
    return sorted(n for n in os.listdir(d) if n != SUMS and not n.startswith(".") and os.path.isfile(os.path.join(d, n)))


def prior_files(d):
    return {n: sha256(os.path.join(d, n)) for n in record_files(d)}


def append_sums(d):
    """Append hashes for files not yet listed. A listed file whose bytes changed is refused."""
    have = listed(d)
    for name, digest in have.items():
        p = os.path.join(d, name)
        if not os.path.exists(p) or sha256(p) != digest:
            raise SystemExit(f"integrity: {name} in {d} no longer matches {SUMS}")
    new = [n for n in record_files(d) if n not in have]
    with open(os.path.join(d, SUMS), "a") as fh:
        fh.writelines(f"{sha256(os.path.join(d, n))}  {n}\n" for n in new)


def tool(tool_dir, commit):
    """Path to the taskprofile CLI in tool_dir after checking it was installed from `commit`."""
    exe = os.path.join(tool_dir, "bin", "taskprofile")
    found = glob.glob(os.path.join(tool_dir, "lib", "python*", "site-packages", "agent_routing_lab-*.dist-info", "direct_url.json"))
    if not os.path.isfile(exe) or not found:
        raise SystemExit(f"pinned tool not installed in {tool_dir} (see the install note in this script)")
    installed = (read_json(found[0]).get("vcs_info") or {}).get("commit_id")
    if installed != commit:
        raise SystemExit(f"tool in {tool_dir} is {installed}, expected {commit}")
    return exe


def record_dir(run_dir, rev):
    if not isinstance(rev, str) or not REVISION.match(rev):
        raise SystemExit(f"not a valid task revision id: {rev!r}")
    return os.path.join(run_dir, "task-profiles", rev)


def write_overhead_and_sums(a, d, validation, unknown, frozen_at, sources=None):
    try:
        wall = round((parse_ts(frozen_at) - parse_ts(a.declare_started_at)).total_seconds(), 3)
    except ValueError:
        wall = None
    write_once(os.path.join(d, "overhead.json"), {
        "declare_started_at": a.declare_started_at, "frozen_at": frozen_at, "declare_wall_seconds": wall,
        "coordinator_call_ids": a.call_id or [], "tool": {**PIN, "tool_dir": a.tool_dir},
        "validation_errors": len(validation["errors"]), "unknown_declared_fields": unknown,
        "sources": sources or {}, "recorder": recorder(), "prior_files": prior_files(d)})
    append_sums(d)


def record_failure(a, raw, stage, detail):
    """An instrumentation failure: recorded, never blocking."""
    frozen_at = now()
    task_id = "unknown"
    try:
        task_id = re.sub(r"[^A-Za-z0-9._-]", "-", str(json.loads(raw)["identity"]["task_id"]))[:100] or "unknown"
    except Exception:
        pass
    key = hashlib.sha256((raw or b"") + stage.encode() + detail.encode()).hexdigest()[:16]
    d = os.path.join(a.run_dir, "task-profiles", f"{task_id}@{key}")
    if os.path.exists(d):
        print(json.dumps({"record_dir": d, "failure": stage, "note": "this failure is already recorded", "dispatch": "may continue"}, indent=1))
        sys.exit(3)
    os.makedirs(d)
    if raw is not None:
        write_once(os.path.join(d, "sidecar-stageB.json"), raw.decode("utf-8", errors="replace"))
    write_once(os.path.join(d, "profile-failed.json"), {"stage": stage, "detail": detail[-4000:]})
    validation = {"valid": False, "errors": [f"{stage} failed"]}
    write_once(os.path.join(d, "validation.json"), validation)
    try:   # keep the packet bytes recoverable when the sidecar at least parses
        sources = archive_sources(json.loads(raw), d)
    except Exception:
        sources = None
    write_overhead_and_sums(a, d, validation, None, frozen_at, sources)
    print(json.dumps({"task_revision": None, "record_dir": d, "valid": False, "failure": stage, "dispatch": "may continue"}, indent=1))


def archive_sources(prof, d):
    """Copy the packet and authority documents into sources/<sha256>, so the bytes stay recoverable."""
    wanted = {doc["sha256"]: doc["path"] for doc in prof["provenance"]["source_documents"]}
    refs = prof["input_snapshot"]["authority_refs"]
    if refs["status"] == "declared":
        wanted.update({r["sha256"]: r["path"] for r in refs["value"]})
    out, src_dir = {}, os.path.join(d, "sources")
    for digest, path in sorted(wanted.items()):
        if os.path.isfile(path) and sha256(path) == digest:
            os.makedirs(src_dir, exist_ok=True)
            with open(path, "rb") as fh, open(os.path.join(src_dir, digest), "xb") as dst:
                dst.write(fh.read())
            out[digest] = {"path": path, "archived": True}
        else:
            out[digest] = {"path": path, "archived": False, "reason": "missing or hash differs"}
    return out


def cmd_profile(a):
    try:
        with open(a.sidecar, "rb") as fh:
            raw = fh.read()
        json.loads(raw)
    except (OSError, ValueError) as e:
        return record_failure(a, raw if "raw" in locals() else None, "read-sidecar", str(e))
    try:
        exe = tool(a.tool_dir, PIN["commit"])
    except SystemExit as e:
        return record_failure(a, raw, "tool", str(e))
    staging = os.path.join(a.run_dir, "task-profiles", ".staging")
    os.makedirs(staging, exist_ok=True)
    out = os.path.join(staging, f"profile-{os.getpid()}.json")
    comp = subprocess.run([exe, "compute", a.sidecar, "--repo", a.repo, "-o", out], capture_output=True, text=True)
    try:
        if comp.returncode != 0:
            return record_failure(a, raw, "compute", comp.stderr)
        prof = read_json(out)
        rev = prof["computed"]["task_revision"]["id"]
        if not isinstance(rev, str) or not REVISION.match(rev):
            return record_failure(a, raw, "revision", f"invalid task revision id {rev!r}")
        d = os.path.join(a.run_dir, "task-profiles", rev)
        if os.path.exists(d):
            print(json.dumps({"task_revision": rev, "record_dir": d, "note": "already profiled", "dispatch": "may continue"}, indent=1))
            sys.exit(3)
        os.makedirs(d)
        frozen_at = now()
        write_once(os.path.join(d, "sidecar-stageB.json"), raw.decode("utf-8", errors="replace"))
        os.replace(out, os.path.join(d, "profile-stageB.json"))
        val = subprocess.run([exe, "validate", os.path.join(d, "profile-stageB.json"), "--production"], capture_output=True, text=True)
        validation = {"valid": val.returncode == 0, "errors": [] if val.returncode == 0 else [ln for ln in val.stdout.splitlines() if ln.strip()]}
        write_once(os.path.join(d, "validation.json"), validation)
        unknown = sum(1 for c in prof["coverage"] if c["status"] == "unknown" and c["field"].startswith("declared."))
        write_overhead_and_sums(a, d, validation, unknown, frozen_at, archive_sources(prof, d))
        print(json.dumps({"task_revision": rev, "record_dir": d, "valid": validation["valid"]}, indent=1))
    finally:
        if os.path.exists(out):
            os.remove(out)
        try:
            os.rmdir(staging)
        except OSError:
            pass  # another profile is staging concurrently


def assign_record(a):
    d = record_dir(a.run_dir, a.task_revision)
    if not os.path.isdir(d):
        raise SystemExit(f"no profile record for {a.task_revision} in {a.run_dir}")
    existing = sorted(glob.glob(os.path.join(d, "assignment*.json")))
    if existing and not (a.retained or a.episode):
        raise SystemExit(f"{existing[-1]} exists; pass --retained for a retained continuation")
    name = "assignment.json" if not existing else f"assignment-{len(existing) + 1:02d}.json"
    prof = os.path.join(d, "profile-stageB.json")
    prof = prof if os.path.exists(prof) else os.path.join(d, "profile-failed.json")
    assigned_at = now()
    frozen_at = read_json(os.path.join(d, "overhead.json"))["frozen_at"]
    packet = read_text(a.dispatched_packet)
    # The revision, its hash and any mention of the pilot; case-insensitive. Reported, never blocking.
    tokens = [a.task_revision, a.task_revision.split("@")[-1], "task-profile", "sidecar", "profile-stageB",
              "task_profile_record", "dogfood-selection-plan"]
    leaked = {t for t in tokens if t.lower() in packet.lower()}
    if a.role == "reviewer":
        leaked.update(t for t in ("gpt-6.1-terra", "gpt-6.1-sol", "Sol 6.1") if t.lower() in packet.lower())
        if re.search(r"\bTerra\b", packet, re.IGNORECASE):
            leaked.add("Terra")
    leaked = sorted(leaked)
    children = [c for c in a.child_session if c != "unknown"]
    write_once(os.path.join(d, name), {
        "task_revision": a.task_revision, "profile_file": os.path.basename(prof), "profile_sha256": sha256(prof),
        "profile_frozen_before_assignment_record": parse_ts(frozen_at) < parse_ts(assigned_at),
        "selected_at": a.selected_at,
        "profile_frozen_before_selection": (parse_ts(frozen_at) < parse_ts(a.selected_at)) if a.selected_at else None,
        "frozen_at": frozen_at, "assigned_at": assigned_at, "role": a.role, "run": a.run,
        "requested": {"model": a.requested_model, "effort": a.requested_effort, "harness": a.harness},
        "reported": {"model": a.reported_model or "unknown", "effort": a.reported_effort or "unknown", "source": "coordinator"},
        "child_sessions": children,
        "attribution": "pending: run `attribute` against the usage ledger" if children else "unresolvable: no child session id recorded",
        "retained_continuation": bool(existing),
        **({"episode_dispatch": True} if a.episode else {}),
        "dispatched_packet": {"path": os.path.abspath(a.dispatched_packet), "sha256": sha256(a.dispatched_packet)},
        "profile_absent_from_dispatch": not leaked, "leaked_tokens": leaked,
        "recorder": recorder(), "prior_files": prior_files(d)})
    append_sums(d)
    return {"assignment": os.path.join(d, name), "profile_absent_from_dispatch": not leaked}


def cmd_assign(a):
    if a.episode or os.path.isfile(os.path.join(a.run_dir, "round.json")):
        return run_round_command(a)
    print(json.dumps(assign_record(a), indent=1))


def cmd_attribute(a):
    """Resolve recorded child sessions from the usage ledger, once per assignment."""
    if a.all:
        # Serialize with round assignments and other batch attribution runs.
        with round_lock(a.run_dir):
            paths = sorted(glob.glob(os.path.join(a.run_dir, "task-profiles", "*", "assignment*.json")))
            pending = [p for p in paths if read_json(p).get("child_sessions") and not os.path.exists(
                os.path.join(os.path.dirname(p), "attribution-" + os.path.basename(p)))]
            results = []
            if pending:
                rows, digest = attribution_ledger(a.ledger)
                results = [attribute_assignment(a, p, rows, digest) for p in pending]
        print(json.dumps({"attributions": results}, indent=1))
        return
    d = record_dir(a.run_dir, a.task_revision)
    names = sorted(glob.glob(os.path.join(d, "assignment*.json")))
    if not names:
        raise SystemExit("no assignment to attribute")
    path = os.path.join(d, a.assignment) if a.assignment else names[-1]
    rows, digest = attribution_ledger(a.ledger)
    print(json.dumps(attribute_assignment(a, path, rows, digest), indent=1))


def attribution_ledger(ledger):
    """Bind attribution rows and their digest to one read of the ledger bytes."""
    with open(os.path.join(ledger, "calls.jsonl"), "rb") as fh:
        raw = fh.read()
    return [json.loads(line) for line in raw.splitlines()], hashlib.sha256(raw).hexdigest()


def attribute_assignment(a, path, ledger_rows, ledger_digest):
    d = os.path.dirname(path)
    asg = read_json(path)
    children = set(asg["child_sessions"])
    calls = collections.defaultdict(lambda: collections.Counter())
    span = {}
    since = parse_ts(a.since) if a.since else None
    until = parse_ts(a.until) if a.until else None
    if children:
        for r in ledger_rows:
            cid = r.get("thread") if r.get("provider") == "codex" else r.get("session")
            if cid in children:
                ts = parse_ts(r["ts"])
                if (since and ts < since) or (until and ts >= until):
                    continue
                calls[cid][(r.get("model"), r.get("effort"))] += 1
                lo, hi = span.get(cid, (r["ts"], r["ts"]))
                span[cid] = (min(lo, r["ts"]), max(hi, r["ts"]))
    req = asg["requested"]
    seen = sorted({m for c in calls.values() for (m, _e) in c})
    efforts = sorted({e for c in calls.values() for (_m, e) in c if e})
    if not children:
        status = "unresolvable: no child session id recorded"
    elif not calls:
        status = "unresolved: no ledger calls for the recorded child sessions"
    else:
        status = "resolved"
    out = os.path.join(d, f"attribution-{os.path.splitext(os.path.basename(path))[0]}.json")
    write_once(out, {
        "assignment": os.path.basename(path), "assignment_sha256": sha256(path), "status": status,
        "ledger": os.path.abspath(a.ledger), "ledger_calls_sha256": ledger_digest,
        "window": {"since": a.since, "until": a.until, "bounded": bool(a.since or a.until)},
        "per_session": {cid: {"calls": [{"model": m, "effort": e, "n": n} for (m, e), n in sorted(c.items(), key=str)],
                              "first_ts": span[cid][0], "last_ts": span[cid][1]} for cid, c in sorted(calls.items())},
        "models_seen": seen, "efforts_seen": efforts,
        "model_matches_request": (seen == [req["model"]]) if calls else None,
        "effort_matches_request": (efforts == [req["effort"]]) if efforts else None,
        "recorder": recorder(), "prior_files": prior_files(d)})
    append_sums(d)
    return {"attribution": out, "status": status, "models_seen": seen}


class RecomputeUnavailable(Exception):
    """The pinned tool is not installed where verify looks for it."""


def verify_records(a):
    worst = 0  # 0 fully verified, 2 recompute not done, 1 problem
    dirs = sorted(glob.glob(os.path.join(a.run_dir, "task-profiles", "*@*")))
    episodes = enrollment_dirs(a.run_dir)
    if not dirs and not episodes:
        print(f"no task-profile records or episode records under {a.run_dir}")
        sys.exit(3)
    for d in episodes:
        try:
            report = recover_episode(d)
            if report["relisted"] or report["damaged_copy"]:
                print(f"{os.path.relpath(d, a.run_dir)}: re-listed {report['relisted']}; damaged copy: {report['damaged_copy']}")
            problems = integrity_problems(d)
        except (IntegrityFailure, OSError, ValueError) as e:
            problems = [str(e)]
        print(f"{os.path.relpath(d, a.run_dir)}: " + ("; ".join(problems) if problems else "verified"))
        if problems:
            worst = 1
    for d in dirs:
        problems, notes = [], []
        have = listed(d)
        files = record_files(d)
        if not have:
            problems.append(f"{SUMS} missing or empty")
        problems += [f"unlisted file: {n}" for n in files if n not in have]
        problems += [f"listed but missing: {n}" for n in have if n not in files]
        problems += [f"hash mismatch: {n}" for n in files if n in have and sha256(os.path.join(d, n)) != have[n]]
        for n in files:
            if n.endswith(".json"):
                try:
                    rec = read_json(os.path.join(d, n))
                except ValueError:
                    problems.append(f"not valid JSON: {n}")
                    continue
                for pn, pd in (rec.get("prior_files") or {}).items():
                    p = os.path.join(d, pn)
                    if not os.path.exists(p) or sha256(p) != pd:
                        problems.append(f"{n} recorded {pn} with a different hash")
                if "profile_sha256" in rec:
                    pf = os.path.join(d, rec.get("profile_file", "profile-stageB.json"))
                    if not os.path.exists(pf) or sha256(pf) != rec["profile_sha256"]:
                        problems.append(f"{n}: profile hash differs")
        src_dir = os.path.join(d, "sources")
        for name in sorted(os.listdir(src_dir)) if os.path.isdir(src_dir) else []:
            if sha256(os.path.join(src_dir, name)) != name:
                problems.append(f"archived source sources/{name} does not match its hash")
        if os.path.exists(os.path.join(d, "overhead.json")):
            try:
                source_listing = read_json(os.path.join(d, "overhead.json")).get("sources") or {}
                for digest, info in source_listing.items():
                    if info.get("archived") and not os.path.isfile(os.path.join(src_dir, digest)):
                        problems.append(f"archived source {digest[:12]} is missing")
            except (ValueError, OSError, TypeError, AttributeError) as e:
                problems.append(f"cannot inspect archived source listing: {e}")
        prof = os.path.join(d, "profile-stageB.json")
        if os.path.exists(prof):
            if a.no_recompute or not a.repo:
                notes.append("recompute not done")
            else:
                try:
                    overhead = read_json(os.path.join(d, "overhead.json"))
                    pin = overhead["tool"]["commit"]
                    try:
                        exe = tool(os.path.join(a.tools_root, pin), pin)
                    except SystemExit as e:
                        raise RecomputeUnavailable(e) from None
                    sidecar = read_json(os.path.join(d, "sidecar-stageB.json"))
                    sources = overhead.get("sources") or {}
                    groups = [("source_documents", sidecar["provenance"]["source_documents"])]
                    refs = sidecar["input_snapshot"]["authority_refs"]
                    if refs["status"] == "declared":
                        groups.append(("authority_refs", refs["value"]))
                    missing, aliases = [], []
                    with tempfile.TemporaryDirectory() as tmp:
                        for group, entries in groups:
                            for index, entry in enumerate(entries):
                                original, digest = entry["path"], entry["sha256"]
                                archived = os.path.join(src_dir, digest)
                                if sources.get(digest, {}).get("archived") and os.path.isfile(archived):
                                    alias = os.path.join(tmp, f"{group}-{index}-{digest}")
                                    shutil.copyfile(archived, alias)
                                    entry["path"] = alias
                                    aliases.append((group, index, alias, original))
                                else:
                                    missing.append(original)
                        sidecar_copy = os.path.join(tmp, "sidecar.json")
                        output = os.path.join(tmp, "profile.json")
                        with open(sidecar_copy, "w") as fh:
                            json.dump(sidecar, fh)
                        subprocess.run([exe, "compute", sidecar_copy, "--repo", a.repo, "-o", output], check=True)
                        computed = read_json(output)
                        for group, index, alias, original in aliases:
                            try:
                                entries = (computed["provenance"]["source_documents"] if group == "source_documents"
                                           else computed["input_snapshot"]["authority_refs"]["value"])
                                if entries[index]["path"] == alias:
                                    entries[index]["path"] = original
                            except (KeyError, IndexError, TypeError):
                                pass  # The comparison below reports the missing or malformed location.
                        canonical = lambda value: json.dumps(value, sort_keys=True, separators=(",", ":"))
                        same = canonical(computed) == canonical(read_json(prof))
                    if not same:
                        detail = "; inputs without archived copies: " + ", ".join(sorted(set(missing))) if missing else ""
                        problems.append("recompute differs from profile-stageB.json" + detail)
                except RecomputeUnavailable as e:  # not established, not a record problem
                    notes.append(f"recompute unavailable: {e}")
                except (Exception, SystemExit) as e:
                    problems.append(f"recompute failed: {e}")
        state = 1 if problems else (2 if notes else 0)
        worst = 1 if 1 in (worst, state) else max(worst, state)
        label = {0: "verified", 2: "hashes ok; " + "; ".join(notes), 1: "; ".join(problems)}[state]
        print(f"{os.path.basename(d)}: {label}")
    sys.exit(worst)


def cmd_verify(a):
    RELISTING_REPORTS.clear()
    if os.path.isfile(os.path.join(a.run_dir, "round.json")):
        with round_lock(a.run_dir):
            verify_records(a)
    else:
        verify_records(a)


TERRA = "gpt-6.1-terra"
SOL = "gpt-6.1-sol"
ROUND_COMMANDS = {"init-round", "enroll", "withdraw", "review", "repair", "close", "pause", "resume", "status", "recover-screening"}
TASK_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")
EPISODE_RECORD = re.compile(r"(?:enrollment|review-first|repair-[0-9]{2,}|close|withdraw)\.json\Z")
RELISTING_REPORTS = []  # Per-command output only; never persisted as another recovery log.


class UsageRefusal(Exception):
    pass


class IntegrityFailure(Exception):
    pass


def nonnegative(text):
    value = int(text)
    if value < 0:
        raise argparse.ArgumentTypeError("must be nonnegative")
    return value


def episode_dir(run_dir, task_id):
    if not TASK_ID.fullmatch(task_id):
        raise UsageRefusal("invalid task ID")
    return os.path.join(run_dir, "enrollments", task_id)


def content_sha256(data):
    payload = {k: v for k, v in data.items() if k != "content_sha256"}
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def seal_episode_record(data):
    return {**data, "content_sha256": content_sha256(data)}


def read_episode_record(path):
    try:
        data = read_json(path)
        if not isinstance(data, dict) or data.get("content_sha256") != content_sha256(data):
            raise IntegrityFailure(f"{path}: content_sha256 mismatch or missing")
        return data
    except (OSError, ValueError, TypeError) as e:
        raise IntegrityFailure(f"{path}: invalid episode record: {e}") from e


def integrity_problems(d):
    """Check an episode's complete hash list and its backward record links."""
    problems = []
    try:
        have = listed(d)
        files = record_files(d)
        if not have:
            problems.append(f"{SUMS} missing or empty")
        problems += [f"unlisted file: {n}" for n in files if n not in have]
        problems += [f"listed but missing: {n}" for n in have if n not in files]
        problems += [f"hash mismatch: {n}" for n in files if n in have and sha256(os.path.join(d, n)) != have[n]]
        for n in files:
            if not n.endswith(".json"):
                continue
            data = read_episode_record(os.path.join(d, n))
            for pn, digest in data.get("prior_files", {}).items():
                if pn not in files or sha256(os.path.join(d, pn)) != digest:
                    problems.append(f"{n} recorded {pn} with a different hash")
    except (IntegrityFailure, OSError, ValueError, TypeError, AttributeError) as e:
        problems.append(f"unreadable integrity record: {e}")
    return problems


def fsync_directory(d):
    fd = os.open(d, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def atomic_publish(path, data):
    """Publish complete bytes without replacing an existing record."""
    d = os.path.dirname(os.fspath(path))
    fd, tmp = tempfile.mkstemp(prefix=".record-", dir=d)
    try:
        with os.fdopen(fd, "wb") as fh:
            raw = data if isinstance(data, bytes) else (json.dumps(data, indent=1, sort_keys=True) + "\n").encode()
            fh.write(raw)
            fh.flush()
            os.fsync(fh.fileno())
        os.link(tmp, path)  # Same-directory, atomic publication; EEXIST preserves the winner.
    finally:
        os.unlink(tmp)
        fsync_directory(d)


def durable_sums(d):
    append_sums(d)
    with open(os.path.join(d, SUMS), "rb") as fh:
        os.fsync(fh.fileno())
    fsync_directory(d)


def repair_manifest_tail(d):
    """Preserve a torn final line; complete entries remain authoritative."""
    path = os.path.join(d, SUMS)
    if not os.path.exists(path):
        return None
    with open(path, "rb") as fh:
        raw = fh.read()
    lines = raw.splitlines(keepends=True)
    torn = bool(raw) and not raw.endswith(b"\n")
    for index, line in enumerate(lines):
        match = re.fullmatch(rb"([0-9a-f]{64})  ([^\r\n]+)\n?", line)
        if match:
            digest, name = match.group(1).decode(), match.group(2).decode("utf-8")
            if name not in record_files(d):
                if torn and index == len(lines) - 1:
                    continue  # A torn append can stop inside the filename.
                raise IntegrityFailure(f"{d}: listed but missing: {name}")
            if sha256(os.path.join(d, name)) != digest:
                raise IntegrityFailure(f"{d}: hash mismatch: {name}")
        elif not (torn and index == len(lines) - 1):
            raise IntegrityFailure(f"{path}: malformed complete line {index + 1}")
    if not torn:
        return None
    backup = path + ".damaged-" + now().replace(":", "-")
    atomic_publish(backup, raw)
    with open(path, "r+b") as fh:
        fh.truncate(raw.rfind(b"\n") + 1)
        fh.flush()
        os.fsync(fh.fileno())
    return backup


def recover_episode(d):
    """Re-list only self-verifying records; no separate recovery log is trusted."""
    for name in record_files(d):
        if EPISODE_RECORD.fullmatch(name):
            read_episode_record(os.path.join(d, name))
    backup = repair_manifest_tail(d)
    report = {"episode": os.path.basename(d), "relisted": [], "damaged_copy": backup}
    if backup:
        RELISTING_REPORTS.append(report)
    problems = integrity_problems(d)
    fatal = [p for p in problems if not (p == f"{SUMS} missing or empty" or
             (p.startswith("unlisted file: ") and (EPISODE_RECORD.fullmatch(p[len("unlisted file: "):]) or
              p[len("unlisted file: "):].startswith(SUMS + ".damaged-"))))]
    if fatal:
        raise IntegrityFailure(f"{d}: " + "; ".join(fatal))
    files = record_files(d)
    have = listed(d)
    missing = [n for n in files if n not in have]
    if missing:
        if not backup:
            RELISTING_REPORTS.append(report)
        durable_sums(d)
        report["relisted"] = missing
    if not files:
        return report
    remaining = integrity_problems(d)
    if remaining:
        raise IntegrityFailure("; ".join(remaining))
    return report


def episode_write(d, name, data):
    recover_episode(d)
    if os.path.exists(os.path.join(d, name)):
        raise UsageRefusal(f"{name} already exists: records are write-once")
    data = seal_episode_record({**data, "recorder": recorder(), "prior_files": prior_files(d)})
    atomic_publish(os.path.join(d, name), data)
    durable_sums(d)
    return data


def enrollment_dirs(run_dir):
    return sorted(d for d in glob.glob(os.path.join(run_dir, "enrollments", "*")) if os.path.isdir(d))


def enrollments(run_dir):
    out = []
    for d in enrollment_dirs(run_dir):
        recover_episode(d)
        path = os.path.join(d, "enrollment.json")
        if os.path.isfile(path):
            out.append((d, read_episode_record(path)))
    return out


def terminal(d):
    return any(os.path.exists(os.path.join(d, n)) for n in ("close.json", "withdraw.json"))


def terra_repairs(run_dir):
    return sum(len(glob.glob(os.path.join(d, "repair-*.json"))) for d, e in enrollments(run_dir) if e["arm"] == TERRA)


def log_tail_problem(raw):
    if not raw:
        return None
    if not raw.endswith(b"\n"):
        return "log does not end at a line boundary"
    try:
        if not isinstance(json.loads(raw.splitlines()[-1]), dict):
            return "last log line is not an object"
    except (ValueError, UnicodeError):
        return "last log line is malformed JSON"
    return None


def append_event(path, data):
    if os.path.exists(path):
        with open(path, "rb") as fh:
            problem = log_tail_problem(fh.read())
        if problem:
            raise IntegrityFailure(f"{path}: {problem}; explicit recovery required")
    with open(path, "a") as fh:
        fh.write(json.dumps(data, sort_keys=True) + "\n")
        fh.flush()
        os.fsync(fh.fileno())


def screening(run_dir, data):
    append_event(os.path.join(run_dir, "screening.jsonl"), data)


def cmd_recover_screening(a):
    path = os.path.join(a.run_dir, "screening.jsonl")
    if not os.path.exists(path):
        return {"outcome": "no_recovery_needed"}
    with open(path, "rb") as fh:
        raw = fh.read()
    if not log_tail_problem(raw):
        return {"outcome": "no_recovery_needed"}
    backup = os.path.join(a.run_dir, "screening-damaged-" + now().replace(":", "-") + ".jsonl")
    atomic_publish(backup, raw)
    # Preserve every earlier byte, including any malformed interior lines.
    tail_end = len(raw) - 1 if raw.endswith(b"\n") else len(raw)
    prefix = raw[:raw.rfind(b"\n", 0, tail_end) + 1]
    with open(path, "r+b") as fh:
        fh.truncate(len(prefix))
        fh.flush()
        os.fsync(fh.fileno())
    return {"outcome": "recovered", "backup": backup, "removed_bytes": len(raw) - len(prefix)}


def trigger_facts(run_dir, episodes=None):
    return episode_trigger_facts(enrollments(run_dir) if episodes is None else episodes)


def episode_trigger_facts(episodes):
    """The trigger facts of already-listed episodes. Read-only: it opens no file for writing."""
    p1 = []
    repairs = 0
    for d, e in episodes:
        if e["arm"] != TERRA:
            continue
        review = os.path.join(d, "review-first.json")
        if os.path.exists(review) and read_episode_record(review)["p1"] > 0:
            p1.append(e["task_id"])
        repairs += len(glob.glob(os.path.join(d, "repair-*.json")))
    return {"p1_episodes": sorted(p1), "terra_repair_rounds": repairs}


def pause_coverage(run_dir):
    """The triggers already acknowledged by pause history: (covered P1 episodes, covered repair rounds). Read-only."""
    covered_p1, covered_repairs = set(), -1
    for path in glob.glob(os.path.join(run_dir, "pauses", "*.json")):
        covered = read_json(path).get("covered_triggers", {})
        covered_p1.update(covered.get("p1_episodes", []))
        covered_repairs = max(covered_repairs, covered.get("terra_repair_rounds", -1))
    return covered_p1, covered_repairs


def terra_stop_rule(facts, covered_p1, covered_repairs, budget):
    """The one Terra stop rule. Pure: (uncovered P1 episodes, whether repair rounds are over budget and uncovered).

    A stop is a non-empty first element or a true second element. enforce_triggers writes PAUSED on a stop;
    the admission command only reports it.
    """
    new_p1 = sorted(set(facts["p1_episodes"]) - covered_p1)
    over_budget = facts["terra_repair_rounds"] >= budget and facts["terra_repair_rounds"] > covered_repairs
    return new_p1, over_budget


def enforce_triggers(run_dir, episodes=None):
    episodes = enrollments(run_dir) if episodes is None else episodes
    facts = episode_trigger_facts(episodes)
    covered_p1, covered_repairs = pause_coverage(run_dir)
    budget = read_json(os.path.join(run_dir, "round.json"))["terra_repair_budget"]
    new_p1, over_budget = terra_stop_rule(facts, covered_p1, covered_repairs, budget)
    if new_p1 or over_budget:
        reason = f"Terra trigger: P1 episodes {new_p1}; repair rounds {facts['terra_repair_rounds']} (budget {budget})"
        pause_round(run_dir, reason)
    return facts


def pause_round(run_dir, reason):
    path = os.path.join(run_dir, "PAUSED")
    if os.path.exists(path):
        return {"outcome": "already_paused", "pause": read_json(path)}
    data = {"reason": reason, "paused_at": now(), "recorder": recorder()}
    atomic_publish(path, data)
    return {"outcome": "paused", "pause": data}


def cmd_init_round(a):
    path = os.path.join(a.run_dir, "round.json")
    if os.path.exists(path):
        raise UsageRefusal("round.json already exists")
    data = {"cap": a.cap, "terra_in_flight": a.terra_in_flight, "terra_repair_budget": a.terra_repair_budget,
            "initialized_at": now(), "recorder": recorder()}
    write_once(path, data)
    return {"outcome": "initialized", "round": data}


def cmd_enroll(a):
    d = episode_dir(a.run_dir, a.task_id)
    if a.packet:
        identity = {"packet_sha256": sha256(a.packet)}
    else:
        try:
            record_dir(a.run_dir, a.task_revision)
        except SystemExit as e:
            raise UsageRefusal(str(e)) from e
        if a.task_revision.split("@")[0] != a.task_id:
            raise UsageRefusal("task revision belongs to a different task")
        identity = {"task_revision": a.task_revision}
    screened = {"time": now(), "task_id": a.task_id, **identity,
                "eligible": a.eligible == "yes", "reason": a.reason}
    if not a.packet:
        screening(a.run_dir, {"event": "screened", **screened})
    episodes = enrollments(a.run_dir)
    enforce_triggers(a.run_dir, episodes)
    path = os.path.join(d, "enrollment.json")
    out = {"task_id": a.task_id}
    if os.path.exists(path):
        out.update(outcome="already_enrolled", arm=read_episode_record(path)["arm"])
    elif not screened["eligible"]:
        out["outcome"] = "not_eligible"
    elif os.path.exists(os.path.join(a.run_dir, "PAUSED")):
        out["outcome"] = "paused"
    else:
        config = read_json(os.path.join(a.run_dir, "round.json"))
        open_terra = sum(e["arm"] == TERRA and not terminal(ed) for ed, e in episodes)
        if len(episodes) >= config["cap"]:
            out["outcome"] = "cap_reached"
        elif open_terra >= config["terra_in_flight"]:
            out["outcome"] = "in_flight_limit"
        elif a.packet:
            return {**out, "outcome": "profile_required"}
        else:
            byte = os.urandom(1)[0]
            arm = TERRA if byte & 1 else SOL
            data = {**screened, "screened_at": screened["time"], "enrolled_at": now(), "arm": arm,
                    "draw_source": "os.urandom(1)[0] & 1", "draw_byte": byte, "recorder": recorder()}
            data = seal_episode_record(data)
            os.makedirs(d, exist_ok=True)
            try:
                atomic_publish(path, data)
            except FileExistsError:
                recover_episode(d)
                out.update(outcome="already_enrolled", arm=read_episode_record(path)["arm"])
            else:
                durable_sums(d)
                out.update(outcome="enrolled", arm=arm)
    if a.packet:
        screening(a.run_dir, {"event": "screened", **screened, "recorder": recorder()})
    screening(a.run_dir, {"event": "result", "time": now(), **identity, **out})
    return out


def active_episode(a, allow_terminal=False):
    d = episode_dir(a.run_dir, a.task_id)
    path = os.path.join(d, "enrollment.json")
    if not os.path.isfile(path):
        raise UsageRefusal("task is not enrolled")
    recover_episode(d)
    if terminal(d) and not allow_terminal:
        raise UsageRefusal("episode is already closed or withdrawn")
    return d, read_episode_record(path)


def assignments(run_dir, task_id):
    paths = glob.glob(os.path.join(run_dir, "task-profiles", task_id + "@*", "assignment*.json"))
    return [{"path": os.path.abspath(p), "sha256": sha256(p), "assigned_at": read_json(p)["assigned_at"]}
            for p in sorted(paths)]


def file_evidence(paths):
    return [{"path": os.path.abspath(p), "sha256": sha256(p)} for p in paths]


def cmd_withdraw(a):
    d, _ = active_episode(a)
    if glob.glob(os.path.join(a.run_dir, "task-profiles", a.task_id + "@*", "assignment*.json")):
        raise UsageRefusal("cannot withdraw after dispatch")
    data = episode_write(d, "withdraw.json", {"task_id": a.task_id, "reason": a.reason, "withdrawn_at": now()})
    return {"outcome": "withdrawn", "record": data}


def cmd_review(a):
    if a.verdict == "accept" and (a.p1 or a.p2):
        raise UsageRefusal("accept requires zero P1 and P2 findings")
    if a.verdict == "changes_required" and not (a.p1 or a.p2):
        raise UsageRefusal("changes_required requires a P1 or P2 finding")
    d, _ = active_episode(a)
    data = episode_write(d, "review-first.json", {"task_id": a.task_id, "verdict": a.verdict,
                         "first_review_pass": a.verdict == "accept", "reviewer_session": a.reviewer_session,
                         "p1": a.p1, "p2": a.p2, "p3": a.p3, "receipt": file_evidence([a.receipt])[0], "reviewed_at": now()})
    enforce_triggers(a.run_dir)
    return {"outcome": "reviewed", "record": data}


def cmd_repair(a):
    d, _ = active_episode(a, allow_terminal=True)
    receipt = file_evidence([a.review_receipt])[0]
    rounds = sorted(glob.glob(os.path.join(d, "repair-*.json")))
    for path in rounds:
        record = read_episode_record(path)
        if record["review_receipt"]["sha256"] == receipt["sha256"]:
            facts = enforce_triggers(a.run_dir)
            return {"outcome": "already_repaired", "record": record, "terra_repair_rounds": facts["terra_repair_rounds"]}
    if terminal(d):
        raise UsageRefusal("episode is already closed or withdrawn")
    number = max([int(os.path.basename(p)[7:-5]) for p in rounds], default=0) + 1
    data = episode_write(d, f"repair-{number:02d}.json", {"task_id": a.task_id, "round": number,
                         "review_receipt": receipt, "resubmitted_at": now()})
    facts = enforce_triggers(a.run_dir)
    return {"outcome": "repaired", "record": data, "terra_repair_rounds": facts["terra_repair_rounds"]}


def cmd_close(a):
    d, _ = active_episode(a)
    windows = assignments(a.run_dir, a.task_id)
    first = min((w["assigned_at"] for w in windows), key=parse_ts, default=None)
    closed = now()
    data = episode_write(d, "close.json", {"task_id": a.task_id, "disposition": a.disposition,
                         "reason": a.reason, "pre_review_fixes": a.pre_review_fixes, "escalations": a.escalations,
                         "repair_rounds": len(glob.glob(os.path.join(d, "repair-*.json"))),
                         "review_receipts": file_evidence(a.review_receipt or []), "artifacts": file_evidence(a.artifact or []),
                         "assignments": windows, "first_dispatch_at": first, "closed_at": closed,
                         "elapsed_seconds": (parse_ts(closed) - parse_ts(first)).total_seconds() if first else None})
    return {"outcome": "closed", "record": data}


def cmd_pause(a):
    return pause_round(a.run_dir, a.reason)


def cmd_resume(a):
    if not a.owner_approval.strip():
        raise UsageRefusal("owner approval text must not be empty")
    facts = enforce_triggers(a.run_dir)
    path = os.path.join(a.run_dir, "PAUSED")
    if not os.path.exists(path):
        raise UsageRefusal("round is not paused")
    data = {**read_json(path), "owner_approval": a.owner_approval, "resumed_at": now(),
            "pause_sha256": sha256(path), "resume_recorder": recorder(), "covered_triggers": facts}
    history = os.path.join(a.run_dir, "pauses")
    os.makedirs(history, exist_ok=True)
    dest = os.path.join(history, data["resumed_at"].replace(":", "-") + ".json")
    # Persist the augmented history before removing the active flag; a failed write stays paused.
    atomic_publish(dest, data)
    os.unlink(path)
    fsync_directory(a.run_dir)
    return {"outcome": "resumed", "history": dest, "record": data}


def cmd_status(a):
    episodes = enrollments(a.run_dir)
    counts = {arm: 0 for arm in (TERRA, SOL)}
    reviews = {arm: {v: 0 for v in ("accept", "changes_required", "blocked_evidence", "blocked_scope", "pending")} for arm in counts}
    reviewer_sessions = set()
    for path in glob.glob(os.path.join(a.run_dir, "task-profiles", "*", "assignment*.json")):
        assignment = read_json(path)
        if assignment.get("role") == "reviewer":
            reviewer_sessions.update(assignment.get("child_sessions", []))
    closed = withdrawn = open_terra = 0
    log_path = os.path.join(a.run_dir, "screening.jsonl")
    log, log_problems = [], []
    if os.path.exists(log_path):
        with open(log_path, "rb") as fh:
            for number, line in enumerate(fh, 1):
                try:
                    entry = json.loads(line)
                    if not isinstance(entry, dict) or not line.endswith(b"\n"):
                        raise ValueError("not a complete object line")
                    log.append(entry)
                except (ValueError, UnicodeError):
                    log_problems.append(f"screening.jsonl: malformed line {number}")
    results = collections.Counter(r["outcome"] for r in log if r.get("event") == "result")
    recorded = {r["task_id"] for r in log if r.get("event") == "result" and r.get("outcome") == "enrolled"}
    problems = list(log_problems)
    for d, e in episodes:
        arm = e["arm"]
        counts[arm] += 1
        closed += os.path.exists(os.path.join(d, "close.json"))
        withdrawn += os.path.exists(os.path.join(d, "withdraw.json"))
        open_terra += arm == TERRA and not terminal(d)
        review = os.path.join(d, "review-first.json")
        if os.path.exists(review):
            review_record = read_episode_record(review)
            reviews[arm][review_record["verdict"]] += 1
            if review_record["reviewer_session"] not in reviewer_sessions:
                problems.append(f"{e['task_id']}: reviewer_unlinked")
        else:
            reviews[arm]["pending"] += 1
        if e["task_id"] not in recorded:
            problems.append(f"{e['task_id']}: enrollment has no enrolled result line")
        problems.extend(f"{e['task_id']}: {p}" for p in integrity_problems(d))
        if os.path.exists(os.path.join(d, "close.json")) and os.path.exists(os.path.join(d, "withdraw.json")):
            problems.append(f"{e['task_id']}: both closed and withdrawn")
        if os.path.exists(os.path.join(d, "withdraw.json")) and glob.glob(os.path.join(a.run_dir, "task-profiles", e["task_id"] + "@*", "assignment*.json")):
            problems.append(f"{e['task_id']}: both withdrawal and assignment")
    known = {e["task_id"] for _, e in episodes}
    problems.extend(f"{t}: enrolled result has no enrollment" for t in sorted(recorded - known))
    problems.extend(f"{os.path.basename(d)}: episode directory has no enrollment" for d in enrollment_dirs(a.run_dir)
                    if not os.path.isfile(os.path.join(d, "enrollment.json")))
    enforce_triggers(a.run_dir, episodes)
    paused = os.path.join(a.run_dir, "PAUSED")
    out = {"round": read_json(os.path.join(a.run_dir, "round.json")), "paused": os.path.exists(paused),
           "pause": read_json(paused) if os.path.exists(paused) else None,
           "enrollments": {"total": len(episodes), "per_arm": counts}, "closed": closed, "withdrawn": withdrawn,
           "open": len(episodes) - closed - withdrawn, "open_terra": open_terra, "terra_repair_rounds": terra_repairs(a.run_dir),
           "first_reviews": reviews, "screening": {"screened": sum(r.get("event") == "screened" for r in log),
                                                 "outcomes": dict(results)}, "inconsistencies": problems}
    if a.task_id:
        d = episode_dir(a.run_dir, a.task_id)
        out["episode"] = ({n: read_episode_record(os.path.join(d, n)) for n in record_files(d) if n.endswith(".json")}
                          if os.path.isfile(os.path.join(d, "enrollment.json")) else "no record")
    return out


ADMISSION_EXIT = {"active": 0, "paused": 10, "stop_reached": 11, "unknown": 12}
ADMISSION_EPILOG = """\
Read-only gate: decide from the run directory alone whether a supervisor may screen a task. It reads no plan,
takes no lock, writes and syncs nothing, and runs no trigger. Prints one JSON object with state, reason
(code and text) and the facts it used.

Exit code and state:
  0   active        screen
  10  paused        a PAUSED entry exists; only the owner resumes. Any entry named PAUSED counts, including
                    a malformed or unreadable file, a directory and a broken symlink (os.path.lexists,
                    stricter than enroll, which uses os.path.exists and would not see a broken symlink)
  11  stop_reached  no PAUSED entry, but the pause trigger is met and unmarked (reason pause_trigger_unmarked:
                    an uncovered Terra P1 episode, or Terra repair rounds at the budget and above the covered
                    count) or enrollments are at the cap (reason cap_reached); do not screen, tell the owner
  12  unknown       the run directory, round.json, an enrollment, review-first.json or pause history is
                    missing, unreadable or fails its integrity check, or an episode needs manifest recovery
                    that enroll would perform; admission never repairs, so do not screen, tell the owner.
                    Also unknown: any directory it must list or entry it must examine that cannot be
                    (enrollments/, an episode directory, pauses/, PAUSED: not a directory, a dangling
                    symlink, a permission or I/O error; reasons enrollments_unlistable, episode_unlistable,
                    pause_history_unlistable, pause_marker_unreadable), and a pause-history file whose
                    covered_triggers is not an object, p1_episodes not a list of strings or
                    terra_repair_rounds not an integer (a boolean or float is not; reason
                    pause_history_malformed). A genuinely absent enrollments/ or pauses/ means none.
                    Also unknown: a count that is not a nonnegative integer (a boolean, float, NaN, Infinity,
                    string, null or missing value is not one) in round.json, a first review's p1 or a pause
                    history's terra_repair_rounds (reasons round_file_invalid, review_record_invalid,
                    pause_history_malformed); a review, repair or enrollment record that contradicts the
                    episode it sits in (another task, a repair number that differs from its name or repeats;
                    reasons review_record_invalid, repair_record_invalid, enrollment_record_invalid); an
                    unresolvable entry of enrollments/ (a dangling link) or a dangling link, FIFO, directory
                    or other non-regular file named enrollment.json, review-first.json, repair-*.json,
                    close.json, withdraw.json, SHA256SUMS or pauses/*.json; a file with a duplicate JSON
                    key, or NaN or Infinity in round.json or pause history; the run directory unreadable
                    (run_dir_unreadable). Files are read through a non-blocking open checked on the
                    descriptor: a FIFO is refused, never waited on.
Any other exit code, including a usage error (2) or a crash (1), is not an admission: do not screen.
The Terra in-flight limit is reported as a fact and is not a stop.
"""


class AdmissionUnknown(Exception):
    def __init__(self, code, text):
        super().__init__(text)
        self.code = code
        self.text = text


def pause_marker_fact(path):
    """Describe a PAUSED entry without writing. A FIFO or special file is never opened."""
    fact = {"present": True}
    if os.path.islink(path) and not os.path.exists(path):
        return {**fact, "kind": "broken_symlink", "valid": False}
    if os.path.isdir(path):
        return {**fact, "kind": "directory", "valid": False}
    if not os.path.isfile(path):
        return {**fact, "kind": "other", "valid": False}
    fact["kind"] = "file"
    try:
        data = read_json(path)
        if not isinstance(data, dict):
            raise ValueError("not a JSON object")
        return {**fact, "valid": True, "reason": data.get("reason"), "paused_at": data.get("paused_at")}
    except (OSError, ValueError, UnicodeError) as e:
        return {**fact, "valid": False, "error": f"{type(e).__name__}: {e}"}


SPECIAL_RECORD = re.compile(r"(?:enrollment|review-first|close|withdraw)\.json\Z")
REPAIR_GLOB = "repair-*.json"  # What the shared trigger rule counts, by glob.
REPAIR_NAME = re.compile(r"repair-([0-9]{2,})\.json\Z")  # What the recorder writes: repair-{number:02d}.json


def admission_read_bytes(path, code, label):
    """All bytes of a regular file, or AdmissionUnknown.

    The file is opened non-blocking and its type is checked on the descriptor, so a FIFO, socket or device
    that is (or has become) the entry is refused instead of waited on, and any I/O error is unknown.
    """
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
    except OSError as e:
        raise AdmissionUnknown(code, f"{label} cannot be opened: {e}") from e
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            raise AdmissionUnknown(code, f"{label} is not a regular file")
        chunks = []
        while True:
            chunk = os.read(fd, 1 << 20)
            if not chunk:
                return b"".join(chunks)
            chunks.append(chunk)
    except OSError as e:
        raise AdmissionUnknown(code, f"{label} cannot be read: {e}") from e
    finally:
        os.close(fd)


def no_duplicate_keys(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError(f"duplicate key {key!r}")
        out[key] = value
    return out


def refuse_constant(text):
    raise ValueError(f"non-finite number {text}")


def finite_float(text):
    value = float(text)
    if not math.isfinite(value):
        raise ValueError(f"non-finite number {text}")
    return value


def admission_parse_json(raw, code, label, finite=False):
    """Decode UTF-8 JSON, refusing duplicate keys (the last would silently win). With finite, also refuse
    NaN, Infinity and numbers that overflow to infinity. Episode records keep Python's defaults for those:
    their consumed fields are type-checked, so a float never passes as a count."""
    kwargs = {"parse_constant": refuse_constant, "parse_float": finite_float} if finite else {}
    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=no_duplicate_keys, **kwargs)
    except (ValueError, UnicodeError, RecursionError) as e:
        raise AdmissionUnknown(code, f"{label} is malformed: {type(e).__name__}: {e}") from e


def admission_record(raw, code, label):
    """A sealed episode record from bytes already read: an object whose content_sha256 matches."""
    data = admission_parse_json(raw, code, label)
    if not isinstance(data, dict) or data.get("content_sha256") != content_sha256(data):
        raise AdmissionUnknown(code, f"{label}: content_sha256 mismatch or missing")
    return data


def read_round_config(run_dir):
    path = os.path.join(run_dir, "round.json")
    st = admission_stat(path, "round_file_missing", "round.json", dangling_absent=True)
    if st is None or not stat.S_ISREG(st.st_mode):
        raise AdmissionUnknown("round_file_missing", "round.json is missing or not a regular file")
    config = admission_parse_json(admission_read_bytes(path, "round_file_unreadable", "round.json"),
                                  "round_file_unreadable", "round.json", finite=True)
    fields = ("cap", "terra_in_flight", "terra_repair_budget")
    if not isinstance(config, dict) or any(type(config.get(f)) is not int or config[f] < 0 for f in fields):
        raise AdmissionUnknown("round_file_invalid",
                               "round.json lacks nonnegative integer cap, terra_in_flight and terra_repair_budget")
    return {f: config[f] for f in fields}


def admission_stat(path, code, label, dangling_absent):
    """os.stat that tells absent from unexaminable: None when absent, AdmissionUnknown on any other error.

    glob and the os.path predicates (exists, isfile, isdir) turn every OSError into "not there". The gate
    cannot: a path it could not examine may hold the stop it is looking for. A dangling symlink is absent
    where dangling_absent is set (an entry the gate does not consume), else unknown.
    """
    try:
        return os.stat(path)
    except FileNotFoundError as e:
        if os.path.islink(path) and not dangling_absent:
            raise AdmissionUnknown(code, f"{label} is a dangling symlink") from e
        return None
    except OSError as e:
        raise AdmissionUnknown(code, f"{label} cannot be examined: {e}") from e


def admission_listdir(path, code, label, missing_ok):
    """Sorted names in a directory. An absent directory is [] only when missing_ok; anything else that is
    not a listable directory (a file, a dangling symlink, a permission or I/O error) is unknown."""
    st = admission_stat(path, code, label, dangling_absent=False)
    if st is None:
        if missing_ok:
            return []
        raise AdmissionUnknown(code, f"{label} is missing")
    if not stat.S_ISDIR(st.st_mode):
        raise AdmissionUnknown(code, f"{label} exists but is not a directory")
    try:
        return sorted(os.listdir(path))
    except OSError as e:
        raise AdmissionUnknown(code, f"{label} cannot be listed: {e}") from e


def admission_enrollment_dirs(run_dir):
    """enrollment_dirs() for the gate: the same set where it can be listed, unknown where it cannot.

    A genuinely absent enrollments/ is no enrollments, as for enroll. glob("*") skips hidden names, as here.
    A regular file there is not an episode and is skipped (as enrollment_dirs does). An entry that cannot be
    resolved (a dangling link, a loop, a permission error) or vanished while listing might be an episode, so
    it is unknown; enrollment_dirs would skip it.
    """
    base = os.path.join(run_dir, "enrollments")
    out = []
    for name in admission_listdir(base, "enrollments_unlistable", "enrollments/", missing_ok=True):
        if name.startswith("."):
            continue
        path = os.path.join(base, name)
        st = admission_stat(path, "enrollments_unlistable", f"enrollments/{name}", dangling_absent=False)
        if st is None:
            raise AdmissionUnknown("enrollments_unlistable", f"enrollments/{name} vanished while listing")
        if stat.S_ISDIR(st.st_mode):
            out.append(path)
    return out


def admission_episode_listing(d):
    """The record files of one episode directory and whether SHA256SUMS exists, as record_files() and
    os.path.exists() see them, but with any listing or stat error unknown instead of an absent file.

    Names the decision consumes (enrollment, review-first, close, withdraw, repair-*.json) must be regular
    files: a dangling link, a directory, a FIFO or a vanished entry of such a name is unknown, where the old
    predicates read it as absent (or, for a FIFO, blocked). Other entries are not consumed and are skipped
    as record_files() skips them.
    """
    name = os.path.basename(d)
    code = "episode_unlistable"
    files = []
    for n in admission_listdir(d, code, f"{name}/", missing_ok=False):
        if n == SUMS or n.startswith("."):
            continue
        special = bool(SPECIAL_RECORD.fullmatch(n) or fnmatch.fnmatchcase(n, REPAIR_GLOB))
        st = admission_stat(os.path.join(d, n), code, f"{name}/{n}", dangling_absent=not special)
        if st is None:
            if special:
                raise AdmissionUnknown(code, f"{name}/{n} vanished while listing")
            continue
        if stat.S_ISREG(st.st_mode):
            files.append(n)
        elif special:
            raise AdmissionUnknown(code, f"{name}/{n} is not a regular file")
    sums = admission_stat(os.path.join(d, SUMS), code, f"{name}/{SUMS}", dangling_absent=False)
    if sums is not None and not stat.S_ISREG(sums.st_mode):
        raise AdmissionUnknown("episode_manifest_unreadable", f"{name}/{SUMS} is not a regular file")
    return files, sums is not None


def admission_pause_coverage(run_dir):
    """The triggers acknowledged by pause history: (covered P1 episodes, covered repair rounds). Read-only.

    The strict equivalent of pause_coverage: the same sets on valid input, from one read of each file, and
    unknown (never a guess) where a file cannot be listed, opened or decoded, or is not what the rule needs.
    pause_coverage applies set.update and max to whatever the files hold, so a parseable but malformed
    document can cover a trigger it never covered (a dict of falses, a boolean, a float). Valid is: an
    object; covered_triggers, when present, an object; p1_episodes, when present, a list of strings;
    terra_repair_rounds, when present, an integer, not a boolean, not negative. Absent keys keep
    pause_coverage's defaults; duplicate keys, NaN and Infinity are refused.
    """
    base = os.path.join(run_dir, "pauses")
    covered_p1, covered_repairs = set(), -1
    for name in admission_listdir(base, "pause_history_unlistable", "pauses/", missing_ok=True):
        if name.startswith(".") or not name.endswith(".json"):
            continue  # pause_coverage's glob("*.json") does not see these either.
        label = f"pauses/{name}"
        code = "trigger_inputs_unreadable"
        document = admission_parse_json(admission_read_bytes(os.path.join(base, name), code, label), code, label,
                                        finite=True)

        def malformed(why):
            return AdmissionUnknown("pause_history_malformed", f"{label}: {why}")

        if not isinstance(document, dict):
            raise malformed("the document is not a JSON object")
        if "covered_triggers" not in document:
            continue
        covered = document["covered_triggers"]
        if not isinstance(covered, dict):
            raise malformed("covered_triggers is not an object")
        episodes = covered.get("p1_episodes", [])
        if not isinstance(episodes, list) or any(not isinstance(item, str) for item in episodes):
            raise malformed("covered_triggers.p1_episodes is not a list of strings")
        rounds = covered.get("terra_repair_rounds", -1)
        if type(rounds) is not int or (rounds < 0 and "terra_repair_rounds" in covered):
            raise malformed("covered_triggers.terra_repair_rounds is not a nonnegative integer")
        covered_p1.update(episodes)
        covered_repairs = max(covered_repairs, rounds)
    return covered_p1, covered_repairs


def admission_episode_records(d, files, manifest):
    """The sealed JSON records of one episode, each read once, checked against the manifest bytes.

    A strict equivalent of integrity_problems for the gate: the listing came from admission_episode_listing,
    so every name is a regular file; each is read through the non-blocking checked open; the manifest must
    list exactly those files, once each, with the digests of the bytes just read (and in the line shape
    repair_manifest_tail accepts); every .json record must carry a matching content_sha256 and backward links
    (prior_files) that match. Returns {name: record} for the .json files.
    """
    name = os.path.basename(d)
    code = "episode_integrity_problem"
    raw = {n: admission_read_bytes(os.path.join(d, n), code, f"{name}/{n}") for n in files}
    digests = {n: hashlib.sha256(b).hexdigest() for n, b in raw.items()}
    listed_digests = {}
    # Mirror repair_manifest_tail's per-line test on the raw bytes. listed() splits text with
    # str.splitlines(), which also breaks on \r, \f and U+2028; repair_manifest_tail splits bytes on \n, \r
    # and \r\n only and raises on any line that is not "<hash>  <name>" for an existing, matching record. A
    # manifest the first accepts and the second rejects must not be admitted.
    for line in manifest.splitlines(keepends=True):
        match = re.fullmatch(rb"([0-9a-f]{64})  ([^\r\n]+)\n?", line)
        try:
            entry = match.group(2).decode("utf-8") if match else None
        except UnicodeDecodeError:
            entry = None  # repair_manifest_tail raises on an undecodable name as well.
        if entry not in digests or digests[entry] != match.group(1).decode():
            raise AdmissionUnknown("episode_manifest_needs_repair",
                                   f"{name}: {SUMS} has a line that repair_manifest_tail rejects")
        if entry in listed_digests:
            raise AdmissionUnknown(code, f"{name}: {SUMS} lists {entry} more than once")
        listed_digests[entry] = digests[entry]
    unlisted = sorted(set(files) - set(listed_digests))
    if unlisted:
        raise AdmissionUnknown(code, f"{name}: unlisted file: {', '.join(unlisted)}")
    records = {n: admission_record(raw[n], code, f"{name}/{n}") for n in files if n.endswith(".json")}
    for n, data in records.items():
        prior = data.get("prior_files", {})
        if not isinstance(prior, dict):
            raise AdmissionUnknown(code, f"{name}/{n}: prior_files is not an object")
        for prior_name, digest in prior.items():
            if digests.get(prior_name) != digest:
                raise AdmissionUnknown(code, f"{name}/{n} recorded {prior_name} with a different hash")
    return records


def read_episodes_unrepaired(run_dir):
    """List enrollments as enrollments() does, but refuse where recover_episode would repair. Read-only.

    Returns (directory, enrollment record, record file names, {name: sealed record}) per episode.
    """
    episodes = []
    for d in admission_enrollment_dirs(run_dir):
        name = os.path.basename(d)
        # Strict listing: an unexaminable record or manifest is unknown, not an absent file (a directory that
        # lists but cannot be searched would otherwise look empty and be skipped).
        files, has_manifest = admission_episode_listing(d)
        # Read the manifest bytes as repair_manifest_tail does (same existence test), without repairing.
        manifest = b""
        if has_manifest:
            manifest = admission_read_bytes(os.path.join(d, SUMS), "episode_manifest_unreadable", f"{name}/{SUMS}")
        if not files:
            if manifest:
                # recover_episode raises on a complete entry for a file that is gone, and archives a torn tail.
                raise AdmissionUnknown("episode_manifest_without_records",
                                       f"{name}: {SUMS} is not empty but the directory holds no record files")
            continue  # Nothing listed and nothing recorded: enrollments() neither writes nor raises.
        problems = integrity_problems(d)
        if problems:
            raise AdmissionUnknown("episode_integrity_problem", f"{name}: " + "; ".join(problems))
        if manifest and not manifest.endswith(b"\n"):
            # repair_manifest_tail treats a final line with no newline as torn and rewrites the manifest,
            # even when that line parses and verifies.
            raise AdmissionUnknown("episode_manifest_needs_repair", f"{name}: {SUMS} does not end at a line boundary")
        records = admission_episode_records(d, files, manifest)
        if "enrollment.json" not in files:
            raise AdmissionUnknown("episode_without_enrollment", f"{name}: records exist but enrollment.json does not")
        record = records["enrollment.json"]
        if record.get("arm") not in (TERRA, SOL) or not isinstance(record.get("task_id"), str):
            raise AdmissionUnknown("enrollment_record_invalid", f"{name}: enrollment lacks a known arm and a task_id")
        if record["task_id"] != name:
            raise AdmissionUnknown("enrollment_record_invalid",
                                   f"{name}: enrollment names task {record['task_id']!r}, not its directory")
        episodes.append((d, record, files, records))
    return episodes


def admission_trigger_facts(episodes):
    """episode_trigger_facts for the gate: the same facts from the strictly read episodes, or unknown.

    P1 episodes: Terra episodes whose first review has p1 > 0, where p1 must be an int (not a bool), not
    negative, and the review must name the episode's task. Repair rounds: the repair-*.json records of Terra
    episodes (the shared rule counts the glob), each of which must be named as the recorder names them,
    carry the number in its name as round, name the episode's task, and share its number with no other;
    an episode's numbers must be exactly 1 through n, as cmd_repair writes them.
    """
    p1 = []
    repairs = 0
    for d, e, files, records in episodes:
        if e["arm"] != TERRA:
            continue
        name = os.path.basename(d)
        task = e["task_id"]
        review = records.get("review-first.json")
        if review is not None:
            count = review.get("p1")
            if type(count) is not int or count < 0:
                raise AdmissionUnknown("review_record_invalid",
                                       f"{name}/review-first.json: p1 is {count!r}, not a nonnegative integer")
            if review.get("task_id") != task:
                raise AdmissionUnknown("review_record_invalid", f"{name}/review-first.json names another task")
            if count > 0:
                p1.append(task)
        numbers = set()
        for n in files:
            if not fnmatch.fnmatchcase(n, REPAIR_GLOB):
                continue
            match = REPAIR_NAME.fullmatch(n)
            number = int(match.group(1)) if match else None
            round_field = records[n].get("round")
            if (match is None or number in numbers or type(round_field) is not int or round_field != number
                    or records[n].get("task_id") != task):
                raise AdmissionUnknown("repair_record_invalid",
                                       f"{name}/{n}: not a repair record numbered as named for this episode")
            numbers.add(number)
            repairs += 1
        if numbers != set(range(1, len(numbers) + 1)):
            # cmd_repair numbers a record as the highest existing number plus one, so a record numbered n asserts
            # rounds 1..n-1: a skipped number is missing or inconsistent evidence, and the round total is understated.
            raise AdmissionUnknown("repair_record_invalid",
                                   f"{name}: repair records are numbered {sorted(numbers)}, not 1 through {len(numbers)}")
    return {"p1_episodes": sorted(p1), "terra_repair_rounds": repairs}


def admission_decision(run_dir):
    facts = {key: {"evaluated": False} for key in ("pause_marker", "round", "enrollments", "trigger", "terra_in_flight")}

    def outcome(state, code, text):
        return {"state": state, "reason": {"code": code, "text": text}, "run_dir": run_dir, "facts": facts}

    try:
        run_stat = admission_stat(run_dir, "run_dir_unreadable", "run directory", dangling_absent=True)
        if run_stat is None or not stat.S_ISDIR(run_stat.st_mode):
            return outcome("unknown", "run_dir_missing", "run directory is missing or not a directory")
        marker = os.path.join(run_dir, "PAUSED")
        try:
            os.lstat(marker)
            marker_present = True
        except FileNotFoundError:
            marker_present = False
        except OSError as e:  # os.path.lexists would call this absent. It may be the marker.
            raise AdmissionUnknown("pause_marker_unreadable", f"PAUSED cannot be examined: {e}") from e
        if marker_present:
            facts["pause_marker"] = {"evaluated": True, **pause_marker_fact(marker)}
            return outcome("paused", "pause_marker_present", "a PAUSED entry exists; only the owner resumes the round")
        facts["pause_marker"] = {"evaluated": True, "present": False}
        config = read_round_config(run_dir)
        facts["round"] = {"evaluated": True, **config}
        episodes = read_episodes_unrepaired(run_dir)
        open_terra = sum(e["arm"] == TERRA and not {"close.json", "withdraw.json"} & set(files)
                         for _, e, files, _ in episodes)
        facts["enrollments"] = {"evaluated": True, "count": len(episodes), "cap": config["cap"]}
        facts["terra_in_flight"] = {"evaluated": True, "open_terra": open_terra, "limit": config["terra_in_flight"],
                                    "limit_reached": open_terra >= config["terra_in_flight"], "gates_admission": False}
        try:
            # Strict equivalents of pause_coverage and episode_trigger_facts: malformed coverage must not
            # cover, and an unreadable or invalid record must not read as "no P1" or "no repair".
            covered_p1, covered_repairs = admission_pause_coverage(run_dir)
            trigger = admission_trigger_facts(episodes)
        except (IntegrityFailure, OSError, ValueError, KeyError, TypeError, AttributeError) as e:
            raise AdmissionUnknown("trigger_inputs_unreadable", f"review-first.json or pause history is unreadable: {e}") from e
        new_p1, over_budget = terra_stop_rule(trigger, covered_p1, covered_repairs, config["terra_repair_budget"])
        facts["trigger"] = {"evaluated": True, **trigger, "covered_p1_episodes": sorted(covered_p1),
                            "covered_repair_rounds": covered_repairs, "uncovered_p1_episodes": new_p1,
                            "repair_rounds_over_budget": over_budget}
        if new_p1 or over_budget:
            return outcome("stop_reached", "pause_trigger_unmarked",
                           f"Terra trigger met with no PAUSED marker: uncovered P1 episodes {new_p1}; repair rounds "
                           f"{trigger['terra_repair_rounds']} (budget {config['terra_repair_budget']}, covered {covered_repairs})")
        if len(episodes) >= config["cap"]:
            return outcome("stop_reached", "cap_reached", f"{len(episodes)} enrollments at cap {config['cap']}")
        return outcome("active", "admitted", "no marker, trigger or cap stops screening")
    except AdmissionUnknown as e:
        return outcome("unknown", e.code, e.text)
    except Exception as e:  # An unexpected failure is never an admission.
        return outcome("unknown", "unexpected_exception", f"{type(e).__name__}: {e}")


def cmd_admission(a):
    out = admission_decision(a.run_dir)
    print(json.dumps(out, sort_keys=True))
    return ADMISSION_EXIT[out["state"]]


@contextlib.contextmanager
def round_lock(run_dir):
    with open(os.path.join(run_dir, ".round.lock"), "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def run_round_command(a):
    RELISTING_REPORTS.clear()
    try:
        if a.cmd != "init-round" and not os.path.isfile(os.path.join(a.run_dir, "round.json")):
            raise UsageRefusal("round.json is required; run init-round first")
        os.makedirs(a.run_dir, exist_ok=True)
        with round_lock(a.run_dir):
            if a.cmd == "assign":
                if a.episode:
                    a.task_id = a.episode
                    _, enrollment = active_episode(a, allow_terminal=True)
                    a.task_revision = enrollment["task_revision"]
                task_id = a.task_revision.split("@")[0]
                d = episode_dir(a.run_dir, task_id)
                if os.path.isdir(d):
                    recover_episode(d)
                    if os.path.exists(os.path.join(d, "withdraw.json")):
                        raise UsageRefusal("cannot assign a withdrawn task")
                out = assign_record(a)
            else:
                out = globals()["cmd_" + a.cmd.replace("-", "_")](a)
        if RELISTING_REPORTS:
            out["manifest_relistings"] = RELISTING_REPORTS
        print(json.dumps(out, sort_keys=True))
        if out.get("outcome") == "profile_required":
            raise SystemExit(4)
    except UsageRefusal as e:
        print(json.dumps({"error": str(e), "outcome": "refused", "manifest_relistings": RELISTING_REPORTS}))
        raise SystemExit(2)
    except (IntegrityFailure, OSError, ValueError, KeyError, TypeError) as e:
        print(json.dumps({"error": str(e), "outcome": "integrity_failure", "manifest_relistings": RELISTING_REPORTS}))
        raise SystemExit(1)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    default_tool = os.path.join(TOOLS_ROOT, PIN["commit"])
    sub.add_parser("start", help="print the current time (UTC, microseconds); pass it as --declare-started-at")
    c = sub.add_parser("check-tool")
    c.add_argument("--tool-dir", default=default_tool)
    p = sub.add_parser("profile")
    p.add_argument("--run-dir", required=True)
    p.add_argument("--sidecar", required=True)
    p.add_argument("--repo", required=True)
    p.add_argument("--declare-started-at", required=True)
    p.add_argument("--call-id", action="append")
    p.add_argument("--tool-dir", default=default_tool)
    s = sub.add_parser("assign")
    s.add_argument("--run-dir", required=True)
    assignment_source = s.add_mutually_exclusive_group(required=True)
    assignment_source.add_argument("--task-revision")
    assignment_source.add_argument("--episode", help="append a dispatch using an enrolled task's frozen profile")
    s.add_argument("--role", choices=("builder", "reviewer"), required=True)
    s.add_argument("--requested-model", required=True)
    s.add_argument("--requested-effort", required=True)
    s.add_argument("--harness", required=True)
    s.add_argument("--reported-model")
    s.add_argument("--reported-effort")
    s.add_argument("--run", required=True)
    s.add_argument("--dispatched-packet", required=True)
    s.add_argument("--child-session", action="append", required=True)
    s.add_argument("--selected-at")
    s.add_argument("--retained", action="store_true")
    t = sub.add_parser("attribute")
    t.add_argument("--run-dir", required=True)
    attribution_source = t.add_mutually_exclusive_group(required=True)
    attribution_source.add_argument("--task-revision")
    attribution_source.add_argument("--all", action="store_true", help="attribute every assignment with child sessions and no attribution yet")
    t.add_argument("--ledger", required=True)
    t.add_argument("--assignment")
    t.add_argument("--since")
    t.add_argument("--until")
    v = sub.add_parser("verify")
    v.add_argument("--run-dir", required=True)
    v.add_argument("--repo")
    v.add_argument("--no-recompute", action="store_true")
    v.add_argument("--tools-root", default=TOOLS_ROOT)
    ad = sub.add_parser("admission", help="read-only: may a supervisor screen? exit 0 only when active",
                        description=ADMISSION_EPILOG, formatter_class=argparse.RawDescriptionHelpFormatter)
    ad.add_argument("--run-dir", required=True)
    for name in sorted(ROUND_COMMANDS):
        r = sub.add_parser(name)
        r.add_argument("--run-dir", required=True)
        if name in {"enroll", "withdraw", "review", "repair", "close", "status"}:
            r.add_argument("--task-id", required=name != "status")
        if name in {"enroll", "withdraw", "close", "pause"}:
            r.add_argument("--reason", required=True)
        if name == "init-round":
            for flag in ("--cap", "--terra-in-flight", "--terra-repair-budget"):
                r.add_argument(flag, type=nonnegative, required=True)
        elif name == "enroll":
            source = r.add_mutually_exclusive_group(required=True)
            source.add_argument("--task-revision")
            source.add_argument("--packet", help="screen a frozen packet without a profile; exit 4 (profile_required) writes no screening or enrollment")
            r.add_argument("--eligible", choices=("yes", "no"), required=True)
        elif name == "review":
            r.add_argument("--verdict", choices=("accept", "changes_required", "blocked_evidence", "blocked_scope"), required=True)
            r.add_argument("--reviewer-session", required=True)
            for flag in ("--p1", "--p2", "--p3"):
                r.add_argument(flag, type=nonnegative, required=True)
            r.add_argument("--receipt", required=True)
        elif name == "repair":
            r.add_argument("--review-receipt", required=True)
        elif name == "close":
            r.add_argument("--disposition", choices=("accepted", "abandoned"), required=True)
            r.add_argument("--pre-review-fixes", type=nonnegative, required=True)
            r.add_argument("--escalations", type=nonnegative, required=True)
            r.add_argument("--review-receipt", action="append")
            r.add_argument("--artifact", action="append")
        elif name == "resume":
            r.add_argument("--owner-approval", required=True)
    a = ap.parse_args()
    if a.cmd == "attribute" and a.all and a.assignment:
        ap.error("--assignment cannot be combined with --all")
    if a.cmd in ROUND_COMMANDS:
        run_round_command(a)
    elif a.cmd == "admission":
        sys.exit(cmd_admission(a))
    elif a.cmd == "start":
        print(now())
    elif a.cmd == "check-tool":
        print(f"ok: {tool(a.tool_dir, PIN['commit'])} at {PIN['commit']}")
    else:
        {"profile": cmd_profile, "assign": cmd_assign, "attribute": cmd_attribute, "verify": cmd_verify}[a.cmd](a)


if __name__ == "__main__":
    main()
