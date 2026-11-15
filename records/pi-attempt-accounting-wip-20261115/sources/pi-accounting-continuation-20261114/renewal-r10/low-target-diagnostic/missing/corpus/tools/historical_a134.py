"""Comparison only: run every corpus case's inputs through the frozen A1.3.4 reader (086bdec1) and store its outcome in
`historical.a134_086bdec1`. Never used to derive or change an expectation. Run gated (env -i, dead DOCKER_HOST).
Usage: historical_a134.py <corpus_dir> <a134 reader path>"""
import datetime, glob, hashlib, importlib.util, json, os, sys, tempfile
from decimal import Decimal
from unittest import mock

corpus, reader = sys.argv[1], sys.argv[2]
spec = importlib.util.spec_from_file_location('L', reader); L = importlib.util.module_from_spec(spec); spec.loader.exec_module(L)
DIGEST = hashlib.sha256(open(reader, 'rb').read()).hexdigest()
assert DIGEST.startswith('086bdec1'), DIGEST


def usd(n): return float(Decimal(n) / 10 ** 9)


def run(case, ev):
    with tempfile.TemporaryDirectory() as d:
        state = os.path.join(d, 'state'); os.makedirs(state); jdir = os.path.join(d, 'locks'); os.makedirs(jdir)
        rows = []
        rec_paths = {o['objective']: os.path.join(d, f'record-{o["objective"]}.json') for o in ev['objectives']}
        for s in case['inputs']['sessions']:
            sid = s['session_id']; host = os.path.join(state, 'pi', sid, 'host'); os.makedirs(host)
            paths = {}
            for turn, log in s['logs'].items():
                p = os.path.join(host, f'{turn}.proxy.jsonl'); paths[turn] = p
                open(p, 'w').write('\n'.join(log['lines']) + ('\n' if log['final_newline'] and log['lines'] else ''))
            for name, log in (s.get('extra_files') or {}).items():
                open(os.path.join(host, name), 'w').write('\n'.join(log['lines']) + ('\n' if log['final_newline'] else ''))
            env = {}
            if s.get('ignored_sources', {}).get('pi-session-file'):
                sd_ = os.path.join(d, 'sessions'); os.makedirs(sd_, exist_ok=True)
                f = s['ignored_sources']['pi-session-file']
                open(os.path.join(sd_, f'x_{sid}.jsonl'), 'w').write(json.dumps({'type': 'message', 'message': {'role': 'assistant', 'responseId': f['responseId'], 'usage': {'cacheWrite': f['cacheWrite']}}}) + '\n')
                env = {'container_environment': {'session_dir': sd_}}
            for lc in s['lifecycle']:
                r = {'host': 'pi', 'event': lc['event'], 'session_id': sid}
                if lc.get('at'): r['at'] = lc['at']
                if lc['event'] in ('launch', 'deliver'):
                    r.update(turn=lc['turn'], files={'proxy_log': paths.get(f'turn-{lc["turn"]}', os.path.join(host, f'turn-{lc["turn"]}.proxy.jsonl'))}, thinking='high')
                    if s['launch']: r.update(model=s['launch']['model'], auth_lane=s['launch']['auth_lane'], **env)
                    if lc['event'] == 'launch': r['parent_session_id'] = (s['launch'] or {}).get('parent', 'root')
                rows.append(r)
            if s['launch'] is None:
                rows.append({'host': 'pi', 'event': 'deliver', 'session_id': sid, 'turn': 1, 'files': {'proxy_log': paths.get('turn-1')}, 'at': '2026-11-10T06:00:10Z'})
        for o in ev['objectives']:
            rp = rec_paths[o['objective']]
            sb = dict(objective_id=o['objective'], started_at=o['started_at'], bindings=[], sessions=[dict(session_id='root', provider='codex', **{'from': o['started_at']})],
                      cost_limits=[dict(pool='pi_api', node=None, max_usd=usd(o['limit']), marks_usd=[], set_by='user')])
            if o['closed_at']: sb['closed_at'] = o['closed_at']
            json.dump(dict(scope_budget=sb), open(rp, 'w'))
            jp = None
            with mock.patch.dict(os.environ, {'ROLE_HOST_OBJECTIVE_LOCK_DIR': jdir}):
                jp = L.admission_journal_path(rp)
            for a in o['admissions']:
                for r in a['rows']:
                    if r['event'] == 'objective_admission':
                        rows.append({'host': 'pi', 'event': 'objective_admission', 'session_id': r['session_id'], 'objective_record': os.path.abspath(rp), 'max_usd': r['max_usd'], 'at': r['at']})
                    else:
                        with open(jp, 'a') as fh: fh.write(json.dumps({**r, 'state_ledger': os.path.join(state, 'ledger.jsonl')}) + '\n')
        led = os.path.join(state, 'ledger.jsonl')
        open(led, 'w').write(''.join(json.dumps(r) + '\n' for r in rows))
        ref = datetime.datetime.fromisoformat(case['inputs']['reference_time'].replace('Z', '+00:00'))
        class Frozen(datetime.datetime):
            @classmethod
            def now(cls, tz=None): return ref.astimezone(tz) if tz else ref.replace(tzinfo=None)
        req = ev['admission_request']; real = L.transcript_available
        ps = [mock.patch.object(L, 'build_codex', return_value=([], [], [], 0)), mock.patch.object(L, 'build_claude', return_value=([], [], 0)),
              mock.patch.object(L, 'unrecorded_claude', return_value=[]), mock.patch.object(L.dt, 'datetime', Frozen),
              mock.patch.object(L, 'transcript_available', side_effect=lambda p, s_, *a, **kw: True if s_ == 'root' else real(p, s_, *a, **kw)),
              mock.patch.dict(os.environ, {'ROLE_HOST_OBJECTIVE_LOCK_DIR': jdir})]
        for p in ps: p.start()
        try:
            L.ISSUES.clear()
            try:
                out = L.pi_admission(rec_paths[req['objective']], usd(req['new']), 'root', led, None)
                res = {'decision': out['decision'], 'spent_high_nano': None if out.get('spent_high_usd') is None else str(round(out['spent_high_usd'] * 10 ** 9)),
                       'spent_low_nano': None if out.get('spent_low_usd') is None else str(round(out['spent_low_usd'] * 10 ** 9)),
                       'blocking_kinds': sorted({i['kind'] for i in out.get('blocking_issues') or []}), 'reasons': out.get('reasons')}
            except Exception as e:  # noqa: BLE001  (a reader crash is itself a historical outcome)
                res = {'decision': 'raised', 'error': f'{type(e).__name__}: {str(e)[:120]}'}
            L.ISSUES.clear()
        finally:
            for p in ps: p.stop()
        return res


n = 0
for path in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json'))):
    c = json.load(open(path))
    hist = c.get('historical') or {}
    hist['a134_086bdec1'] = {'reader_sha256': DIGEST, 'evaluation': c['evaluations'][0]['name'], **run(c, c['evaluations'][0]),
                             'note': 'comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window'}
    c['historical'] = hist
    json.dump(c, open(path, 'w'), indent=1, sort_keys=True); open(path, 'a').write('\n'); n += 1
print(f'{n} cases annotated with the A1.3.4 comparison')
