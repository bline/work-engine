#!/usr/bin/env python3
"""Build one real changed-tree, profiled, gate-executed SC3 test subject.

The resulting capture is fixture evidence, not a production gate attestation.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


HERE = Path(__file__).resolve()
PROJECT = HERE.parents[5]
FIXTURES = HERE.parent.parent / "fixtures"
VECTORS = FIXTURES / "vectors"

# Historical SC0 pair independently pinned by src/subject_binding.rs.
ANALYZER = PROJECT / "skills" / "code-change-profile" / "scripts" / "code_change_profile.py"
ANALYZER_SHA256 = "47e486ce1539bbfe1ab466d77650d9bb3e0338222478ae445fe6a07ba95e2003"
# Exact bytes of skills/slice-checkpoint/scripts/checkpoint.py at
# 0a71dfada0852b3d8445961ffeef9b15c27e0b43 (blob 4d18174e4521e0d3bec4f627c3c7f5b870cee7f9).
CHECKPOINT_FIXTURE = FIXTURES / "sc0-checkpoint-validator-v2.py.fixture"
CHECKPOINT_SHA256 = "1b814da3ef9e1f20804b934b91f6621ed65159dd9dfab5bfbfdd2df95f82d20b"
GIT = "/usr/bin/git"
PYTHON = "/usr/bin/python3.14"
GATE_RUNNER = "skills/slice-builder/scripts/run_gate.py"
# Same reviewed pins as subject_binding.rs. Measure the executable in use;
# either reviewed Git image may run the gate without borrowing the seed's pin.
GIT_SHA256 = {
    "5516c9f362c29376ab9a499a33082f9f611941d8c75930c880e30ad109e39c9a",
    "ed8b2df973b28c0fb773b9c0dc0816d3ee2b3835bb932d9153c21417bb51b362",
}
PYTHON_SHA256 = "52e0a13e60a981d8c4b6478be2ba5176f69da07948a056bf49cf6f077e30cb41"
GATE_RUNNER_SHA256 = "c8fee4d50fc5689c253fb1ef76a4b86159a3c8a916b76db9a764fbe5c0ed0dba"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canon(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def git(repo, *args, input=None, env=None, raw=False):
    result = subprocess.check_output(
        [GIT, "-C", str(repo), *args], input=input, env=env
    )
    return result if raw else result.strip()


def gate_runtime(runner):
    git_sha, python_sha, runner_sha = (sha(Path(p).read_bytes()) for p in (GIT, PYTHON, runner))
    if (git_sha not in GIT_SHA256 or python_sha != PYTHON_SHA256
            or runner_sha != GATE_RUNNER_SHA256):
        raise SystemExit("SC0 fixture gate runtime or runner differs from the reviewed pins")
    version = subprocess.check_output([GIT, "--version"], text=True).strip()
    python = json.loads(subprocess.check_output([
        PYTHON, "-I", "-c", "import json, platform; print(json.dumps({"
        "'implementation': platform.python_implementation(), 'version': platform.python_version()}))",
    ]))
    return {"executable_sha256": git_sha, "version": version}, {
        **python, "executable_sha256": python_sha,
    }, runner_sha


def gate_checkout_state(checkout, paths):
    return {
        "head": git(checkout, "rev-parse", "HEAD").decode(),
        "head_tree": git(checkout, "rev-parse", "HEAD^{tree}").decode(),
        "path_sha256": {p: sha((checkout / p).read_bytes()) for p in paths},
        "all_status": git(checkout, "status", "--porcelain", "--untracked-files=all").decode(),
        "tracked_paths": git(checkout, "ls-files", "-z", raw=True).decode().rstrip("\0").split("\0"),
    }


def fixture(digest):
    return json.loads((VECTORS / f"{digest}.json").read_text())


def main(repo, output, predecessor_path=None):
    if sha(ANALYZER.read_bytes()) != ANALYZER_SHA256:
        raise SystemExit("SC0 fixture analyzer does not match the historical SHA-256")
    checkpoint_bytes = CHECKPOINT_FIXTURE.read_bytes()
    if sha(checkpoint_bytes) != CHECKPOINT_SHA256:
        raise SystemExit("SC0 fixture checkpoint validator does not match the historical SHA-256")

    output.mkdir(parents=True, exist_ok=True)
    candidate = fixture("4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd")
    predecessor = json.loads(predecessor_path.read_text()) if predecessor_path else candidate
    generation = 2 if predecessor_path else 1
    original = predecessor["checkpoint_commit_oid"]
    original_tree = predecessor["checkpoint_tree_oid"]
    old = git(repo, "show", f"{original_tree}:sample.py", raw=True)
    blob = git(repo, "hash-object", "-w", "--stdin",
               input=old + f"\n# retained remediation fixture generation {generation}\n".encode()).decode()
    env = os.environ.copy()
    env["GIT_INDEX_FILE"] = str(output / "candidate-index")
    git(repo, "read-tree", original_tree, env=env)
    git(repo, "update-index", "--cacheinfo", f"100644,{blob},sample.py", env=env)
    tree = git(repo, "write-tree", env=env).decode()
    commit_env = os.environ.copy()
    commit_env.update({
        "GIT_AUTHOR_NAME": "SC3 fixture", "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
        "GIT_COMMITTER_NAME": "SC3 fixture", "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
        "GIT_AUTHOR_DATE": "2026-10-08T00:00:00+00:00",
        "GIT_COMMITTER_DATE": "2026-10-08T00:00:00+00:00",
    })
    gate_commit = git(repo, "commit-tree", tree, "-p", original,
                      "-m", f"SC3 fixture frozen gate tree {generation}", env=commit_env).decode()
    gate_checkout = output / "gate-checkout"
    git(repo, "worktree", "add", "--detach", str(gate_checkout), gate_commit)
    paths = [row["path"] for row in candidate["paths"]]
    hashes = {path: sha(git(repo, "show", f"{tree}:{path}", raw=True)) for path in paths}
    check = ("import hashlib, pathlib, subprocess, sys\n"
             f"expected = {hashes!r}\n"
             "if sys.flags.isolated != 1 or sys.flags.optimize != 0: raise SystemExit(17)\n"
             f"if subprocess.check_output([{GIT!r}, 'rev-parse', 'HEAD'], text=True).strip() != {gate_commit!r}: raise SystemExit(20)\n"
             f"if subprocess.check_output([{GIT!r}, 'rev-parse', 'HEAD^{{tree}}'], text=True).strip() != {tree!r}: raise SystemExit(21)\n"
             "actual = {path: hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest() for path in expected}\n"
             "if actual != expected: raise SystemExit(19)\n"
             f"if subprocess.check_output([{GIT!r}, 'status', '--porcelain', '--untracked-files=all'], text=True).strip(): raise SystemExit(22)\n")
    manifest = {"checks": [{"command": [PYTHON, "-I", "-c", check],
                            "identity": "sc3-changed-tree-exact-path-bytes",
                            "requirement": "controlled_subject_validation", "timeout_seconds": 10}]}
    manifest_raw = canon(manifest)
    runner = PROJECT / GATE_RUNNER
    argv = [PYTHON, "-I", str(runner), "--manifest-json",
            manifest_raw.decode(), "--cwd", str(gate_checkout)]
    before = gate_checkout_state(gate_checkout, paths)
    runtime = gate_runtime(runner)
    process = subprocess.run(argv, check=True, capture_output=True)
    if gate_runtime(runner) != runtime:
        raise SystemExit("SC0 fixture gate runtime changed across execution")
    after = gate_checkout_state(gate_checkout, paths)
    if (before != after or before["all_status"] or before["head"] != gate_commit
            or before["head_tree"] != tree or before["path_sha256"] != hashes):
        raise SystemExit("SC0 fixture gate checkout differs from the selected tree and paths")
    receipt_raw = process.stdout.strip()
    receipt = json.loads(receipt_raw)
    if receipt["status"] != "passed" or process.stderr:
        raise SystemExit(f"SC0 fixture gate did not pass cleanly: {process.stderr!r}; {receipt!r}")
    capture = fixture("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206")
    git_info, python_info, runner_sha = runtime
    capture.update(argv=argv, git_runtime_before_after=git_info,
                   python_runtime_before_after=python_info, producer=GATE_RUNNER,
                   producer_sha256_before_after=runner_sha, child_interpreter=PYTHON,
                   child_interpreter_sha256=python_info["executable_sha256"])
    capture["argv_manifest_sha256"] = sha(manifest_raw)
    capture["candidate_tree_oid"] = tree
    capture["frozen_gate_tree_oid"] = tree
    capture["frozen_gate_commit_oid"] = gate_commit
    capture["candidate_diff_paths"] = paths
    capture["candidate_manifest_path_sha256"] = hashes
    capture["candidate_tree_path_sha256"] = hashes
    capture["tested_path_sha256"] = hashes
    capture["cwd"] = str(gate_checkout)
    capture["before"] = before
    capture["after"] = after
    capture["exit_status"] = process.returncode
    capture["stderr"] = process.stderr.decode()
    capture["failing_probe"] = None
    receipt_digest = sha(receipt_raw)
    candidate.update({
        "repository": str(repo), "candidate_attempt": generation + 1,
        "checkpoint_id": sha(f"sc3-remediation-fixture-generation-{generation}".encode()),
        "parent_checkpoint_commit_oid": original,
        "checkpoint_tree_oid": tree,
        "gate_receipt_digest": receipt_digest,
        "created_at": f"2026-10-08T00:00:0{generation}+00:00",
        "ref": f"refs/work-engine/checkpoints/sc0-oracle-c2/slice-1/candidate-{generation + 1}",
    })
    for row in candidate["paths"]:
        row["content_digest"] = hashes[row["path"]]
    candidate["manifest_digest"] = sha(canon(candidate["paths"]))
    patch = git(repo, "diff-tree", "--binary", "--no-renames", "--no-ext-diff",
                candidate["baseline_tree_oid"], tree, raw=True)
    candidate["task_patch_digest"] = sha(patch)
    metadata = {"work_engine_checkpoint": 1, "kind": "candidate",
                "request_id": candidate["checkpoint_id"], "run_id": candidate["run_id"],
                "slice_number": candidate["slice_number"], "candidate_attempt": generation + 1,
                "tree": tree, "task_patch_digest": candidate["task_patch_digest"],
                "manifest_digest": candidate["manifest_digest"],
                "gate_receipt_digest": receipt_digest,
                "plan_version": candidate["plan_version"],
                "scope_revision": candidate["scope_revision"]}
    commit = git(repo, "commit-tree", tree, "-p", original,
                 "-m", canon(metadata).decode(), env=commit_env).decode()
    git(repo, "update-ref", candidate["ref"], commit)
    candidate["checkpoint_commit_oid"] = commit
    capture["candidate_commit_oid"] = commit
    candidate_path = output / "candidate.json"
    candidate_path.write_bytes(canon(candidate))
    subject_path = output / "subject.json"
    subject_path.write_bytes(canon({"schema_version": 2,
        "construction_method": "slice_checkpoint_candidate_receipt",
        "evidence_cutoff": candidate["created_at"], "checkpoint": candidate}))
    validator = repo / "skills" / "slice-checkpoint" / "scripts" / "checkpoint.py"
    validator.parent.mkdir(parents=True, exist_ok=True)
    validator.write_bytes(checkpoint_bytes)
    profile_raw = subprocess.check_output([
        "/usr/bin/python3", str(ANALYZER), "profile", "--receipt", str(subject_path),
        "--repository", str(repo)]).strip()
    artifacts = {"candidate": candidate_path, "profile": output / "profile.json",
                 "capture": output / "capture.json", "manifest": output / "manifest.json",
                 "receipt": output / "receipt.json"}
    artifacts["profile"].write_bytes(profile_raw)
    artifacts["capture"].write_bytes(canon(capture))
    artifacts["manifest"].write_bytes(manifest_raw)
    artifacts["receipt"].write_bytes(receipt_raw)
    print(json.dumps({key: str(path) for key, path in artifacts.items()}))


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve(),
         Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None)
