"""Revision7 modular review harness, adapted from sealed P1.35 isolation/capture mechanics."""
import datetime, glob, hashlib, json, os, shutil, subprocess, sys

W = '/home/bline/cbm-work/pi-accounting-sprint-20261113/r7/reviews/module-B'
SID = sys.argv[1]
ST = os.path.expanduser(f'~/.local/state/work-engine/role-host/pi/{SID}')
RAW, SEALED = f'{W}/review/raw', f'{W}/review/sealed'
PROMPT = '/home/bline/cbm-work/pi-accounting-sprint-20261113/r7/reviews/module-B/packet/prompt-oneshot.md'


def sha(p): return hashlib.sha256(open(p, 'rb').read()).hexdigest()


assert not os.path.exists(f'{SEALED}/SEAL.sha256'), 'already sealed'
copies = {'launch.json': f'{W}/review/launch.json', 'launch.stderr': f'{W}/review/launch.stderr',
          'spec.json': f'{ST}/host/turn-1.spec.json', 'admission.json': f'{ST}/host/turn-1.admission.json',
          'runner.jsonl': f'{ST}/host/turn-1.runner.jsonl', 'runner.log': f'{ST}/host/turn-1.runner.log',
          'proxy.jsonl': f'{ST}/host/turn-1.proxy.jsonl', 'events.jsonl': f'{ST}/container/turn-1.events.jsonl',
          'stderr': f'{ST}/container/turn-1.stderr', 'exit': f'{ST}/container/turn-1.exit',
          'delivered-prompt': f'{ST}/container/turn-1.prompt', 'tier.jsonl': f'{ST}/container/turn-1.tier.jsonl'}
for i, s in enumerate(sorted(glob.glob(f'{ST}/container/sessions/*.jsonl'))):
    copies[f'pi-session-{i}.jsonl'] = s
present = {}
for name, src in copies.items():
    if os.path.exists(src):
        shutil.copyfile(src, f'{RAW}/{name}'); present[name] = sha(f'{RAW}/{name}')
# session-dir listing digest (names and sizes, no content of auth files)
listing = sorted((os.path.relpath(os.path.join(d, f), ST), os.path.getsize(os.path.join(d, f))) for d, _, fs in os.walk(ST) for f in fs)
json.dump(listing, open(f'{RAW}/session-dir-listing.json', 'w'), indent=0)

# final assistant text and model identity from Pi events
texts, models, stops, errors, meta = [], set(), [], [], []
for l in open(f'{RAW}/events.jsonl'):
    try: e = json.loads(l)
    except ValueError: continue
    if e.get('type') == 'message_end':
        m = e.get('message') or {}
        if m.get('role') == 'assistant':
            models.add(f"{m.get('provider')}/{m.get('model')}")
            stops.append(m.get('stopReason')); errors.append(m.get('errorMessage'))
            meta.append({k: m.get(k) for k in ('responseId', 'rawStopReason', 'durationMs', 'thinkingLevel', 'usage')})
            t = ''.join(c.get('text', '') for c in m.get('content') or [] if c.get('type') == 'text')
            texts.append(t)
final = texts[-1] if texts else ''
open(f'{SEALED}/reviewer-response.md', 'w').write(final)
sealed_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
open(f'{SEALED}/SEALED_AT', 'w').write(sealed_at + '\n')
files = [f'sealed/reviewer-response.md'] + [f'raw/{n}' for n in sorted(os.listdir(RAW))]
with open(f'{SEALED}/SEAL.sha256', 'w') as fh:
    for f in files: fh.write(f'{sha(os.path.join(W, "review", f))}  {f}\n')
seal = sha(f'{SEALED}/SEAL.sha256')

# mechanical verification
rows = [json.loads(l) for l in open(f'{RAW}/proxy.jsonl') if l.strip()]
ev = {}
for r in rows: ev[r.get('event')] = ev.get(r.get('event'), 0) + 1
req = [r for r in rows if r.get('event') in ('request', 'upstream_error', 'stream_error')]
launch = json.load(open(f'{RAW}/launch.json'))
tok = json.load(open(os.path.expanduser('~/.codex/auth.json')))['tokens']['access_token']
needles = [tok.encode(), tok.split('.')[2].encode()] if tok.count('.') == 2 else [tok.encode()]
del tok
leak, scanned = [], 0
for root in (ST, f'{W}/run/review-work', RAW, SEALED):
    for d, _, fs in os.walk(root):
        for f in fs:
            try: data = open(os.path.join(d, f), 'rb').read()
            except OSError: continue
            scanned += 1
            if any(n in data for n in needles): leak.append(os.path.join(d, f))
del needles
v = dict(session_id=SID, sealed_at=sealed_at, seal_sha256=seal, response_sha256=sha(f'{SEALED}/reviewer-response.md'),
         response_chars=len(final), assistant_messages=len(texts), assistant_meta=meta, models=sorted(models), stop_reasons=stops, error_messages=errors,
         delivered_prompt_equals_frozen=present.get('delivered-prompt') == sha(PROMPT), prompt_sha256=sha(PROMPT),
         launch_auth_lane=launch.get('auth_lane'), launch_model=launch.get('model'), launch_parent=launch.get('parent_session_id'),
         proxy_event_counts=ev, upstream_requests=[{k: r.get(k) for k in ('at', 'event', 'n', 'http_status', 'model', 'provider', 'auth_lane', 'usage', 'subscription_proxy', 'terminal', 'response_id', 'reason')} for r in req],
         any_usd_or_reservation_rows=any('usd' in r or r.get('event') == 'usd_reserved' for r in rows),
         exit_status=open(f'{RAW}/exit').read().strip() if os.path.exists(f'{RAW}/exit') else None,
         token_leak=dict(files_scanned=scanned, token_found_in=leak), raw_files=present)
json.dump(v, open(f'{W}/review/verification.json', 'w'), indent=1, default=str)
print(json.dumps({k: v[k] for k in ('seal_sha256', 'response_sha256', 'response_chars', 'models', 'stop_reasons', 'error_messages',
                                    'delivered_prompt_equals_frozen', 'launch_auth_lane', 'proxy_event_counts', 'any_usd_or_reservation_rows',
                                    'exit_status', 'token_leak')}, default=str))
