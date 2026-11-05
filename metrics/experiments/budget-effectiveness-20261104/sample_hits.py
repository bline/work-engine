import json, sys, re, random, subprocess
ledger, SINCE, UNTIL, seed, n, *threads = sys.argv[1:]
threads=set(threads); cost={}; files=set()
for line in open(ledger):
    c=json.loads(line)
    if c['provider']!='codex' or c.get('thread') not in threads or not (SINCE<=c['ts']<UNTIL): continue
    cost[c['call_id']]=c.get('speed_adjusted_api_usd_low') or 0; files.add(c['source'].rsplit(':',1)[0])
from overhead_re import STRICT
hits=[]
for p in sorted(files):
    raw=open(p,errors='ignore').read().splitlines() if not p.endswith('.zst') else subprocess.run(['zstd','-dc',p],capture_output=True).stdout.decode('utf-8','ignore').splitlines()
    pending=[]
    for l in raw:
        try: r=json.loads(l)
        except Exception: continue
        pl=r.get('payload') if isinstance(r.get('payload'),dict) else {}
        if r.get('type')=='response_item' and pl.get('type') in ('custom_tool_call','function_call'):
            pending.append(str(pl.get('input') or pl.get('arguments') or '')); continue
        if r.get('type')!='token_usage_record': continue
        rid=pl.get('response_id'); txt='\n'.join(pending); pending=[]
        if rid in cost and STRICT.search(txt): hits.append((r.get('timestamp'), cost[rid], txt))
random.seed(int(seed))
for ts,v,txt in random.sample(hits, min(int(n),len(hits))):
    print('=====', ts, '$%.3f'%v, 'len', len(txt)); print(txt[:900].replace('\\n','\n'))
