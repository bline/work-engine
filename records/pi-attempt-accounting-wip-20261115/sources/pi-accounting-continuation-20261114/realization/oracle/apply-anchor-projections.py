# Finite seven-case policy amendment. No raw correlation or financial reduction.
import copy,json
from pathlib import Path
r=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1'); old=Path('/home/bline/cbm-work/p137-s1r7/s1')
controls={'N01-identity-refusal-dearer-served':('res:S1/turn-1:rid=1','S1/turn-1:2','row:S1/turn-1:line=2'),'N01-orphan-identity-refusal':('res:S1/turn-1:rid=1','S1/turn-1:2','row:S1/turn-1:line=2'),'N01-variant-reservation-above-both':('res:S1/turn-1:rid=1','S1/turn-1:2','row:S1/turn-1:line=2'),'N04-budgeted-header-failure-own-reservation':('res:S1/turn-1:rid=2','S1/turn-1:4','row:S1/turn-1:line=4'),'N04-duplicated-read-failure':('row:S1/turn-1:line=3','S1/turn-1:1','row:S1/turn-1:line=1'),'P130-R05-conflicting-keeps-valuation-high':('res:S1/turn-1:rid=1','S1/turn-1:2','row:S1/turn-1:line=2'),'VC07-read-failure-paired':('row:S1/turn-1:line=2','S1/turn-1:1','row:S1/turn-1:line=1')}
template=json.loads((old/'corpus/cases/N04-duplicated-read-failure.json').read_text())['expected']['attempts'][0]
for cid,(primary,ref,key) in controls.items():
 p=r/'corpus/cases'/f'{cid}.json';c=json.loads(p.read_text());before=json.loads((old/'corpus/cases'/p.name).read_text())
 a=next(a for a in c['expected']['attempts'] if a['key']==primary);a['evidence'].remove(ref);a['correlation'].pop('auxiliary',None);a['diagnostics']=[d for d in a['diagnostics'] if d!='ReadFailurePaired']
 anchor=copy.deepcopy(template);anchor['key']=key;anchor['evidence']=[ref];c['expected']['attempts'].append(anchor)
 for e in c['evaluations']:
  primary_at=next(t for t in e['expected']['attribution'] if t['attempt']==primary);new=copy.deepcopy(primary_at);new['attempt']=key;e['expected']['attribution'].append(new)
  for o in e['expected']['objectives']:
   related=(new['ownership']=='owned' and new['objective']==o['objective']) or (new['ownership'] in ('uncertain','unadmitted') and o['objective'] in new.get('candidates',[]))
   if related:
    o['provisional']=True;o['reasons']=sorted(set(o['reasons'])|{'UnaccountedAttempt','UnresolvedExposure'});o['status']='not_established'
    if new['ownership']=='owned':o['owned_closed']['high']['kind']='at_least'
 c['revisions_r8']=[]
 for field in ('expected','evaluations'):
  if c[field]!=before[field]:c['revisions_r8'].append({'field':field,'from':before[field],'to':copy.deepcopy(c[field]),'rule':'PLAN43d0e7c8 sections4,5,8; raw bytes unchanged; unlinked auxiliary origin gets separate AtLeast0; retained usage contexts/highs and local supported low; conditional initial scope only'})
 p.write_text(json.dumps(c,indent=1,sort_keys=True)+'\n')
