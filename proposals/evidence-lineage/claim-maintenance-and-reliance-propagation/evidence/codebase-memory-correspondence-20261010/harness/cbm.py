"""Thin wrapper over the isolated codebase-memory CLI (supported interface only).

The one exception is `oracle_*`: direct read-only SQLite reads used as an
experimental oracle, never by the observer itself.
"""
import json
import os
import sqlite3
import subprocess

# Work directory holding cbm.sh (isolated runner) and cache/; see README.md.
SP = os.environ["CBM_EXPERIMENT_DIR"]
RUNNER = os.path.join(SP, "cbm.sh")
CACHE = os.path.join(SP, "cache")


class CbmError(RuntimeError):
    pass


def call(tool, **flags):
    args = [RUNNER, tool]
    for k, v in flags.items():
        if v is None:
            continue
        if isinstance(v, (list, dict)):
            v = json.dumps(v)
        elif isinstance(v, bool):
            v = "true" if v else "false"
        args += ["--" + k.replace("_", "-"), str(v)]
    p = subprocess.run(args, capture_output=True, text=True, timeout=600)
    out = p.stdout.strip()
    try:
        data = json.loads(out)
    except json.JSONDecodeError:
        raise CbmError(f"{tool}: non-JSON output rc={p.returncode}: {out[:300]} {p.stderr[-300:]}")
    if isinstance(data, dict) and "error" in data and len(data) <= 3:
        raise CbmError(f"{tool}: {data}")
    return data


def index(repo):
    return call("index_repository", repo_path=repo)


def query(project, cypher):
    d = call("query_graph", project=project, query=cypher, format="json", max_rows=0)
    return [dict(zip(d["columns"], r)) for r in d["rows"]]


def snippet(project, qn):
    return call("get_code_snippet", project=project, qualified_name=qn, source_mode="full", format="json")


def coverage(project, paths):
    return call("check_index_coverage", project=project, paths=paths, format="json", diagnostics="full")


def detect_changes(project, **kw):
    return call("detect_changes", project=project, format="json", **kw)


def delete_project(project):
    return call("delete_project", project=project)


def q(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


# ── oracle (private storage; never used by the observer) ──────────────────
def _db(project):
    return sqlite3.connect(f"file:{os.path.join(CACHE, project + '.db')}?mode=ro", uri=True)


def oracle_generation(project):
    with _db(project) as c:
        return dict(c.execute("select k, v from store_meta").fetchall())


def oracle_file_sha(project, rel_path):
    with _db(project) as c:
        r = c.execute("select sha256 from file_hashes where project=? and rel_path=?",
                      (project, rel_path)).fetchone()
        return r[0] if r else None
