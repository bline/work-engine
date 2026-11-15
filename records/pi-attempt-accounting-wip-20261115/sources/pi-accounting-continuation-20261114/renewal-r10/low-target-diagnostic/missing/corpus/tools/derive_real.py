"""D8: derive the sanitised, local-only real-record corpus from the role_host ledger and every proxy log it names.
Raw logs stay where they are; nothing here is for the repository. Each sanitised file records its source digest.
Sanitisation (field inventory in P1.28 report):
  - response_id -> 'resp_' + sha256(original)[:24]  (stable, so duplicate identity is preserved)
  - input_basis -> {'items': <count>}  (prompt-chain digests removed)
  - detail -> '[redacted]'  (key kept: its presence is the stream_error shape)
  - role_host rows keep only event, session_id, turn, at, model, auth_lane, parent_session_id, usd_limit, thinking;
    files.proxy_log becomes the relative sanitised log name; cwd, argv, pid and environment are dropped;
    objective_record becomes 'objective:' + sha256(path)[:16]
Also records, per session, exact producer-field sums and the frozen A1.3.4 per-session outcome (comparison only).
Usage: derive_real.py <out_dir> <a134 reader path>"""
import collections, hashlib, importlib.util, json, os, sys
from decimal import Decimal

out, reader = sys.argv[1], sys.argv[2]
STATE = os.path.expanduser('~/.local/state/work-engine')
LEDGER = os.path.join(STATE, 'role-host', 'ledger.jsonl')
HOME = os.path.expanduser('~')
sha = lambda b: hashlib.sha256(b).hexdigest()
KEEP = ('host', 'event', 'session_id', 'turn', 'at', 'model', 'auth_lane', 'parent_session_id', 'usd_limit', 'thinking', 'max_usd')


def clean_row(p):
    p = dict(p)
    if isinstance(p.get('response_id'), str): p['response_id'] = 'resp_' + sha(p['response_id'].encode())[:24]
    if isinstance(p.get('input_basis'), dict): p['input_basis'] = {'items': p['input_basis'].get('items')}
    if 'detail' in p and p['detail'] is not None: p['detail'] = '[redacted]'
    return p


ledger_raw = open(LEDGER, 'rb').read()
rows = [json.loads(l) for l in ledger_raw.decode().splitlines() if l.strip()]
pi = [r for r in rows if r.get('host') == 'pi']
os.makedirs(os.path.join(out, 'sessions'), exist_ok=True)
manifest = {'derived_from': {'ledger': LEDGER.replace(HOME, '~'), 'ledger_sha256': sha(ledger_raw), 'pi_rows': len(pi)},
            'sanitisation': __doc__.split('Sanitisation')[1].split('Also records')[0].strip(), 'sources': [], 'sessions': {}}
clean_ledger = []
for r in pi:
    c = {k: r[k] for k in KEEP if k in r}
    log = (r.get('files') or {}).get('proxy_log')
    if log:
        c['files'] = {'proxy_log': f'sessions/{r["session_id"]}/turn-{r.get("turn")}.proxy.jsonl'}
        raw = open(log, 'rb').read()
        lines = raw.decode().split('\n')
        final_nl = raw.endswith(b'\n')
        clean = []
        for ln in lines[:-1] if final_nl else lines:
            try: clean.append(json.dumps(clean_row(json.loads(ln))) if ln.strip() else ln)
            except ValueError: clean.append('[unparseable line withheld]')
        dst = os.path.join(out, c['files']['proxy_log']); os.makedirs(os.path.dirname(dst), exist_ok=True)
        body = ('\n'.join(clean) + ('\n' if final_nl else '')).encode(); open(dst, 'wb').write(body)
        ev = collections.Counter(); charged = reserved = Decimal(0)
        for ln in clean:
            try: p = json.loads(ln)
            except ValueError: continue
            ev[p.get('event')] += 1
            if isinstance(p.get('usd'), dict) and isinstance(p['usd'].get('charged_usd'), (int, float)): charged += Decimal(str(p['usd']['charged_usd']))
            if p.get('event') == 'usd_reserved' and isinstance(p.get('reserved_usd'), (int, float)): reserved += Decimal(str(p['reserved_usd']))
        manifest['sources'].append({'source': log.replace(HOME, '~'), 'source_sha256': sha(raw), 'sanitised': c['files']['proxy_log'],
                                    'sanitised_sha256': sha(body), 'lines': len(clean), 'final_newline': final_nl, 'events': dict(ev),
                                    'sum_charged_usd': str(charged), 'sum_reserved_usd': str(reserved)})
    if 'objective_record' in r: c['objective_record'] = 'objective:' + sha(r['objective_record'].encode())[:16]
    clean_ledger.append(c)
open(os.path.join(out, 'ledger.jsonl'), 'w').write(''.join(json.dumps(x) + '\n' for x in clean_ledger))
# frozen A1.3.4 per-session outcome on the *raw* evidence (comparison only)
spec = importlib.util.spec_from_file_location('L', reader); L = importlib.util.module_from_spec(spec); spec.loader.exec_module(L)
L.ISSUES.clear(); calls, cov = L.build_pi(None, None, state=os.path.dirname(LEDGER), ledger=LEDGER)
per = collections.defaultdict(lambda: {'calls': 0, 'pi_api_high_nano': 0, 'pi_api_low_nano': 0, 'codex_calls': 0, 'unpriced': 0})
for cl in calls:
    s = per[cl['thread']]; s['calls'] += 1
    sp = L.call_spend(cl)
    if 'pi_api' in cl['pools']:
        s['pi_api_high_nano'] += round(sp['quota_usd_high'] * 10 ** 9); s['pi_api_low_nano'] += round(sp['quota_usd_low'] * 10 ** 9)
        s['unpriced'] += sp['unpriced_calls']
    else: s['codex_calls'] += 1
issues = collections.defaultdict(list)
for i in L.ISSUES: issues[i.get('session_id')].append(i['kind'])
for sid in {r['session_id'] for r in pi}:
    manifest['sessions'][sid] = {'a134_086bdec1': {**per.get(sid, {'calls': 0}), 'issue_kinds': sorted(set(issues.get(sid, [])))},
                                 'normative': None,
                                 'ambiguity': 'Normative per-session expectations are established in S3 (differential against A1.3.4 with every difference classified and reviewed); S1 does not invent them.'}
manifest['a134_totals'] = {'calls': len(calls), 'coverage': cov, 'issue_kinds': sorted({i['kind'] for i in L.ISSUES}),
                           'reader_sha256': sha(open(reader, 'rb').read())}
json.dump(manifest, open(os.path.join(out, 'MANIFEST.json'), 'w'), indent=1, sort_keys=True)
open(os.path.join(out, 'README.md'), 'w').write('# Sanitised real-record corpus (local-only)\n\nDerived under owner decision D8. Do not commit or upload. '
                                               'Raw sources remain in ~/.local/state/work-engine; MANIFEST.json maps every sanitised file to its source digest.\n')
print(f"{len(manifest['sources'])} logs, {len(pi)} role_host Pi rows, {len(manifest['sessions'])} sessions, A1.3.4 calls {len(calls)}")
