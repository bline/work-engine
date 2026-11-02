"""Local test double. Not an installed provider or a production launch path."""
import hashlib,json,os,signal,subprocess,sys,time
from pathlib import Path
root=Path(__file__).parent
cfgpath=root/'config.json'
cfg=json.loads(cfgpath.read_text())
def save():cfgpath.write_text(json.dumps(cfg))
def arg(k,default=None):
 try:return sys.argv[sys.argv.index(k)+1]
 except ValueError:return default
def count(k):cfg[k]=cfg.get(k,0)+1;save()
def out(v):print(json.dumps(v))
name=Path(__file__).name
if name=='role_host.py':
 if 'prepare' in sys.argv:
  count('creates');time.sleep(cfg.get('prepare_delay',0))
  if cfg.get('crash_create'):os.kill(os.getppid(),signal.SIGKILL);time.sleep(1)
  if cfg.get('unknown_create'):sys.exit(32)
  out({'session_id':cfg['sid'],'realized':{'sandbox':{'type':'readOnly'}} if not cfg.get('no_grant') else {}})
 elif 'start-prepared' in sys.argv or 'deliver' in sys.argv:
  count('starts')
  Path(cfg['rollout']).write_text(json.dumps({'type':'session_meta','payload':{'id':cfg['sid']}})+'\n')
  if not cfg.get('missing_result'):
   doc={}
   if cfg.get('with_artifact'):
    p=Path(cfg['result']).parent/'output.txt';p.write_text('actual output')
    doc={'artifacts':[{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}]}
   Path(cfg['result']).write_text(json.dumps(doc))
  if cfg.get('crash_start'):os.kill(os.getppid(),signal.SIGKILL);time.sleep(1)
  if cfg.get('unknown_start'):sys.exit(32)
  out({'turn_id':'turn-'+str(cfg['starts'])})
 elif 'identity' in sys.argv:
  if not Path(cfg['rollout']).exists():sys.exit(32)
  out({'session_id':cfg['sid'],'rollout':cfg['rollout']})
 elif 'exact-turn' in sys.argv:
  count('turn_reads')
  if cfg.get('observer_fail'):sys.exit(32)
  out({'session_id':cfg['sid'],'turn_id':arg('--turn-id'),'status':cfg.get('turn_status','completed'),'turn':{'itemsView':'full','items':[{'id':'native-final','type':'agentMessage','phase':'final_answer','text':json.dumps({'native':True})}]}})
 elif 'retire' in sys.argv:out({'retired':'idle_released'})
 elif 'settings' in sys.argv:
  out({'session_id':cfg['sid'],'turn_id':arg('--turn-id'),'turn_attribution':'exact_turn_interval' if arg('--turn-id') and not cfg.get('stale_settings') else 'latest_session_observation','model':'fixture-model','reasoning_effort':'medium','service_tier':cfg.get('observed_tier','default')})
elif name=='child_context.py':
 if cfg.get('context_fail') or (arg('--session')==cfg['sid'] and not Path(cfg['rollout']).exists()):out({'status':'unavailable'})
 else:out({'status':'observed','recommendation':'retain_ok','input_tokens':100,'epoch':0})
elif name=='usage_ledger.py':
 if cfg.get('budget_fail'):sys.exit(32)
 out({'schema':'objective-budget-control-v1','topic':'control:budget:objective','closed_at':None,'events':[],
      'next_budget_cursor':'stable-cursor','snapshot':{'status':'complete','cost_controls':[{'max_usd':10,'remaining_usd':cfg.get('headroom',9)}],'totals':{'codex':{'quota_usd_low':1,'quota_usd_high':1}},'post_close':{'candidate_totals':cfg.get('tail_candidate',0),'settlement':{'status':'not_established'}}}})
elif name=='scope_budget.py':
 p=Path(arg('--record'));d=json.loads(p.read_text())
 if 'bind' in sys.argv:
  count('bindings');d['scope_budget']['bindings'].append({'session_id':arg('--session'),'parent_session_id':arg('--parent'),'provider':'codex'})
 if 'close' in sys.argv:count('closes');d['scope_budget']['closed_at']='closed';d['scope_budget']['close']={'reason':arg('--reason'),'disposition':arg('--disposition')}
 p.write_text(json.dumps(d));out({'stop':False,'unknown':[]} if 'check' in sys.argv else {'ok':True})
elif name=='chatboard.py':
 if 'claim' in sys.argv:count('claims')
 elif 'release' in sys.argv:count('releases')
 out({'ok':True})
elif name=='profile.py':count('profiles');out({'assigned':True})
