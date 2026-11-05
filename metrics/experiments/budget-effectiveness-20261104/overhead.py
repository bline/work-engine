"""Share of each Codex root's spend on model calls that issued budget/accounting tool calls.
Convention (as in skill-change-analysis-20261030/practice_profile.py): a model call = one
token_usage_record; its tool calls are those issued since the previous record in the same rollout.
Usage: overhead.py LEDGER_CALLS SINCE UNTIL THREAD [THREAD ...]"""
import json, sys, re, subprocess, collections
ledger, SINCE, UNTIL, *threads = sys.argv[1:]
threads = set(threads)
cost = {}; files = collections.defaultdict(set)
for line in open(ledger):
    c = json.loads(line)
    if c['provider'] != 'codex' or c.get('thread') not in threads: continue
    if not (SINCE <= c['ts'] < UNTIL): continue
    cost[c['call_id']] = c.get('speed_adjusted_api_usd_low') or 0
    files[c['thread']].add(c['source'].rsplit(':', 1)[0])
STRICT = re.compile(r"scope_budget\.py['\"]?\s+(init|add|revise|add-session|bind|cost-limit|close|listen|check)\b|usage_ledger\.py['\"]?\s+(tree|control|build)\b|--budget-record")
BROAD = re.compile(r"\[['\"](scope_budget|cost_limits|accounting|budget_observation)['\"]\]|accounting[-_\w]*\.json")
def lines(p):
    if p.endswith('.zst'):
        out = subprocess.run(['zstd', '-dc', p], capture_output=True).stdout.decode('utf-8', 'ignore')
        return out.splitlines()
    return open(p, errors='ignore').read().splitlines()
res = {}
for t in sorted(threads):
    S = collections.Counter()
    for p in files[t]:
        pending = []
        for raw in lines(p):
            try: r = json.loads(raw)
            except Exception: continue
            pl = r.get('payload') if isinstance(r.get('payload'), dict) else {}
            if r.get('type') == 'response_item' and pl.get('type') in ('custom_tool_call', 'function_call'):
                pending.append(str(pl.get('input') or pl.get('arguments') or '')); continue
            if r.get('type') != 'token_usage_record': continue
            rid = pl.get('response_id')
            if rid not in cost: pending = []; continue
            v = cost[rid]; S['usd'] += v; S['calls'] += 1
            txt = '\n'.join(pending); pending = []
            if not txt: S['usd_notool'] += v; continue
            if STRICT.search(txt): S['usd_strict'] += v; S['calls_strict'] += 1
            elif BROAD.search(txt): S['usd_broad_only'] += v; S['calls_broad_only'] += 1
    res[t] = S
    u = S['usd'] or 1
    print(t[:8], 'calls %5d usd %7.2f | strict %5.1f%% (%d calls) | +broad %5.1f%% | no-tool %5.1f%%' % (
        S['calls'], S['usd'], 100 * S['usd_strict'] / u, S['calls_strict'], 100 * S['usd_broad_only'] / u, 100 * S['usd_notool'] / u))
