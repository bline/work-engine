#!/usr/bin/env python3
"""Build the Pi Host runtime root for the claude-sdk provider (W2/W4).

Usage: claude_runtime.py <out-dir>
The root is the pinned Pi runtime (hard links) plus, under node_modules, the
pi-claude-subscription extension and its dependency closure from the
operator's Pi package directory (the Claude Agent SDK, its linux-x64 `claude`
binary and the MCP SDK). Packages the Pi runtime already has are not
replaced. Writes <out-dir>/root and <out-dir>/runtime-manifest.json (the Host's
inventory format) and prints a JSON summary.
"""
import hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

BASE = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi')
EXT = Path.home() / '.pi/agent/npm/node_modules'
PLATFORM = '@anthropic-ai/claude-agent-sdk-linux-x64'

def closure():
    seen, todo = {}, ['pi-claude-subscription']
    while todo:
        name = todo.pop()
        if name in seen:
            continue
        pj = EXT / name / 'package.json'
        if not pj.exists():
            seen[name] = None  # optional platform packages of other systems
            continue
        p = json.loads(pj.read_text())
        seen[name] = p['version']
        deps = dict(p.get('dependencies', {}))
        deps.update(p.get('optionalDependencies', {}))
        todo.extend(deps)
    return {k: v for k, v in seen.items() if v is not None}

def sha(p):
    h = hashlib.sha256()
    with open(p, 'rb') as f:
        for b in iter(lambda: f.read(1 << 20), b''):
            h.update(b)
    return h.hexdigest()

def inventory(root):
    out = {}
    for dp, dns, fns in os.walk(root):
        for n in list(dns):
            p = Path(dp) / n
            if p.is_symlink():
                dns.remove(n)
                fns.append(n)
        for n in fns:
            p = Path(dp) / n
            rel = str(p.relative_to(root))
            if p.is_symlink():
                if not str(p.resolve()).startswith(str(root)):
                    sys.exit(f'symlink escapes root: {rel}')
                out[rel] = {'kind': 'symlink', 'target': os.readlink(p)}
            elif p.is_file():
                out[rel] = {'kind': 'file', 'sha256': sha(p)}
            else:
                sys.exit(f'special file: {rel}')
    return dict(sorted(out.items()))

def main():
    out = Path(sys.argv[1]).resolve()
    root = out / 'root'
    if root.exists():
        sys.exit(f'{root} exists')
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run(['cp', '-al', str(BASE), str(root)], check=True)
    packages = closure()
    added, kept = [], []
    for name, version in sorted(packages.items()):
        dst = root / 'node_modules' / name
        if dst.exists():
            kept.append(name)
            continue
        dst.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(['cp', '-al', str(EXT / name), str(dst)], check=True)
        added.append(f'{name}@{version}')
    if PLATFORM not in packages or not (root / 'node_modules' / PLATFORM / 'claude').is_file():
        sys.exit('claude binary absent')
    inv = inventory(root)
    manifest = json.dumps(inv, separators=(',', ':'), sort_keys=True)
    (out / 'runtime-manifest.json').write_text(manifest)
    summary = {'root': str(root), 'entries': len(inv), 'added': added, 'kept_from_pi_runtime': kept,
               'manifest_sha256': hashlib.sha256(manifest.encode()).hexdigest(),
               'claude_sha256': inv[f'node_modules/{PLATFORM}/claude']['sha256'],
               'extension_version': packages['pi-claude-subscription']}
    (out / 'summary.json').write_text(json.dumps(summary, indent=1))
    print(json.dumps(summary, indent=1))

if __name__ == '__main__':
    main()
