"""Owned event wait; seals a terminal one-shot review before model inspection."""
import ctypes
import json
import os
import pathlib
import select
import subprocess
import sys
import time

module = pathlib.Path(sys.argv[1]).resolve()
launch = json.loads((module / 'review/launch.json').read_text())
sid = launch['session_id']
runner = pathlib.Path(launch['files']['runner_record'])
libc = ctypes.CDLL(None, use_errno=True)
libc.inotify_init1.argtypes = [ctypes.c_int]
libc.inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
fd = libc.inotify_init1(os.O_NONBLOCK | os.O_CLOEXEC)
if fd < 0:
    raise OSError(ctypes.get_errno(), 'inotify_init1')
if libc.inotify_add_watch(fd, os.fsencode(runner), 0x00000002 | 0x00000008) < 0:
    os.close(fd)
    raise OSError(ctypes.get_errno(), 'inotify_add_watch')
deadline = time.monotonic() + 1500
try:
    while True:
        rows = []
        for line in runner.read_text().splitlines():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
        if any(row.get('event') == 'container_removed' for row in rows):
            break
        if time.monotonic() >= deadline:
            raise TimeoutError('terminal runner event not observed; root retains collection')
        ready, _, _ = select.select([fd], [], [], min(60, deadline-time.monotonic()))
        if ready:
            os.read(fd, 4096)
finally:
    os.close(fd)
subprocess.run([sys.executable, '-I', str(module / 'tools/capture_and_seal.py'), sid], check=True)
v = json.loads((module / 'review/verification.json').read_text())
subprocess.run(['python3', 'skills/durable-state/scripts/codex_chatboard.py', '--repository', '.', 'post', '--author', 'codex-pi-accounting-watch', '--session-id', '01a1d6e2-32ba-7701-9115-740dcb41af11', '--topic', 'aspect:pi-accounting', '--wake', '--body', f'Current revision6 review {sid} terminal and sealed before comparison; root retains collection. seal={v["seal_sha256"]}; model={v["models"]}; exit={v["exit_status"]}.', '--ref', str(module / 'review/verification.json')], check=True)
print('terminal review sealed; root must collect and retire', sid, flush=True)
