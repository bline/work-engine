#!/usr/bin/env python3
"""Deploy committed skills to agent skill directories as immutable releases.

A release is a read-only export of one commit's ``skills/`` tree at
``<release-root>/releases/<commit>``. ``<release-root>/current`` is a symlink to
the active release and is replaced atomically. Each agent skill-directory entry
named in the install manifest is a symlink to
``<release-root>/current/skills/<name>``, so switching ``current`` switches
every agent at once.
"""

from __future__ import annotations

import argparse
import contextlib
import datetime as dt
import fcntl
import hashlib
import importlib.util
import io
import json
import os
import secrets
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

CATALOG_PATH = "skills/catalog.json"
CATALOG_SCHEMA = "work-engine.skill-catalog.v1"
KINDS = ("workflow", "tool", "research", "other-project")
STATES = ("active", "stable", "dormant", "retired")
CATALOG_FIELDS = {"kind", "state", "install", "install_reason", "not_installed_reason"}
# Install lists before the catalog existed (415ceda2); read only for older commits.
MANIFEST_PATH = "skills/skill-release/install-manifest.json"
MANIFEST_SCHEMA = "work-engine.skill-install-manifest.v1"
RELEASE_SCHEMA = "work-engine.skill-release.v1"
HOME = Path.home()
DEFAULT_RELEASE_ROOT = HOME / ".local/share/work-engine/skill-releases"
DEFAULT_BACKUP_ROOT = HOME / "cbm-work/skill-release-retired"
DEFAULT_TARGETS = {"claude": HOME / ".claude/skills", "codex": HOME / ".codex/skills"}
IGNORED_DIRS = {"__pycache__"}
IGNORED_SUFFIXES = (".pyc",)


class ReleaseError(Exception):
    pass


# --- git and content identity -------------------------------------------------

def git(repo: Path, *args: str, binary: bool = False, env: dict | None = None):
    result = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, check=False,
                            env={**os.environ, **env} if env else None)
    if result.returncode != 0:
        raise ReleaseError(f"git {' '.join(args)}: {result.stderr.decode(errors='replace').strip()}")
    return result.stdout if binary else result.stdout.decode()


def working_tree_skills(repo: Path) -> str:
    """Tree id of the working tree's skills/, untracked files included and ignored ones not.

    Uses a temporary index, so the user's index is untouched; only objects are written.
    """
    with tempfile.TemporaryDirectory() as scratch:
        env = {"GIT_INDEX_FILE": os.path.join(scratch, "index")}
        git(repo, "read-tree", "--empty", env=env)
        git(repo, "add", "-A", "--", "skills", env=env)
        return git(repo, "write-tree", "--prefix=skills/", env=env).strip()


def resolve_commit(repo: Path, ref: str) -> str:
    return git(repo, "rev-parse", "--verify", f"{ref}^{{commit}}").strip()


def blob_oid(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def tree_entries(repo: Path, commit: str, path: str) -> dict[str, tuple[str, str]]:
    """Map repo-relative path -> (mode, blob oid) for every file under path at commit."""
    entries = {}
    for record in filter(None, git(repo, "ls-tree", "-r", "-z", commit, "--", path).split("\0")):
        meta, name = record.split("\t", 1)
        mode, kind, oid = meta.split()
        if kind != "blob":
            raise ReleaseError(f"{name}: unsupported tree entry {kind}")
        entries[name] = (mode, oid)
    return entries


def dir_entries(root: Path, prefix: str) -> dict[str, tuple[str, str]]:
    """Map prefix-relative path -> (mode, blob oid) for files under a directory, as git would."""
    entries = {}
    for current, dirs, files in os.walk(root, followlinks=False):
        links = [d for d in dirs if os.path.islink(os.path.join(current, d))]
        dirs[:] = sorted(d for d in dirs if d not in IGNORED_DIRS and d not in links)
        for name in files + links:
            if name.endswith(IGNORED_SUFFIXES):
                continue
            full = os.path.join(current, name)
            rel = f"{prefix}/{os.path.relpath(full, root)}"
            if os.path.islink(full):
                entries[rel] = ("120000", blob_oid(os.fsencode(os.readlink(full))))
            else:
                mode = "100755" if os.stat(full).st_mode & stat.S_IXUSR else "100644"
                entries[rel] = (mode, blob_oid(Path(full).read_bytes()))
    return entries


def compare(expected: dict, actual: dict) -> dict[str, list[str]]:
    return {
        "differ": sorted(p for p in expected.keys() & actual.keys() if expected[p] != actual[p]),
        "missing": sorted(expected.keys() - actual.keys()),
        "extra": sorted(actual.keys() - expected.keys()),
    }


def summary(problems: dict[str, list[str]]) -> str:
    return ", ".join(f"{len(v)} {k}" for k, v in problems.items() if v) or "identical"


def frontmatter(text: str) -> str:
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return ""
    for index, line in enumerate(lines[1:], 1):
        if line.strip() == "---":
            return "\n".join(lines[1:index])
    return ""


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


# --- manifest -----------------------------------------------------------------

def validate_manifest(manifest, targets: dict[str, Path], has_skill) -> dict[str, list[str]]:
    if not isinstance(manifest, dict) or manifest.get("schema") != MANIFEST_SCHEMA:
        raise ReleaseError(f"install manifest schema must be {MANIFEST_SCHEMA}")
    declared = manifest.get("targets")
    if not isinstance(declared, dict) or not declared:
        raise ReleaseError("install manifest needs a non-empty targets object")
    result = {}
    for target, names in declared.items():
        if target not in targets:
            raise ReleaseError(f"manifest target {target!r} has no configured directory")
        if not isinstance(names, list) or len(set(names)) != len(names):
            raise ReleaseError(f"manifest target {target!r} must be a list of unique names")
        for name in names:
            if not isinstance(name, str) or not name or "/" in name or name.startswith("."):
                raise ReleaseError(f"manifest target {target!r}: invalid skill name {name!r}")
            if not has_skill(name):
                raise ReleaseError(f"manifest target {target!r}: {name} has no SKILL.md in the release")
        result[target] = list(names)
    return result


def catalog_problems(catalog, skill_dirs: set[str], other_dirs: set[str], targets) -> list[str]:
    """Rule violations in a catalog, given the skills/ directories with and without SKILL.md.

    The rules keep classification and installation consistent: an uncatalogued
    directory would ship with no stated status, and every installed description
    costs each worker listing space, so installing one that is dormant, research
    or another project's needs a stated reason.
    """
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        return [f"catalog schema must be {CATALOG_SCHEMA}"]
    skills = catalog.get("skills")
    if not isinstance(skills, dict):
        return ["catalog needs a skills object"]
    problems = [f"{name}: directory has no SKILL.md, so it is not a skill" for name in sorted(other_dirs)]
    problems += [f"{name}: SKILL.md has no catalog entry" for name in sorted(skill_dirs - skills.keys())]
    for name, entry in sorted(skills.items()):
        if name not in skill_dirs:
            problems.append(f"{name}: catalog entry has no skill directory with SKILL.md")
        if not isinstance(entry, dict):
            problems.append(f"{name}: entry must be an object")
            continue
        unknown = sorted(entry.keys() - CATALOG_FIELDS)
        if unknown:
            problems.append(f"{name}: unknown fields {unknown}")
        kind, state, install = entry.get("kind"), entry.get("state"), entry.get("install")
        if kind not in KINDS:
            problems.append(f"{name}: kind must be one of {list(KINDS)}")
        if state not in STATES:
            problems.append(f"{name}: state must be one of {list(STATES)}")
        if not isinstance(install, list) or len(set(install)) != len(install) or any(t not in targets for t in install):
            problems.append(f"{name}: install must list unique targets from {sorted(targets)}")
            continue
        for field in ("install_reason", "not_installed_reason"):
            if field in entry and not (isinstance(entry[field], str) and entry[field].strip()):
                problems.append(f"{name}: {field} must be non-empty text")
        if install and state == "retired":
            problems.append(f"{name}: a retired skill is not installed")
        if install and (state == "dormant" or kind in ("research", "other-project")) and "install_reason" not in entry:
            problems.append(f"{name}: installed while {state if state == 'dormant' else kind}, so it needs install_reason")
        if not install and state in ("active", "stable") and "not_installed_reason" not in entry:
            problems.append(f"{name}: {state} but not installed, so it needs not_installed_reason")
    return problems


def install_lists(catalog) -> dict[str, list[str]]:
    lists: dict[str, list[str]] = {}
    for name, entry in sorted(catalog["skills"].items()):
        for target in entry["install"]:
            lists.setdefault(target, []).append(name)
    return lists


def skills_tree_dirs(paths) -> tuple[set[str], set[str]]:
    """Split skills/<dir> names into those with and without a SKILL.md."""
    paths = set(paths)
    dirs = {p.split("/")[1] for p in paths if p.count("/") >= 2}
    with_skill = {d for d in dirs if f"skills/{d}/SKILL.md" in paths}
    return with_skill, dirs - with_skill


def manifest_at(repo: Path, commit: str, targets: dict[str, Path]) -> dict[str, list[str]]:
    """Install lists for a commit: from its catalog, or from the older install manifest."""
    tree = set(tree_entries(repo, commit, "skills"))
    if CATALOG_PATH in tree:
        catalog = json.loads(git(repo, "show", f"{commit}:{CATALOG_PATH}"))
        problems = catalog_problems(catalog, *skills_tree_dirs(tree), targets)
        if problems:
            raise ReleaseError(f"{CATALOG_PATH} at {commit[:12]} breaks catalog rules:\n  " + "\n  ".join(problems))
        return install_lists(catalog)
    if MANIFEST_PATH not in tree:
        raise ReleaseError(f"{commit} has neither {CATALOG_PATH} nor {MANIFEST_PATH}")
    raw = git(repo, "show", f"{commit}:{MANIFEST_PATH}")
    return validate_manifest(json.loads(raw), targets, lambda name: f"skills/{name}/SKILL.md" in tree)


def synopsis_problems(repo: Path) -> list[str]:
    """Stale or malformed generated CLI synopses in the working tree's markdown."""
    spec = importlib.util.spec_from_file_location("cli_synopsis", Path(__file__).with_name("cli_synopsis.py"))
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return [f"cli-synopsis {problem}" for problem in module.synopsis_problems(repo)]


def check_working_tree(repo: Path, targets: dict[str, Path]) -> dict:
    """Validate the working tree's catalog against its skill directories, and its generated CLI synopses."""
    path = repo / CATALOG_PATH
    if not path.exists():
        return {"ok": False, "problems": [f"{CATALOG_PATH} does not exist"]}
    paths = [f"skills/{d.name}/{f.name}" for d in (repo / "skills").iterdir() if d.is_dir() for f in d.iterdir()]
    catalog = json.loads(path.read_text())
    problems = catalog_problems(catalog, *skills_tree_dirs(paths), targets)
    problems += synopsis_problems(repo)
    return {"ok": not problems, "problems": problems,
            "install": install_lists(catalog) if not problems else None}


# --- release store ------------------------------------------------------------

class Store:
    def __init__(self, root: Path):
        self.root = root
        self.releases = root / "releases"
        self.current = root / "current"
        self.tmp = root / "tmp"
        self.history = root / "history.jsonl"

    @contextlib.contextmanager
    def locked(self):
        for path in (self.root, self.releases, self.tmp):
            path.mkdir(parents=True, exist_ok=True)
        with open(self.root / "lock", "w") as handle:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            yield

    def release_dir(self, commit: str) -> Path:
        return self.releases / commit

    def current_commit(self) -> str | None:
        return Path(os.readlink(self.current)).name if self.current.is_symlink() else None

    def release_record(self, commit: str) -> dict:
        return json.loads((self.release_dir(commit) / "RELEASE.json").read_text())

    def link_target(self, name: str) -> str:
        return str(self.current / "skills" / name)

    def owns(self, entry: Path) -> bool:
        return entry.is_symlink() and os.readlink(entry).startswith(str(self.root) + os.sep)

    def staged_link(self, target: str) -> Path:
        staged = self.tmp / f"link-{os.getpid()}-{secrets.token_hex(6)}"
        os.symlink(target, staged)
        return staged

    def append_history(self, record: dict) -> None:
        with open(self.history, "a") as handle:
            handle.write(json.dumps(record, sort_keys=True) + "\n")

    def last_switch(self) -> dict | None:
        if not self.history.exists():
            return None
        records = [json.loads(line) for line in self.history.read_text().splitlines() if line.strip()]
        switches = [r for r in records if r.get("action") in ("deploy", "rollback")]
        return switches[-1] if switches else None


def set_read_only(root: Path) -> None:
    for current, dirs, files in os.walk(root, topdown=False):
        for name in files + dirs:
            full = os.path.join(current, name)
            if not os.path.islink(full):
                os.chmod(full, os.stat(full).st_mode & ~0o222)
    os.chmod(root, os.stat(root).st_mode & ~0o222)


def remove_tree(root: Path) -> None:
    for current, dirs, files in os.walk(root):
        os.chmod(current, os.stat(current).st_mode | 0o700)
    shutil.rmtree(root)


def verify_release(repo: Path, store: Store, commit: str) -> dict[str, list[str]]:
    return compare(tree_entries(repo, commit, "skills"),
                   dir_entries(store.release_dir(commit) / "skills", "skills"))


def build_release(repo: Path, store: Store, commit: str, manifest: dict) -> bool:
    """Export commit's skills/ into releases/<commit>. Returns False if it already existed."""
    final = store.release_dir(commit)
    if final.exists():
        problems = verify_release(repo, store, commit)
        if any(problems.values()):
            raise ReleaseError(f"existing release {final} does not match {commit}: {summary(problems)}")
        return False
    staging = store.tmp / f"export-{commit}-{os.getpid()}"
    if staging.exists():
        remove_tree(staging)
    staging.mkdir()
    archive = git(repo, "archive", "--format=tar", commit, "skills", binary=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(staging, filter="data")
    problems = compare(tree_entries(repo, commit, "skills"), dir_entries(staging / "skills", "skills"))
    if any(problems.values()):
        remove_tree(staging)
        raise ReleaseError(f"export of {commit} does not match its tree: {summary(problems)}")
    # Node scripts resolve packages by walking up from their own location.
    node_modules, lock = repo / "node_modules", repo / "package-lock.json"
    if node_modules.is_dir():
        os.symlink(node_modules, staging / "node_modules")
    source = CATALOG_PATH if (staging / CATALOG_PATH).exists() else MANIFEST_PATH
    record = {
        "schema": RELEASE_SCHEMA,
        "commit": commit,
        "skills_tree": git(repo, "rev-parse", f"{commit}:skills").strip(),
        "manifest": manifest,
        "install_source": {"path": source, "blob": git(repo, "rev-parse", f"{commit}:{source}").strip()},
        "created_at": now(),
        "repository": str(repo),
        "node_modules": {
            "link": str(node_modules) if node_modules.is_dir() else None,
            "package_lock_sha256": hashlib.sha256(lock.read_bytes()).hexdigest() if lock.exists() else None,
        },
    }
    (staging / "RELEASE.json").write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    set_read_only(staging / "skills")
    os.chmod(staging / "RELEASE.json", 0o444)
    os.rename(staging, final)  # needs staging writable (its ".." entry changes)
    os.chmod(final, 0o555)
    return True


# --- planning -----------------------------------------------------------------

def entry_kind(entry: Path) -> str:
    if entry.is_symlink():
        return "symlink"
    if entry.is_dir():
        return "copy"
    return "file" if entry.exists() else "absent"


def content_change(repo: Path, commit: str, name: str, entry: Path) -> dict | None:
    """Compare what an agent reads through entry now with skills/<name> at commit."""
    if not entry.exists() or not entry.resolve().is_dir():
        return None
    resolved = entry.resolve()
    problems = compare(tree_entries(repo, commit, f"skills/{name}"), dir_entries(resolved, f"skills/{name}"))
    skill_md = resolved / "SKILL.md"
    current_front = frontmatter(skill_md.read_text(errors="replace")) if skill_md.exists() else ""
    new_front = frontmatter(git(repo, "show", f"{commit}:skills/{name}/SKILL.md"))
    return {"files": {k: len(v) for k, v in problems.items()}, "paths": problems,
            "listing_changes": current_front != new_front}


def plan(repo: Path, store: Store, targets: dict[str, Path], commit: str,
         manifest: dict[str, list[str]]) -> dict:
    result = {"commit": commit, "current": store.current_commit(),
              "release_exists": store.release_dir(commit).exists(), "targets": {}}
    for target, names in manifest.items():
        directory = targets[target]
        if not directory.is_dir():
            raise ReleaseError(f"target directory {directory} does not exist")
        actions = []
        for name in names:
            entry = directory / name
            kind = entry_kind(entry)
            action = {"absent": "link", "copy": "replace-copy", "file": "blocked"}.get(kind)
            if kind == "symlink":
                action = "keep" if os.readlink(entry) == store.link_target(name) else "retarget"
            actions.append({"name": name, "entry": kind, "action": action,
                            "resolves_to": str(entry.resolve()) if entry.exists() else None,
                            "change": content_change(repo, commit, name, entry)})
        others = sorted(e for e in directory.iterdir() if e.name not in names)
        actions += [{"name": e.name, "entry": "symlink", "action": "unlink",
                     "resolves_to": str(e.resolve()), "change": None} for e in others if store.owns(e)]
        result["targets"][target] = {"directory": str(directory), "actions": actions,
                                     "unmanaged": [e.name for e in others if not store.owns(e)]}
    return result


# --- applying -----------------------------------------------------------------

def same_filesystem(a: Path, b: Path) -> bool:
    return os.stat(a).st_dev == os.stat(b).st_dev


def dot_entries(targets: dict[str, Path], manifest: dict) -> dict[str, list[str]]:
    return {t: sorted(e.name for e in targets[t].iterdir() if e.name.startswith(".")) for t in manifest}


def backup_path(backup_root: Path, stamp: str, target: str, name: str) -> Path:
    path = backup_root / stamp / target / name
    if path.exists() or path.is_symlink():
        raise ReleaseError(f"backup path {path} already exists")
    path.parent.mkdir(parents=True, exist_ok=True)
    return path


def stamp() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def apply(repo: Path, store: Store, targets: dict[str, Path], backup_root: Path, commit: str,
          manifest: dict, replace_copies: bool, action_name: str) -> dict:
    planned = plan(repo, store, targets, commit, manifest)
    items = [(t, a) for t, v in planned["targets"].items() for a in v["actions"]]
    blocked = [f"{t}/{a['name']}" for t, a in items
               if a["action"] == "blocked" or (a["action"] == "replace-copy" and not replace_copies)]
    if blocked:
        raise ReleaseError("refusing: real copies or files sit where managed links belong "
                           f"(review the plan, then rerun with --replace-copies): {', '.join(blocked)}")
    if any(a["action"] == "replace-copy" for _, a in items):
        backup_root.mkdir(parents=True, exist_ok=True)
    for target in manifest:
        if not same_filesystem(store.root, targets[target]):
            raise ReleaseError(f"{targets[target]} is not on the release root's filesystem")
        if backup_root.exists() and not same_filesystem(backup_root, targets[target]):
            raise ReleaseError(f"backup root {backup_root} is not on {targets[target]}'s filesystem")

    dots_before = dot_entries(targets, manifest)
    previous = store.current_commit()
    if previous == commit:
        previous = (store.last_switch() or {}).get("previous")
    created = build_release(repo, store, commit, manifest)
    os.replace(store.staged_link(f"releases/{commit}"), store.current)

    when, changes = stamp(), {}
    for target, item in items:
        entry, action = targets[target] / item["name"], item["action"]
        change = {"name": item["name"], "action": action}
        if action == "link":
            os.symlink(store.link_target(item["name"]), entry)
        elif action == "retarget":
            os.replace(store.staged_link(store.link_target(item["name"])), entry)
        elif action == "replace-copy":
            # rename(2) cannot put a symlink over a directory: stage the link outside the
            # skill directory, move the copy out, move the link in.
            staged = store.staged_link(store.link_target(item["name"]))
            change["backup"] = str(backup_path(backup_root, when, target, item["name"]))
            os.rename(entry, change["backup"])
            os.rename(staged, entry)
        elif action == "unlink":
            entry.unlink()
        if action != "keep":
            changes.setdefault(target, []).append(change)

    problems = verify_links(store, targets, manifest, commit)
    if dot_entries(targets, manifest) != dots_before:
        problems.append("dot-prefixed entries changed in a skill directory")
    if problems:
        raise ReleaseError("post-deploy verification failed: " + "; ".join(problems))
    record = {"action": action_name, "at": now(), "commit": commit, "previous": previous,
              "release_created": created, "changes": changes}
    store.append_history(record)
    return record


def verify_links(store: Store, targets: dict[str, Path], manifest: dict, commit: str) -> list[str]:
    problems = []
    for target, names in manifest.items():
        for name in names:
            entry = targets[target] / name
            expected = (store.release_dir(commit) / "skills" / name).resolve()
            if not entry.is_symlink() or entry.resolve() != expected:
                problems.append(f"{target}/{name} does not resolve to {expected}")
    return problems


def retire(store: Store, targets: dict[str, Path], backup_root: Path, target: str, names: list[str]) -> dict:
    if target not in targets:
        raise ReleaseError(f"unknown target {target!r}")
    directory = targets[target]
    current = store.current_commit()
    managed = set(store.release_record(current)["manifest"].get(target, [])) if current else set()
    for name in names:
        entry = directory / name
        if not (entry.exists() or entry.is_symlink()):
            raise ReleaseError(f"{entry} does not exist")
        if name in managed or store.owns(entry):
            raise ReleaseError(f"{entry} is managed by the current release; drop it from the manifest instead")
    backup_root.mkdir(parents=True, exist_ok=True)
    if not same_filesystem(backup_root, directory):
        raise ReleaseError(f"backup root {backup_root} is not on {directory}'s filesystem")
    when, moved = stamp(), []
    for name in names:
        destination = backup_path(backup_root, when, target, name)
        os.rename(directory / name, destination)
        moved.append({"name": name, "backup": str(destination)})
    record = {"action": "retire", "at": now(), "target": target, "moved": moved}
    store.append_history(record)
    return record


def status(repo: Path, store: Store, targets: dict[str, Path], verify: bool) -> dict:
    current = store.current_commit()
    if current is None:
        return {"current": None}
    record = store.release_record(current)
    result = {"current": current, "created_at": record["created_at"], "targets": {}}
    for target, names in record["manifest"].items():
        directory = targets[target]
        result["targets"][target] = {
            "managed": len(names),
            "not_linked": [n for n in names if not (directory / n).is_symlink()
                           or os.readlink(directory / n) != store.link_target(n)],
            "unmanaged": sorted(e.name for e in directory.iterdir()
                                if e.name not in names and not store.owns(e)),
        }
    try:
        changed = git(repo, "diff-tree", "-r", "--name-only", record["skills_tree"],
                      working_tree_skills(repo)).split()
        result["undeployed_changes"] = sorted({p.split("/")[0] for p in changed})
    except ReleaseError:
        result["undeployed_changes"] = "not established (release tree not in this repository)"
    if verify:
        result["release_verification"] = summary(verify_release(repo, store, current))
    return result


# --- output and CLI -----------------------------------------------------------

def describe(item: dict) -> str:
    change = item["change"]
    if change is None:
        return "nothing there now" if item["action"] == "link" else ""
    files = change["files"]
    if not files["differ"] and not files["missing"]:
        note = "same bytes" + (f" ({files['extra']} untracked/ignored files beside them)" if files["extra"] else "")
    else:
        note = ", ".join(f"{n} {k}" for k, n in files.items() if n)
    return note + ("; LISTING CHANGES" if change["listing_changes"] else "")


def render_plan(planned: dict) -> str:
    lines = [f"release {planned['commit']} ({'already built' if planned['release_exists'] else 'new'})",
             f"current {planned['current'] or 'none'}"]
    for target, details in planned["targets"].items():
        lines.append(f"\n[{target}] {details['directory']}")
        for item in details["actions"]:
            lines.append(f"  {item['action']:<13} {item['name']:<32} {item['entry']:<8} {describe(item)}")
        if details["unmanaged"]:
            lines.append(f"  unmanaged, left alone: {', '.join(details['unmanaged'])}")
    return "\n".join(lines)


def configured_targets(overrides: list[str]) -> dict[str, Path]:
    targets = dict(DEFAULT_TARGETS)
    for item in overrides:
        name, _, path = item.partition("=")
        if not name or not path:
            raise ReleaseError(f"--target-dir must be NAME=PATH, got {item!r}")
        targets[name] = Path(path).expanduser().absolute()
    return targets


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repository", type=Path, default=Path("."))
    parser.add_argument("--release-root", type=Path, default=DEFAULT_RELEASE_ROOT)
    parser.add_argument("--backup-root", type=Path, default=DEFAULT_BACKUP_ROOT)
    parser.add_argument("--target-dir", action="append", default=[], metavar="NAME=PATH")
    parser.add_argument("--json", action="store_true", help="print the plan as JSON")
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan", help="show what deploying REF would change; changes nothing")
    p.add_argument("ref", nargs="?", default="HEAD")
    d = sub.add_parser("deploy", help="build REF's release, switch current, reconcile links")
    d.add_argument("ref", nargs="?", default="HEAD")
    d.add_argument("--replace-copies", action="store_true",
                   help="move real directories found in managed entries to the backup root")
    sub.add_parser("check", help="validate the working tree's catalog and CLI synopses; changes nothing")
    s = sub.add_parser("status", help="show the deployed release and link state")
    s.add_argument("--verify", action="store_true", help="re-hash the release against its commit")
    sub.add_parser("rollback", help="redeploy the release that preceded the current one")
    r = sub.add_parser("retire", help="move unmanaged entries out of a skill directory")
    r.add_argument("target")
    r.add_argument("names", nargs="+")
    args = parser.parse_args(argv)

    try:
        repo = Path(git(args.repository.resolve(), "rev-parse", "--show-toplevel").strip())
        store = Store(args.release_root.expanduser().absolute())
        targets = configured_targets(args.target_dir)
        backup_root = args.backup_root.expanduser().absolute()
        if args.command == "plan":
            commit = resolve_commit(repo, args.ref)
            result = plan(repo, store, targets, commit, manifest_at(repo, commit, targets))
            print(json.dumps(result, indent=2) if args.json else render_plan(result))
            return 0
        if args.command == "status":
            print(json.dumps(status(repo, store, targets, args.verify), indent=2))
            return 0
        if args.command == "check":
            result = check_working_tree(repo, targets)
            print(json.dumps(result, indent=2))
            return 0 if result["ok"] else 1
        with store.locked():
            if args.command == "deploy":
                commit = resolve_commit(repo, args.ref)
                result = apply(repo, store, targets, backup_root, commit,
                               manifest_at(repo, commit, targets), args.replace_copies, "deploy")
            elif args.command == "rollback":
                last = store.last_switch()
                if not last or not last.get("previous"):
                    raise ReleaseError("no previous release to roll back to")
                commit = last["previous"]
                release = store.release_dir(commit)
                manifest = validate_manifest(
                    {"schema": MANIFEST_SCHEMA, "targets": store.release_record(commit)["manifest"]},
                    targets, lambda n: (release / "skills" / n / "SKILL.md").exists())
                result = apply(repo, store, targets, backup_root, commit, manifest, False, "rollback")
            else:
                result = retire(store, targets, backup_root, args.target, args.names)
        print(json.dumps(result, indent=2))
        return 0
    except (ReleaseError, OSError, json.JSONDecodeError) as error:
        print(f"skill-release: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
