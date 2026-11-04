#!/usr/bin/python3
"""Static draft: assemble byte-identical raw assessment inputs plus one map."""
import hashlib, json, pathlib, shutil, sys

assert len(sys.argv) == 7, 'RAW_ROOT RAW_MANIFEST PUBLIC_OBLIGATIONS PROMPT MAP OUTPUT_ROOT'
raw_root, manifest, obligations, prompt, map_file, output_root = map(pathlib.Path, sys.argv[1:])
assert all(p.is_file() and not p.is_symlink() for p in (manifest, obligations, prompt, map_file))
assert raw_root.is_dir() and not raw_root.is_symlink()
assert not output_root.exists(), 'no overwrite of assessment inputs'
rows = json.loads(manifest.read_text())
assert isinstance(rows, list) and rows
paths = []
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
for row in rows:
    assert set(row) == {'path', 'sha256'}
    rel = pathlib.PurePosixPath(row['path'])
    assert not rel.is_absolute() and '..' not in rel.parts and len(rel.parts) > 0
    assert rel.parts[0] == 'raw' and rel.name not in ('prompt.md', 'correspondence-map.json')
    assert row['path'] not in paths
    src = raw_root / row['path']
    assert src.is_file() and not src.is_symlink() and src.resolve().is_relative_to(raw_root.resolve())
    assert sha(src) == row['sha256']
    paths.append(row['path'])
assert len(paths) == len(set(paths))
output_root.mkdir(mode=0o700, parents=True)
for arm in ('control', 'intervention'):
    arm_input = output_root / arm / 'input'
    arm_input.mkdir(parents=True, mode=0o700)
    for rel in paths:
        dst = arm_input / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(raw_root / rel, dst)
    for src, name in ((manifest, 'raw-corpus-v1.json'),
                      (obligations, 'public-obligations.json'), (prompt, 'prompt.md')):
        shutil.copyfile(src, arm_input / name)
    if arm == 'intervention':
        shutil.copyfile(map_file, arm_input / 'correspondence-map.json')
    for p in arm_input.rglob('*'):
        p.chmod(0o555 if p.is_dir() else 0o444)
    arm_input.chmod(0o555)
control = output_root / 'control/input'
intervention = output_root / 'intervention/input'
files = lambda d: {str(p.relative_to(d)): sha(p) for p in d.rglob('*') if p.is_file()}
c, i = files(control), files(intervention)
assert len(c) == len(rows) + 3 and len(i) == len(rows) + 4
assert set(i) - set(c) == {'correspondence-map.json'}
assert all(i[k] == v for k, v in c.items())
print(json.dumps({'raw_count': len(rows), 'control_file_count': len(c),
                  'intervention_file_count': len(i), 'common_hashes_equal': True,
                  'only_extra_file': 'correspondence-map.json'}, sort_keys=True))
