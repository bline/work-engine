"""Owner-side deterministic review packet assembly, not product qualification."""
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
BEGIN = "===== BEGIN FILE {path} | sha256 {sha256} | {bytes} bytes | {lines} lines ====="
END = "===== END FILE {path} ====="
SENTINEL = "===== END OF REVIEW SET: {count} files ====="


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def bound(path, expected):
    raw = pathlib.Path(path).read_bytes()
    assert digest(raw) == expected, f"changed source: {path}"
    return raw


def subject(config):
    freeze = json.loads(bound(config['freeze_manifest'], config['freeze_sha256']))
    root = pathlib.Path(config['subject_root'])
    for item in freeze['entries']:
        path = root / item['path']
        if item['type'] == 'file':
            bound(path, item['sha256'])
        elif item['type'] == 'symlink':
            assert path.is_symlink(), str(path)
            raise AssertionError(f"symlink requires explicit review binding: {path}")
        elif item['type'] != 'directory':
            raise AssertionError(f"unhandled frozen entry type: {item['type']}")


def material(item):
    raw = bound(item['source'], item['source_sha256'])
    text = raw.decode('utf-8')
    kind = item.get('kind', 'full')
    if kind == 'full':
        return text
    if kind == 'compact_json':
        data = json.loads(text)
        return f"Complete JSON value, whitespace normalized; source sha256 {item['source_sha256']}.\n" + json.dumps(data, sort_keys=True, separators=(',', ':')) + '\n'
    if kind == 'excerpt':
        lines = text.splitlines()
        parts = [f"Source {item['source']}; sha256 {item['source_sha256']}. Omitted ranges are outside this excerpt."]
        for first, last in item['ranges']:
            assert 1 <= first <= last <= len(lines)
            parts.append(f"Lines {first}-{last}:")
            parts.extend(f"{n:5d}| {lines[n-1]}" for n in range(first, last + 1))
        return '\n'.join(parts) + '\n'
    if kind == 'json_projection':
        data = json.loads(text)
        selected = {key: data[key] for key in item['keys'] if key in data}
        missing = sorted(set(data) - set(selected))
        return f"Source sha256 {item['source_sha256']}; omitted top-level keys {missing}.\n" + json.dumps(selected, sort_keys=True, indent=1) + '\n'
    raise AssertionError(f"unhandled material kind {kind}")


def assemble(config):
    subject(config)
    files, blocks, names = [], [], set()
    for item in config['files']:
        path = item['path']
        assert path not in names and not pathlib.PurePosixPath(path).is_absolute() and '..' not in pathlib.PurePosixPath(path).parts
        names.add(path)
        text = material(item)
        assert all(mark not in text for mark in ('===== BEGIN FILE', '===== END FILE', '===== END OF REVIEW SET'))
        raw = text.encode()
        lines = len(text.splitlines())
        rec = dict(item, sha256=digest(raw), bytes=len(raw), lines=lines)
        files.append(rec)
        numbered = '\n'.join(f'{n:5d}| {line}' for n, line in enumerate(text.splitlines(), 1))
        blocks.append(BEGIN.format(**rec) + '\n' + numbered + '\n' + END.format(path=path))
    brief = bound(config['brief'], config['brief_sha256']).decode()
    inventory = '\n'.join(f"- {item['path']}: {item['role']} ({item['bytes']} bytes; {item.get('kind','full')})" for item in files)
    assert 'PACKET_INDEX' in brief
    brief = brief.replace('PACKET_INDEX', inventory)
    prompt = brief.rstrip() + '\n\n' + '\n\n'.join(blocks) + '\n\n' + SENTINEL.format(count=len(files)) + '\n\nThe complete review set ends here. Return the requested review in this single response.\n'
    raw = prompt.encode()
    manifest = {'module': config['module'], 'prompt_sha256': digest(raw), 'bytes': len(raw), 'files': files,
                'subject_manifest': config['freeze_manifest'], 'subject_manifest_sha256': config['freeze_sha256'],
                'config_sha256': digest(json.dumps(config, sort_keys=True).encode()),
                'builder_sha256': digest(pathlib.Path(__file__).read_bytes())}
    return raw, manifest


def main():
    assert len(sys.argv) in (2, 3) and sys.argv[1] in ('build', 'verify'), 'build|verify [CONFIG]'
    config_path = pathlib.Path(sys.argv[2]).resolve() if len(sys.argv) == 3 else pathlib.Path(__file__).resolve().parent / 'config.json'
    config = json.loads(config_path.read_text())
    prompt, manifest = assemble(config)
    dest = config_path.parent
    if sys.argv[1] == 'build':
        assert not (dest.parent / 'review' / 'launch.json').exists(), 'already attempted review'
        (dest / 'prompt-oneshot.md').write_bytes(prompt)
        (dest / 'PACKET-MANIFEST.json').write_text(json.dumps(manifest, indent=1) + '\n')
    else:
        assert (dest / 'prompt-oneshot.md').read_bytes() == prompt, 'prompt mismatch'
        assert json.loads((dest / 'PACKET-MANIFEST.json').read_text()) == manifest, 'manifest mismatch'
    print(json.dumps({key: manifest[key] for key in ('module', 'prompt_sha256', 'bytes', 'subject_manifest_sha256')}))


if __name__ == '__main__':
    main()
