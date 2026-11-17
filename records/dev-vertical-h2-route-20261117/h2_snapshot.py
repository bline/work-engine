#!/usr/bin/env python3
"""Materialize an arm of the draft H2 snapshot (S2 artifacts) from git.

Usage: h2_snapshot.py <out-dir> treatment|baseline
Each member is read with `git show <commit>:<path>` and checked against the
manifest's size and SHA-256. The manifest is a draft proposal, not approved.
"""
import hashlib, json, subprocess, sys
from pathlib import Path
MANIFEST = Path('/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/007-h2-artifacts/snapshot-manifest.json')
out, arm = Path(sys.argv[1]), sys.argv[2]
m = json.loads(MANIFEST.read_text())
members = (m['treatment_arm_members']['ordinary_policy_members'] + m['treatment_arm_members']['research_bundle_members']
           if arm == 'treatment' else m['ordinary_policy_baseline_members']['members'])
for x in members:
    data = subprocess.run(['git', '-C', '/home/bline/code/work-engine', 'show', f"{x['commit']}:{x['path']}"],
                          capture_output=True, check=True).stdout
    if len(data) != x['bytes'] or hashlib.sha256(data).hexdigest() != x['sha256']:
        sys.exit(f"member mismatch: {x['path']}")
    p = out / x['path']; p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(data)
print(json.dumps({'arm': arm, 'members': len(members), 'bytes': sum(x['bytes'] for x in members), 'manifest_sha256': hashlib.sha256(MANIFEST.read_bytes()).hexdigest()}))
