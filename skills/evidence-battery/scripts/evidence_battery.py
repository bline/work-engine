#!/usr/bin/env python3
"""Run a declared battery of read-only checks in one process and one call."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any

READ_ONLY_GIT = {
    "rev-parse", "status", "ls-files", "diff", "log", "show", "cat-file",
    "merge-base", "describe", "rev-list", "for-each-ref", "ls-tree",
}
FORBIDDEN_GIT_ARGS = re.compile(r"^(--output|--exec|--upload-pack|-c)")
IDENTITY = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*")
DEFAULT_TEXT_LIMIT = 4000
DEFAULT_MATCH_LIMIT = 20


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def resolve(roots: list[Path], relative: str) -> Path:
    """Resolve the parent only: a final-component symlink must stay observable."""
    if not isinstance(relative, str) or not relative:
        fail("every path must be a nonempty string")
    candidate = Path(relative)
    if not candidate.is_absolute():
        candidate = roots[0] / candidate
    resolved = candidate.parent.resolve() / candidate.name
    if not any(resolved == root or resolved.is_relative_to(root) for root in roots):
        fail(f"path escapes every declared root: {relative}")
    return resolved


def entry(roots: list[Path], relative: str) -> dict[str, Any]:
    """The same manifest shape the source-snapshot evidence already uses."""
    path = resolve(roots, relative)
    try:
        info = path.lstat()
    except FileNotFoundError:
        return {"path": relative, "type": "absent"}
    if stat.S_ISLNK(info.st_mode):
        target = os.readlink(path)
        payload, kind = os.fsencode(target), "symlink"
    elif stat.S_ISREG(info.st_mode):
        payload, kind = path.read_bytes(), "file"
    elif stat.S_ISDIR(info.st_mode):
        return {"path": relative, "type": "directory", "mode": stat.S_IMODE(info.st_mode)}
    else:
        fail(f"unsupported path type: {relative}")
    return {
        "path": relative, "type": kind, "mode": stat.S_IMODE(info.st_mode),
        "length": len(payload), "sha256": digest(payload),
        **({"link_target": target} if kind == "symlink" else {}),
    }


def check_sha256(roots: list[Path], spec: Any) -> dict[str, Any]:
    if not isinstance(spec, list):
        fail("sha256 takes a list of paths")
    entries = [entry(roots, name) for name in spec]
    return {"path_count": len(entries), "entries_sha256": digest(canonical(entries)),
            "entries": entries}


def check_aggregate(roots: list[Path], spec: Any) -> dict[str, Any]:
    """One digest over a path set, for `find | xargs sha256sum | sha256sum`."""
    result = check_sha256(roots, spec)
    present = [item for item in result["entries"] if item["type"] != "absent"]
    return {"path_count": result["path_count"], "present_count": len(present),
            "entries_sha256": result["entries_sha256"],
            "absent": [item["path"] for item in result["entries"] if item["type"] == "absent"]}


def check_git(roots: list[Path], spec: Any, text_limit: int) -> dict[str, Any]:
    if not isinstance(spec, list) or not spec:
        fail("git takes a nonempty argument list")
    if spec[0] not in READ_ONLY_GIT:
        fail(f"git {spec[0]} is not a declared read-only subcommand")
    for argument in spec:
        if not isinstance(argument, str) or FORBIDDEN_GIT_ARGS.match(argument):
            fail(f"git argument not permitted here: {argument}")
    complete = subprocess.run(
        ["git", "-C", str(roots[0]), *spec],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    raw = complete.stdout
    text = raw.decode(errors="replace")
    return {"returncode": complete.returncode, "output_sha256": digest(raw),
            "byte_count": len(raw), "line_count": text.count("\n"),
            "text": text[:text_limit], "truncated": len(text) > text_limit,
            **({"stderr": complete.stderr.decode(errors="replace")[:text_limit]}
               if complete.returncode else {})}


def check_json(roots: list[Path], spec: Any) -> dict[str, Any]:
    if not isinstance(spec, dict) or "path" not in spec:
        fail("json takes {path, select?}")
    path = resolve(roots, spec["path"])
    raw = path.read_bytes()
    document = json.loads(raw)
    selected: dict[str, Any] = {}
    for key in spec.get("select") or []:
        cursor: Any = document
        for part in str(key).split("."):
            if isinstance(cursor, dict) and part in cursor:
                cursor = cursor[part]
            elif isinstance(cursor, list) and part.isdigit() and int(part) < len(cursor):
                cursor = cursor[int(part)]
            else:
                cursor = None
                break
        selected[key] = cursor
    return {"sha256": digest(raw), "byte_count": len(raw),
            **({"selected": selected} if spec.get("select") else {"keys": sorted(document)
               if isinstance(document, dict) else None})}


def check_grep(roots: list[Path], spec: Any, match_limit: int) -> dict[str, Any]:
    if not isinstance(spec, dict) or "pattern" not in spec or "paths" not in spec:
        fail("grep takes {pattern, paths, ignore_case?}")
    flags = re.IGNORECASE if spec.get("ignore_case") else 0
    pattern = re.compile(spec["pattern"], flags)
    matches: list[dict[str, Any]] = []
    total = 0
    for name in spec["paths"]:
        path = resolve(roots, name)
        try:
            text = path.read_text(errors="replace")
        except (FileNotFoundError, IsADirectoryError):
            continue
        for number, line in enumerate(text.splitlines(), 1):
            if not pattern.search(line):
                continue
            total += 1
            if len(matches) < match_limit:
                matches.append({"path": name, "line": number, "text": line.strip()[:200]})
    return {"match_count": total, "matches": matches,
            "truncated": total > len(matches)}


def run(plan: dict[str, Any]) -> dict[str, Any]:
    roots = [Path(plan.get("root") or ".").resolve()]
    for extra in plan.get("extra_roots") or []:
        roots.append(Path(extra).resolve())
    text_limit = int(plan.get("text_limit") or DEFAULT_TEXT_LIMIT)
    match_limit = int(plan.get("match_limit") or DEFAULT_MATCH_LIMIT)
    checks = plan.get("checks")
    if not isinstance(checks, list) or not checks:
        fail("a battery needs a nonempty checks list")
    results: dict[str, Any] = {}
    failed: list[str] = []
    for position, check in enumerate(checks):
        if not isinstance(check, dict):
            fail(f"checks[{position}] must be an object")
        name = check.get("id") or f"check{position}"
        if not IDENTITY.fullmatch(str(name)) or name in results:
            fail(f"checks[{position}] needs a unique safe id")
        kinds = [key for key in ("sha256", "aggregate", "git", "json", "grep") if key in check]
        if len(kinds) != 1:
            fail(f"{name} must declare exactly one of sha256, aggregate, git, json, grep")
        kind = kinds[0]
        try:
            if kind == "sha256":
                value = check_sha256(roots, check[kind])
            elif kind == "aggregate":
                value = check_aggregate(roots, check[kind])
            elif kind == "git":
                value = check_git(roots, check[kind], text_limit)
            elif kind == "json":
                value = check_json(roots, check[kind])
            else:
                value = check_grep(roots, check[kind], match_limit)
            value["kind"] = kind
        except Exception as error:                    # one bad check must not lose the rest
            value = {"kind": kind, "error": f"{type(error).__name__}: {error}"}
            failed.append(str(name))
        results[str(name)] = value
    return {"check_count": len(results), "failed": failed,
            "battery_sha256": digest(canonical(results)), "checks": results}


def compare(current: dict[str, Any], previous: dict[str, Any]) -> dict[str, Any]:
    """Report only what moved, so an unchanged battery costs a few tokens."""
    if current["battery_sha256"] == previous.get("battery_sha256"):
        return {"status": "unchanged", "battery_sha256": current["battery_sha256"],
                "check_count": current["check_count"]}
    before, after = previous.get("checks") or {}, current["checks"]
    changed = {
        name: {"before": before.get(name), "after": value}
        for name, value in after.items() if before.get(name) != value
    }
    return {"status": "changed", "battery_sha256": current["battery_sha256"],
            "changed_ids": sorted(changed), "added_ids": sorted(set(after) - set(before)),
            "removed_ids": sorted(set(before) - set(after)), "changed": changed}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", default="-", help="battery plan JSON, or - for stdin")
    parser.add_argument("--compare", type=Path, help="prior result to diff against")
    parser.add_argument("--save", type=Path, help="write the full result here")
    parser.add_argument("--quiet", action="store_true", help="print only the digest and failures")
    arguments = parser.parse_args()
    raw = sys.stdin.read() if arguments.plan == "-" else Path(arguments.plan).read_text()
    result = run(json.loads(raw))
    if arguments.save:
        with arguments.save.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, sort_keys=True, indent=2)
            stream.write("\n")
    if arguments.compare:
        printed = compare(result, json.loads(arguments.compare.read_text()))
    elif arguments.quiet:
        printed = {"battery_sha256": result["battery_sha256"],
                   "check_count": result["check_count"], "failed": result["failed"]}
    else:
        printed = result
    print(json.dumps(printed, sort_keys=True, indent=2))
    return 1 if result["failed"] else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"evidence battery failed: {error}", file=sys.stderr)
        raise SystemExit(2)
