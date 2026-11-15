"""Revision6 module C review harness, adapted from sealed P1.35 isolation/capture mechanics."""
import argparse, base64, datetime, glob, hashlib, json, os, sys, time

W = '/home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/module-C'
PK = '/home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/module-C/packet'
PROMPT = f'{PK}/prompt-oneshot.md'
PROMPT_SHA = 'NOT_YET_BOUND_BLOCK_LAUNCH'
REL = '/home/bline/.local/share/work-engine/skill-releases/releases/92a03789327777b3cd9b5822c6df75a18fa63660'
SCRIPTS = f'{REL}/skills/manual-orchestration/scripts'
EXPECT = {'role_host.py': 'cbca1bc9656ed6de', 'pi_container.py': '780837341790780f', 'pi_container_entry.py': 'b75c71508ec12587'}
STEP_D_COORDINATOR = '7c0cdefa-513d-4f30-9bd4-7575bebdeb52'
RETIRED_ROOTS = {'c110c95a-0e4b-4b0b-9f6e-b43c3e815972', '8e2de292-f1a3-47ca-b46a-9fa0065e3257', '6e6925af-1e4a-41a0-806e-8c8cf519a811',
                 '81c87c1d-b828-4bd4-8d1d-feeba0320e8e', '6e30b9f3-ea82-4ab4-8b5d-dea95015f0e0',
                 'd8343b51-f163-42d1-8eb7-d36fe5813f73', 'e7fa73cd-d2d3-4aeb-87ab-36408b57e681',
                 '4a933f58-d867-4254-b7dd-d2da5ba3c20a', 'b7a05133-14ba-47d5-99d1-4d707f8de1fd',
                 'ef83a261-61c2-476b-b751-348cc00544ad', 'd38ddd60-0a3f-4b0d-bd3c-a1526be3f860', 'fcfff5f8-2246-4a88-9016-da9aa2919620', '74d1a800-e670-4ea0-856b-41f8baf5b126'}   # P1.13 ... P1.25, P1.29, P1.32, P1.34 roots
FORBIDDEN_ENV = ('OPENAI_API_KEY', 'ANTHROPIC_API_KEY', 'OPENAI_CODEX_API_KEY', 'ROLE_HOST_PI_UPSTREAM_CODEX', 'ROLE_HOST_PI_UPSTREAM',
                 'ROLE_HOST_PI_UPSTREAM_ANTHROPIC', 'PYTHONPATH', 'PYTHONHOME', 'PYTHONSTARTUP', 'ROLE_HOST_PI_RUNTIME', 'ROLE_HOST_DOCKER_EXE',
                 'ROLE_HOST_USAGE_LEDGER')
STATE = os.path.expanduser('~/.local/state/work-engine')

a = argparse.ArgumentParser(); a.add_argument('--phase', choices=('oneshot',), required=True); a.add_argument('--out', required=True)
a.add_argument('--as-session'); a.add_argument('--whoami')
args = a.parse_args()
proof = dict(phase=args.phase, at=datetime.datetime.now(datetime.timezone.utc).isoformat(), checks={})


def fail(msg):
    proof['refused'] = msg; json.dump(proof, open(args.out, 'w'), indent=1, default=str); print('refused:', msg); sys.exit(1)


def sha16(p): return hashlib.sha256(open(p, 'rb').read()).hexdigest()[:16]


# ---- identity: a fresh, parentless, single-purpose claude root with no other rows and no children
me = json.loads(args.whoami) if args.whoami else {'host': 'claude', 'session_id': args.as_session}
proof['caller'] = me
sid = me.get('session_id')
if me.get('host') != 'claude': fail('caller is not a claude session')
if sid in {STEP_D_COORDINATOR, '01a1d6e2-32ba-7701-9115-740dcb41af11'}: fail('caller is the coordinator session bound to Plan IR Step D')
if sid in RETIRED_ROOTS: fail('caller is a retired earlier review root')
rows = []
for _ in range(30 if args.whoami else 1):   # role_host appends the root's launch row just after `claude --bg` returns
    rows = [json.loads(l) for l in open(os.path.join(STATE, 'role-host/ledger.jsonl')) if sid and sid in l]
    if rows: break
    time.sleep(1)
own = [r for r in rows if r.get('session_id') == sid]
children = [r for r in rows if r.get('parent_session_id') == sid]
proof['checks']['role_host_rows_of_caller'] = own; proof['checks']['children'] = children
launch = [r for r in own if r.get('event') == 'launch']
if len(launch) != 1 or launch[0].get('host') != 'claude' or launch[0].get('parent_session_id') is not None:
    fail('role_host ledger does not show exactly one parentless claude launch row for this session')
if len(own) != 1 or children: fail('caller is not a fresh root (other rows or children exist)')
if os.path.exists(f'{W}/review/launch.json'): fail('a reviewer launch was already attempted')

# ---- not inside any open objective's sessions, bindings or (as a node) ancestry
records = set(l.split()[0] for l in open(f'{W}/isolation/open-records.txt') if l.strip())
for j in glob.glob(os.path.join(STATE, 'pi-objective-admissions/*.admissions.jsonl')):
    for l in open(j):
        try: r = json.loads(l)
        except ValueError: continue
        if r.get('objective_record'): records.add(r['objective_record'])
binders = []
for p in sorted(records):
    try: sb = json.load(open(p)).get('scope_budget', {})
    except (OSError, ValueError): continue
    if sb.get('closed_at'): continue
    ids = [s.get('session_id') for s in sb.get('sessions') or []] + [b.get('session_id') for b in sb.get('bindings') or []] \
        + [b.get('parent_session_id') for b in sb.get('bindings') or []]
    if sid in ids: binders.append(p)
proof['checks']['open_records_scanned'] = sorted(records); proof['checks']['records_binding_caller'] = binders
if binders: fail(f'caller already bound in {binders}')

# ---- environment, release, host imports, credential metadata
present = [k for k in FORBIDDEN_ENV if k in os.environ]
proof['checks']['forbidden_env_present'] = present
if present: fail(f'forbidden variables in the launch environment: {present}')
got = {f: sha16(os.path.join(SCRIPTS, f)) for f in EXPECT}
proof['checks']['release_files'] = got
if got != EXPECT: fail('installed release files differ from 92a03789')
if os.path.realpath(os.path.expanduser('~/.local/share/work-engine/skill-releases/current')) != REL: fail('current release is not 92a03789')
import subprocess
probe = subprocess.run(['/usr/bin/python3', '-I', '-c', 'import sys,zstandard,os;print(sys.executable);print(zstandard.__file__);'
                        'print("\\n".join(p for p in sys.path if os.path.exists(p)))'], capture_output=True, text=True, env={})
lines = probe.stdout.split('\n'); zfile = lines[1] if len(lines) > 1 else ''
roots = [p for p in lines[2:] if p]
bad = [r for r in roots if os.stat(r).st_uid != 0 or os.access(r, os.W_OK)]   # each import root: root-owned, not writable by this user
proof['checks']['host_python'] = dict(zstandard=zfile, import_roots=roots, writable_or_nonroot_roots=bad)
if not zfile.startswith('/usr/lib/python3/dist-packages/zstandard/') or os.stat(os.path.dirname(zfile)).st_uid != 0 or bad:
    fail('host Python import roots or zstandard are not root-owned and unwritable')
shadows = [p for r in roots for p in glob.glob(os.path.join(r, 'zstandard*')) if not p.startswith('/usr/lib/python3/dist-packages/')]
proof['checks']['zstandard_shadows'] = shadows
if shadows: fail(f'zstandard shadow candidates in host import roots: {shadows}')
try:
    rec = json.load(open(os.path.expanduser('~/.codex/auth.json'))); tok = (rec.get('tokens') or {}).get('access_token') or ''
    parts = tok.split('.'); claims = json.loads(base64.urlsafe_b64decode(parts[1] + '=' * (-len(parts[1]) % 4))) if len(parts) == 3 else {}
    exp = claims.get('exp'); del tok, parts
    cred = dict(auth_mode=rec.get('auth_mode'), iss_ok=claims.get('iss') == 'https://auth.openai.com',
                account_claim_present=bool((claims.get('https://api.openai.com/auth') or {}).get('chatgpt_account_id')),
                expires=datetime.datetime.fromtimestamp(exp, datetime.timezone.utc).isoformat() if isinstance(exp, (int, float)) else None,
                hours_left=round((exp - time.time()) / 3600, 1) if isinstance(exp, (int, float)) else None)
    del claims, rec
except Exception as e:
    cred = dict(error=type(e).__name__)
proof['checks']['credential_metadata'] = cred
if cred.get('auth_mode') != 'chatgpt' or not cred.get('iss_ok') or not cred.get('account_claim_present') or (cred.get('hours_left') or 0) < 2:
    fail('host Codex credential not ready (mode/issuer/account/expiry > 2 h)')
if True:   # one-shot packet
    v = subprocess.run(['/usr/bin/python3', '-I', f'{PK}/build_packet_p136.py', 'verify'], capture_output=True, text=True, env={})
    got_sha = hashlib.sha256(open(PROMPT, 'rb').read()).hexdigest()
    pk = dict(prompt_sha256=got_sha, verify_rc=v.returncode, verify_tail=v.stdout.strip().splitlines()[-1:] if v.stdout else [])
    proof['checks']['packet'] = pk
    if v.returncode != 0 or got_sha != PROMPT_SHA: fail('one-shot prompt does not verify or differs from the pinned digest')
json.dump(proof, open(args.out, 'w'), indent=1, default=str)
print('isolation and preflight checks passed for', sid, 'phase', args.phase)
