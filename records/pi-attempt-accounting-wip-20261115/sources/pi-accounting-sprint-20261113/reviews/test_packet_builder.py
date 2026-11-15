"""Exact-material and tamper controls for the owner-side packet assembler."""
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile

base = pathlib.Path(__file__).resolve().parent
builder = base / 'build_packet_p136.py'
sha = lambda b: hashlib.sha256(b).hexdigest()
with tempfile.TemporaryDirectory(prefix='packet-control-', dir=base) as name:
    root = pathlib.Path(name)
    source = root / 'subject.txt'
    source.write_text('first\nsecond\n')
    freeze = root / 'freeze.json'
    freeze.write_text(json.dumps({'entries': [{'type': 'file', 'path': source.name, 'sha256': sha(source.read_bytes())}]}))
    brief = root / 'brief.md'
    brief.write_text('Test brief\nPACKET_INDEX\n')
    config = root / 'config.json'
    config.write_text(json.dumps({'module': 'TEST', 'freeze_manifest': str(freeze), 'freeze_sha256': sha(freeze.read_bytes()), 'subject_root': str(root),
        'brief': str(brief), 'brief_sha256': sha(brief.read_bytes()), 'files': [{'path': 'subject.txt', 'source': str(source), 'source_sha256': sha(source.read_bytes()), 'role': 'test material'}]}))
    def run(mode):
        return subprocess.run([sys.executable, '-I', str(builder), mode, str(config)], capture_output=True, text=True)
    assert run('build').returncode == 0
    assert run('verify').returncode == 0
    prompt = root / 'prompt-oneshot.md'
    original = prompt.read_bytes()
    assert original.count(b'===== END OF REVIEW SET: 1 files =====') == 1
    assert b'    1| first' in original and b'    2| second' in original
    prompt.write_bytes(original.replace(b'first', b'wrong'))
    assert run('verify').returncode != 0
    prompt.write_bytes(original)
    source.write_text('changed source\n')
    assert run('verify').returncode != 0
    print('PASS: exact roundtrip, line encoding/end sentinel, prompt tamper rejection, source drift rejection')
