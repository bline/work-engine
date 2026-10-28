#!/usr/bin/env python3
"""Keep command-line synopses in markdown generated from the scripts' own --help.

Agents lose whole model calls looking up ``--help``. A marked block puts the
usage lines in the file the agent already reads, and ``check`` fails when a
script's arguments change without the block being regenerated.

Marker syntax (two whole lines; everything between them is generated)::

    <!-- cli-synopsis script=PATH [subs="a b c"] [display="COMMAND"] -->
    ...generated, replaced by ``write``...
    <!-- /cli-synopsis -->

``script``   repository-relative path of an argparse script; it is run from the
             repository root with ``--help`` (and ``<sub> --help``).
``subs``     subcommands to include, in this order, separated by spaces or
             commas. Default: all of them. Ignored for scripts without
             subcommands, where the single top-level usage line is shown.
             When the list is a subset, one line names the other subcommands.
``display``  the invocation shown in front of every line, so the displayed form
             follows the file's path convention (``python3 <skill-dir>/scripts/x.py``
             in a skill, a repository-root path in AGENTS.md). Default:
             ``python3 PATH``.

Values containing spaces are double-quoted. Global options appear once, in a
leading comment line, unless every one is already spelled out in ``display``; each subcommand gets one usage line with ``[-h]`` removed.
Markers inside fenced code blocks are ignored, so this syntax can be documented.

``cli_synopsis.py check`` exits 1 and names every stale or malformed block;
``cli_synopsis.py write`` regenerates them. Standard library only.
"""

from __future__ import annotations

import argparse
import os
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

MARKER = "cli-synopsis"
BEGIN_RE = re.compile(r"^<!-- cli-synopsis (.*) -->$")
END_LINE = "<!-- /cli-synopsis -->"
KEYS = ("script", "subs", "display")
FENCE_RE = re.compile(r"^\s*(`{3,}|~{3,})")
SUB_LIST_RE = re.compile(r"\{([^{}]+)\}")
HELP_TIMEOUT_SECONDS = 60
HELP_ENV = {"COLUMNS": "10000", "LC_ALL": "C.UTF-8", "NO_COLOR": "1",
            "PYTHONDONTWRITEBYTECODE": "1"}


class SynopsisError(Exception):
    """A block could not be parsed or its script's usage could not be read."""


@dataclass
class Block:
    path: str
    begin: int  # 0-based index of the begin marker line
    end: int  # 0-based index of the end marker line
    script: str
    subs: tuple[str, ...] | None
    display: str

    @property
    def where(self) -> str:
        return f"{self.path}:{self.begin + 1}"

    @property
    def name(self) -> str:
        subs = f" subs={' '.join(self.subs)}" if self.subs is not None else ""
        return f"{self.script}{subs}"


# --- parsing --------------------------------------------------------------------

def parse_marker(interior: str) -> dict[str, str]:
    try:
        words = shlex.split(interior)
    except ValueError as error:
        raise SynopsisError(f"unparseable marker ({error})") from None
    values: dict[str, str] = {}
    for word in words:
        key, equals, value = word.partition("=")
        if not equals or not value:
            raise SynopsisError(f"marker word {word!r} is not key=value")
        if key not in KEYS:
            raise SynopsisError(f"unknown marker key {key!r} (known: {', '.join(KEYS)})")
        if key in values:
            raise SynopsisError(f"marker key {key!r} given twice")
        values[key] = value
    if "script" not in values:
        raise SynopsisError("marker has no script=")
    return values


def block_from_marker(path: str, begin: int, end: int, values: dict[str, str]) -> Block:
    script = values["script"]
    parts = Path(script).parts
    if Path(script).is_absolute() or ".." in parts or not parts:
        raise SynopsisError(f"script {script!r} must be a repository-relative path without '..'")
    subs = None
    if "subs" in values:
        subs = tuple(s for s in re.split(r"[\s,]+", values["subs"]) if s)
        if not subs:
            raise SynopsisError("subs= is empty")
        if len(set(subs)) != len(subs):
            raise SynopsisError(f"subs= repeats a subcommand: {' '.join(subs)}")
    return Block(path, begin, end, script, subs, values.get("display", f"python3 {script}"))


def parse_blocks(path: str, lines: list[str]) -> tuple[list[Block], list[str]]:
    """Return the well-formed blocks of a file and a problem string per malformed one."""
    blocks: list[Block] = []
    problems: list[str] = []
    fence: tuple[str, int] | None = None
    open_at: int | None = None
    open_values: dict[str, str] | None = None
    open_error: str | None = None
    for index, line in enumerate(lines):
        at = f"{path}:{index + 1}"
        if open_at is None:
            fenced = FENCE_RE.match(line)
            if fence is not None:
                if fenced and fenced.group(1)[0] == fence[0] and len(fenced.group(1)) >= fence[1] \
                        and line.strip() == fenced.group(1):
                    fence = None
                continue
            if fenced:
                fence = (fenced.group(1)[0], len(fenced.group(1)))
                continue
        if line.strip() == END_LINE and open_at is None:
            problems.append(f"{at}: end marker without a begin marker")
            continue
        if line.startswith(f"<!-- {MARKER}") or (open_at is not None and line.startswith(f"<!-- /{MARKER}")):
            if line.strip() == END_LINE:
                if open_error:
                    problems.append(f"{path}:{open_at + 1}: {open_error}")
                else:
                    try:
                        blocks.append(block_from_marker(path, open_at, index, open_values))
                    except SynopsisError as error:
                        problems.append(f"{path}:{open_at + 1}: {error}")
                open_at = None
                continue
            if open_at is not None:
                kind = "malformed end marker" if line.startswith(f"<!-- /{MARKER}") else "begin marker"
                problems.append(f"{at}: {kind} inside the block begun at line {open_at + 1}")
                open_error = open_error or "block is not closed cleanly"
                continue
            match = BEGIN_RE.match(line)
            open_at, open_values, open_error = index, {}, None
            if not match:
                open_error = "malformed begin marker (expected one line '<!-- cli-synopsis key=value ... -->')"
                continue
            try:
                open_values = parse_marker(match.group(1))
            except SynopsisError as error:
                open_error = str(error)
    if open_at is not None:
        problems.append(f"{path}:{open_at + 1}: begin marker without an end marker")
    return blocks, problems


# --- reading --help -------------------------------------------------------------

class HelpReader:
    """Runs scripts from the repository root and caches their usage lines for one pass."""

    def __init__(self, root: Path):
        self.root = root
        self.cache: dict[tuple[str, tuple[str, ...]], str] = {}

    def usage(self, script: str, *sub: str) -> str:
        key = (script, sub)
        if key not in self.cache:
            self.cache[key] = self._read(script, *sub)
        return self.cache[key]

    def _read(self, script: str, *sub: str) -> str:
        target = self.root / script
        if not target.is_file():
            raise SynopsisError(f"script {script} does not exist")
        env = {k: v for k, v in os.environ.items() if k != "FORCE_COLOR"}
        env.update(HELP_ENV)
        argv = [sys.executable, script, *sub, "--help"]
        try:
            result = subprocess.run(argv, cwd=self.root, env=env, capture_output=True, text=True,
                                    check=False, timeout=HELP_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            raise SynopsisError(f"`{' '.join(argv[1:])}` did not finish within {HELP_TIMEOUT_SECONDS} s")
        if result.returncode != 0:
            detail = result.stderr.strip().splitlines()[-1:] or ["no stderr"]
            raise SynopsisError(f"`{' '.join(argv[1:])}` exited {result.returncode}: {detail[0]}")
        collected: list[str] = []
        for line in result.stdout.splitlines():
            if not collected:
                if line.startswith("usage:"):
                    collected.append(line)
            elif line.strip():
                collected.append(line)
            else:
                break
        if not collected:
            raise SynopsisError(f"`{' '.join(argv[1:])}` printed no usage line")
        return " ".join(" ".join(collected).split())


def strip_help_flag(text: str) -> str:
    return " ".join(text.replace("[-h]", " ", 1).split())


def split_usage(usage: str, sub: str | None = None) -> tuple[str, str]:
    """Return (program, arguments) from a ``usage: PROG ...`` line; for a subcommand,
    the arguments follow the subcommand name."""
    body = usage[len("usage:"):].strip()
    program, _, rest = body.partition(" ")
    if sub is not None:
        name, _, rest = rest.partition(" ")
        if name != sub:
            raise SynopsisError(f"usage of {sub!r} does not name it: {usage}")
    return program, rest


def render(block: Block, reader: HelpReader) -> list[str]:
    """The generated lines (fence included) that belong between the markers."""
    _, top = split_usage(reader.usage(block.script))
    match = SUB_LIST_RE.search(top)
    out: list[str] = []
    if match is None:
        if block.subs is not None:
            raise SynopsisError(f"{block.script} has no subcommands but subs= was given")
        out.append(f"{block.display} {strip_help_flag(top)}".rstrip())
    else:
        known = match.group(1).split(",")
        chosen = list(block.subs) if block.subs is not None else known
        unknown = [s for s in chosen if s not in known]
        if unknown:
            raise SynopsisError(f"{block.script} has no subcommand {', '.join(unknown)} (has: {', '.join(known)})")
        global_options = strip_help_flag(top[:match.start()])
        flags = re.findall(r"--[\w-]+", global_options)
        if global_options and not all(re.search(rf"(?<![\w-]){re.escape(f)}(?![\w-])", block.display) for f in flags):
            out.append(f"# global options, given before the subcommand: {global_options}")
        for sub in chosen:
            _, rest = split_usage(reader.usage(block.script, sub), sub)
            out.append(f"{block.display} {sub} {strip_help_flag(rest)}".rstrip())
        others = [s for s in known if s not in chosen]
        if others:
            out.append(f"# other subcommands (put --help after one for its usage): {', '.join(others)}")
    return ["```text", *out, "```"]


# --- files ----------------------------------------------------------------------

def repository_root(start: Path) -> Path:
    result = subprocess.run(["git", "-C", str(start), "rev-parse", "--show-toplevel"],
                            capture_output=True, text=True, check=False)
    return Path(result.stdout.strip()) if result.returncode == 0 else start.resolve()


def markdown_files(root: Path) -> list[str]:
    """Tracked markdown files (staged new ones included), repository-relative and sorted.

    Untracked files are left out: in a tree shared with other sessions, their drafts
    would fail the check and have their markers' scripts run. Name a file explicitly
    to check or write it before it is tracked.
    """
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--cached", "--", "*.md"],
        capture_output=True, check=False)
    if result.returncode == 0:
        names = [n for n in result.stdout.decode().split("\0") if n]
    else:
        names = [str(p.relative_to(root)) for p in root.rglob("*.md")]
    return sorted(n for n in set(names) if (root / n).is_file())


@dataclass
class Report:
    problems: list[str]
    stale: list[str]
    changed: list[str]
    blocks: int

    @property
    def ok(self) -> bool:
        return not self.problems and not self.stale


def process(root: Path, names: list[str] | None, write: bool) -> Report:
    report = Report([], [], [], 0)
    reader = HelpReader(root)
    for name in names if names is not None else markdown_files(root):
        text = (root / name).read_text(encoding="utf-8")
        if MARKER not in text:
            continue
        lines = text.split("\n")
        blocks, problems = parse_blocks(name, lines)
        report.problems.extend(problems)
        report.blocks += len(blocks)
        rebuilt = list(lines)
        first_stale = len(report.stale)
        file_ok = not problems
        for block in sorted(blocks, key=lambda b: b.begin, reverse=True):
            try:
                wanted = render(block, reader)
            except SynopsisError as error:
                report.problems.append(f"{block.where}: {block.name}: {error}")
                file_ok = False
                continue
            if lines[block.begin + 1:block.end] != wanted:
                report.stale.append(f"{block.where}: stale synopsis for {block.name}")
                rebuilt[block.begin + 1:block.end] = wanted
        report.stale[first_stale:] = report.stale[first_stale:][::-1]
        if write and file_ok and rebuilt != lines:
            (root / name).write_text("\n".join(rebuilt), encoding="utf-8")
            report.changed.append(name)
    return report


def synopsis_problems(root: Path) -> list[str]:
    """Problems for the working tree; used by ``skill_release.py check``."""
    report = process(root, None, write=False)
    return [*report.problems, *report.stale]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repository", type=Path, default=Path("."))
    sub = parser.add_subparsers(dest="command", required=True)
    for command, text in (("check", "exit 1 and name every stale or malformed block; changes nothing"),
                          ("write", "regenerate stale blocks; refuses files with malformed blocks")):
        p = sub.add_parser(command, help=text)
        p.add_argument("files", nargs="*", help="repository-relative markdown files (default: all)")
    args = parser.parse_args(argv)
    root = repository_root(args.repository.resolve())
    report = process(root, args.files or None, write=args.command == "write")
    for line in report.problems:
        print(f"cli-synopsis: {line}", file=sys.stderr)
    if args.command == "write":
        for name in report.changed:
            print(f"rewrote {name}")
        remaining = len(report.problems)
        print(f"{report.blocks} block(s), {len(report.changed)} file(s) rewritten, {remaining} problem(s)")
        return 0 if not report.problems else 1
    for line in report.stale:
        print(f"cli-synopsis: {line}; run cli_synopsis.py write", file=sys.stderr)
    print(f"{report.blocks} block(s), {len(report.stale)} stale, {len(report.problems)} problem(s)")
    return 0 if report.ok else 1


if __name__ == "__main__":
    sys.exit(main())
