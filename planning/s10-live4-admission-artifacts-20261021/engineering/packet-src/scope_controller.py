#!/usr/bin/python3
"""STATIC DRAFT: contain the whole live4 runner in a user-systemd scope.

Never run before owner admission and offline detached-child qualification.
The controller remains outside the scope; its receipt is a custody fact, not
product success. It alone makes the accepted stopped DB/WAL copy.
"""
import hashlib, json, os, pathlib, secrets, shutil, subprocess, sys, time

HERE = pathlib.Path(__file__).resolve().parent
TRIAL = HERE.parent
assert sys.argv[1:] in (['--offline-rehearsal'], ['--live-armed'])
mode = sys.argv[1]
assert os.environ.get('S10_LIVE4_ADMISSION') == 'authorized'
if mode == '--live-armed':
    assert os.environ.get('S10_LIVE4_ROOT_LAUNCH') == 'authorized'
unit = 's10-live4-' + secrets.token_hex(6) + '.scope'
state = TRIAL / ('live4-state' if mode == '--live-armed' else 'rehearsal-state')
assert not state.exists(), 'one-shot state already exists'
logs = TRIAL / 'controller-logs' / unit
logs.mkdir(mode=0o700, parents=True, exist_ok=False)
gate = logs / 'scope-gate.json'
sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
started_at = time.time()
run_cmd = ['systemd-run', '--user', '--scope', '--unit=' + unit,
           '--property=KillMode=control-group', '--property=SendSIGKILL=yes',
           '--property=Delegate=no', '--property=RuntimeMaxSec=1230s',
           '--wait', '/usr/bin/python3',
           str(HERE / 'run_semantic.py'), mode]
env = os.environ.copy()
env['S10_LIVE4_SCOPE_GATE'] = str(gate)
# The credential is inherited through a private environment, never an argument.
proc = None
control_group = None
runner_pid = None
runner_starttime = None
members_seen = []
member_observations = {}
commands = []
errors = []
forced = False
quiescent = False
stop_requested = False
final_unit_state = {}

def command(args, timeout=10):
    try:
        r = subprocess.run(args, capture_output=True, text=True, timeout=timeout, check=False)
        commands.append({'argv': args, 'exit_code': r.returncode,
                         'stdout_sha256': hashlib.sha256(r.stdout.encode()).hexdigest(),
                         'stderr_sha256': hashlib.sha256(r.stderr.encode()).hexdigest()})
        return r
    except Exception as e:
        errors.append({'argv': args, 'error': repr(e)})
        return None

def unit_state():
    r = command(['systemctl', '--user', 'show', unit,
                 '--property=ControlGroup', '--property=ActiveState',
                 '--property=Result', '--property=Delegate', '--no-pager'], 5)
    if r is None or r.returncode != 0:
        return {}
    out = {}
    for line in r.stdout.splitlines():
        if '=' in line:
            k, v = line.split('=', 1); out[k] = v
    return out

def member_ids(group):
    p = pathlib.Path('/sys/fs/cgroup') / group.lstrip('/') / 'cgroup.procs'
    if not p.exists():
        return None
    try:
        return sorted(int(x) for x in p.read_text().split())
    except Exception as e:
        errors.append({'path': str(p), 'error': repr(e)})
        return None

def pid_identity(pid):
    try:
        stat = pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        cg = pathlib.Path(f'/proc/{pid}/cgroup').read_text().strip()
        cmd = pathlib.Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
        return {'pid': pid, 'starttime_ticks': int(stat[19]), 'cgroup': cg,
                'argv_basenames': [pathlib.Path(x.decode('utf-8', 'replace')).name for x in cmd if x]}
    except (FileNotFoundError, ProcessLookupError):
        return None

def stop_scope():
    global forced, stop_requested
    if unit_state().get('ActiveState') == 'inactive':
        return
    stop_requested = True
    r = command(['systemctl', '--user', 'stop', unit], 12)
    if r is None or r.returncode != 0:
        forced = True
    end = time.monotonic() + 8
    while time.monotonic() < end:
        if control_group and member_ids(control_group) == []:
            return
        time.sleep(.1)
    forced = True
    command(['systemctl', '--user', 'kill', '--kill-whom=all', '--signal=SIGKILL', unit], 5)

try:
    with (logs / 'systemd-run.stdout.log').open('wb') as stdout, (logs / 'systemd-run.stderr.log').open('wb') as stderr:
        proc = subprocess.Popen(run_cmd, env=env, cwd=TRIAL, stdin=subprocess.DEVNULL,
                                stdout=stdout, stderr=stderr, start_new_session=True)
        gate_deadline = time.monotonic() + 15
        while time.monotonic() < gate_deadline:
            s = unit_state()
            candidate = s.get('ControlGroup', '')
            if candidate.startswith('/user.slice/') and s.get('ActiveState') == 'active' and s.get('Delegate') in ('no', 'false'):
                ids = member_ids(candidate)
                if ids:
                    matches = []
                    for pid in ids:
                        identity = pid_identity(pid)
                        if identity:
                            members_seen.append(identity)
                            if 'run_semantic.py' in identity['argv_basenames'] and identity['cgroup'] == '0::' + candidate:
                                matches.append(identity)
                    if len(matches) == 1:
                        control_group = candidate
                        runner_pid = matches[0]['pid']
                        runner_starttime = matches[0]['starttime_ticks']
                        gate.write_text(json.dumps({'unit': unit, 'control_group': control_group,
                                                    'runner_pid': runner_pid,
                                                    'runner_starttime_ticks': runner_starttime,
                                                    'established_at': time.time()}) + '\n')
                        gate.chmod(0o600)
                        break
            if proc.poll() is not None:
                raise RuntimeError('scope exited before containment identity was established')
            time.sleep(.1)
        assert control_group and runner_pid, 'active nondelegated scope/runner identity not established'
        deadline = time.monotonic() + 1230
        while proc.poll() is None and time.monotonic() < deadline:
            for pid in member_ids(control_group) or []:
                identity = pid_identity(pid)
                if identity:
                    key = str(pid) + ':' + str(identity['starttime_ticks'])
                    item = member_observations.setdefault(key, {'identity': identity,
                        'first_seen_at': time.time(), 'last_seen_at': time.time()})
                    item['last_seen_at'] = time.time()
            time.sleep(.2)
        if proc.poll() is None:
            errors.append({'error': 'external wall deadline exceeded'})
            forced = True
        stop_scope()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            forced = True
            proc.kill(); proc.wait(timeout=5)
finally:
    if proc is not None and proc.poll() is None:
        stop_scope()
        try: proc.wait(timeout=5)
        except subprocess.TimeoutExpired: proc.kill(); proc.wait(timeout=5)
    if control_group:
        end = time.monotonic() + 10
        while time.monotonic() < end:
            s = unit_state(); ids = member_ids(control_group)
            cg_file = pathlib.Path('/sys/fs/cgroup') / control_group.lstrip('/') / 'cgroup.procs'
            # None also means a read error. Only treat it as gone if the file
            # itself has disappeared; a present unreadable cgroup fails closed.
            empty_or_gone = ids == [] or (ids is None and not cg_file.exists())
            if s.get('ActiveState') == 'inactive' and empty_or_gone and not errors:
                quiescent = True; break
            time.sleep(.1)
    final_unit_state = unit_state()
    copies = []; copy_errors = []
    if quiescent and state.exists():
        dest = state / 'stopped-copy-final'
        dest.mkdir(mode=0o700, exist_ok=False)
        for src in (state / 'tasks.sqlite', state / 'tasks.sqlite-wal', state / 'tasks.sqlite-shm',
                    state / 'lifecycle-store/lifecycle.sqlite',
                    state / 'lifecycle-store/lifecycle.sqlite-wal',
                    state / 'lifecycle-store/lifecycle.sqlite-shm'):
            if not src.exists(): continue
            try:
                before = sha(src); target = dest / src.name
                shutil.copy2(src, target)
                after = sha(src)
                assert before == after == sha(target), 'copy race/hash mismatch'
                copies.append({'source': str(src), 'copy': str(target), 'sha256': before})
            except Exception as e:
                copy_errors.append({'source': str(src), 'error': repr(e)})
        if not (dest / 'tasks.sqlite').exists() or not (dest / 'lifecycle.sqlite').exists():
            copy_errors.append({'error': 'required task/lifecycle database absent after quiescence'})
    receipt = {'schema': 's10-live4-scope-custody-v1', 'unit': unit, 'mode': mode,
               'started_at': started_at, 'finished_at': time.time(),
               'control_group': control_group, 'runner_pid': runner_pid,
               'runner_starttime_ticks': runner_starttime,
               'systemd_run_exit': proc.returncode if proc else None,
               'scope_quiescent': quiescent,
               'controller_explicit_escalation': forced,
               'controller_stop_requested': stop_requested,
               'actual_termination_signal_or_systemd_escalation': 'unknown',
               'final_unit_state': final_unit_state,
               'members_seen': members_seen,
               'member_observations': list(member_observations.values()),
               'descendant_exit_codes': 'unavailable unless independently recorded by host; absence is not natural-exit proof',
               'commands': commands,
               'copies': copies, 'copy_errors': copy_errors, 'errors': errors,
               'product_result':'unassessed; runner and semantic evidence remain separate'}
    path = logs / 'scope-receipt.json'
    path.write_text(json.dumps(receipt, indent=2) + '\n')
    path.chmod(0o600)
    print(json.dumps({'unit': unit, 'scope_quiescent': quiescent, 'forced': forced,
                      'receipt': str(path), 'receipt_sha256': sha(path)}))
    if not quiescent or copy_errors or errors:
        sys.exit(2)
