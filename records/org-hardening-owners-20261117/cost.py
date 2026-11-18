#!/usr/bin/env python3
"""Objective cost proxy: Pi root session (since objective start) + listed Claude child transcripts.
Priced with metrics/scripts/usage_ledger.py RATES (API-equivalent USD; no speed adjustment for Claude)."""
import json, sys, glob, os
sys.path.insert(0, '/home/bline/code/work-engine/metrics/scripts')
import usage_ledger as ul

N = os.path.dirname(os.path.abspath(__file__))
START = '2026-11-17T20:06:09'
# Root attribution: root cost until the step-3 objective started (20:57:43Z) plus the fixed $0.90 share of later root
# coordination already reported to the operator; all later root cost is attributed to org-hosted-review-20261117.
END = '2026-11-17T20:57:43'
ROOT_SHARE_AFTER = 0.90
ROOT = '/home/bline/.pi/agent/sessions/--home-bline-code-work-engine--/2026-11-16T17-36-42-515Z_01a1e56e-be12-7424-977b-c7c7983887d0.jsonl'
rates = {**getattr(ul, 'RATES', {})}

def rate(model):
    for name in ('CLAUDE_RATES', 'RATES', 'NATIVE_RATES'):
        t = getattr(ul, name, None)
        if isinstance(t, dict) and model in t:
            return t[model]
    raise SystemExit(f'no rate for {model}')

def price(model, inp, out, cr, cw5, cw1):
    r = rate(model)
    return (inp * r['input'] + out * r['output'] + cr * r['cache_read'] + cw5 * r['cache_write_5m'] + cw1 * r['cache_write_1h']) / 1e6

def root_cost():
    tot = 0.0; n = 0
    for l in open(ROOT):
        o = json.loads(l)
        m = o.get('message') or {}
        if m.get('role') != 'assistant' or not m.get('usage'):
            continue
        ts = o.get('timestamp') or ''
        if ts[:19] < START or ts[:19] >= END:
            continue
        u = m['usage']; model = (m.get('model') or 'claude-opus-5-5').split('/')[-1]
        cw1 = u.get('cacheWrite1h', 0) or 0; cw5 = max((u.get('cacheWrite', 0) or 0) - cw1, 0)
        tot += price(model, u.get('input', 0), u.get('output', 0), u.get('cacheRead', 0), cw5, cw1); n += 1
    return tot, n

SEEN_IDS = set()

def claude_cost(path):
    msgs = {}
    for l in open(path):
        try: o = json.loads(l)
        except Exception: continue
        if o.get('type') != 'assistant': continue
        m = o['message']; u = m.get('usage') or {}
        if not u or m.get('model') in (None, '<synthetic>'): continue
        mid = m.get('id') or o.get('uuid')
        prev = msgs.get(mid)
        if prev is None or (u.get('output_tokens', 0) >= prev[1].get('output_tokens', 0)):
            msgs[mid] = (m['model'], u)
    tot = 0.0
    for mid in list(msgs):
        if mid in SEEN_IDS: del msgs[mid]
    SEEN_IDS.update(msgs)
    for model, u in msgs.values():
        cc = u.get('cache_creation') or {}
        cw1 = cc.get('ephemeral_1h_input_tokens', 0); cw5 = cc.get('ephemeral_5m_input_tokens', u.get('cache_creation_input_tokens', 0) - cw1)
        tot += price(model, u.get('input_tokens', 0), u.get('output_tokens', 0), u.get('cache_read_input_tokens', 0), cw5, cw1)
    return tot, len(msgs)

children = [l.split()[0] for l in open(os.path.join(N, 'children.txt')) if l.strip() and not l.startswith('#')]
out = {'root': None, 'children': {}, 'total': 0.0}
c, n = root_cost(); c += ROOT_SHARE_AFTER; out['root'] = {'usd': round(c, 4), 'calls': n, 'window_end': END, 'share_after': ROOT_SHARE_AFTER}; out['total'] += c
for sid in children:
    fs = glob.glob(f'/home/bline/.claude/projects/*/{sid}.jsonl')
    if not fs:
        out['children'][sid] = 'transcript_missing'; continue
    c, n = claude_cost(fs[0]); out['children'][sid] = {'usd': round(c, 4), 'calls': n}; out['total'] += c
out['total'] = round(out['total'], 4); out['limit'] = 32
print(json.dumps(out, indent=1))
