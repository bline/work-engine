import json,pathlib,itertools
r=pathlib.Path('/home/bline/cbm-work/p137-s1r7/s1');p=r/'spec/ck.json';s=json.loads(p.read_text());d=s['decisions'][0];bs=[]
for vals in itertools.product(*d['dimensions'].values()):
 q=dict(zip(d['dimensions'],vals));l,r0,t,v,c=[q[k] for k in ['lane','reservation','settlement','valuation','charge']];why=None;valid=t in ['usage_eligible','usage_ineligible','reservation','upstream_error','error_status']
 if l=='subscription' and t=='not_applicable' and (r0!='none' or v!='not_applicable' or c!='none'):why='NotApplicable clean subscription has no dollar components; a dollar-evidence group has applicable settlement absent/rejected/conflicting/valid, regardless of launch lane.'
 elif l=='unbudgeted' and (r0!='none' or t!='not_applicable' or c!='none'):why='Unbudgeted requests have no reservation-backed settlement or settlement charge.'
 elif l=='pi_api' and t=='not_applicable':why='PiApi dollar-evidence groups always have an applicable settlement decision.'
 elif valid and (r0!='retained' or c!='present'):why='Valid settlement requires unique reservation and a well-formed charge; witness owned by settlement validity.'
 elif t=='usage_eligible' and v!='complete':why='EligibleUsageBound requires complete established fully priced usage.'
 elif t=='absent' and c!='none':why='Absent settlement has no claimed settlement charge.'
 result=None
 if not why:
  available=['zero']+(['reservation'] if r0=='retained' else [])+(['charge'] if c=='present' else [])+(['usage_low','usage_high'] if v in ['complete','incomplete'] else ['supported_low','priced_high'] if v=='unpriceable' else [])
  low='usage_low' if v in ['complete','incomplete'] else 'supported_low' if v=='unpriceable' else 'zero'
  if l=='subscription' and t=='not_applicable':result={'pool':'codex','low':None,'high':None,'components':[],'billing':'unknown','witnesses':[]}
  else:
   exact=t=='usage_eligible' or (l=='unbudgeted' and v=='complete');components=[x for x in available if x not in ['zero','usage_low','supported_low']]
   if t=='usage_eligible':components.remove('reservation')
   result={'pool':'pi_api','low':low,'high':'exact' if exact else 'at_least','components':components or ['zero'],'billing':'established' if exact else 'unresolved','witnesses':['EligibleUsageBound'] if t=='usage_eligible' else ['CompleteUnbudgetedUsage'] if exact else []}
 bs.append({'id':'temp','revision':1,'guard':{k:[x] for k,x in q.items()},'exclusion':why,'result':result,'refs':['X-COMPONENTS','SETTLEMENT-WITNESS','Valuation','Exposure']})
changed=True
while changed:
 changed=False
 for dim in d['dimensions']:
  groups={}
  for b in bs:
   key=json.dumps([b['result'],b['exclusion'],{k:v for k,v in b['guard'].items() if k!=dim}],sort_keys=True)
   if key not in groups:groups[key]=b
   else:groups[key]['guard'][dim]=sorted(set(groups[key]['guard'][dim]+b['guard'][dim]));changed=True
  bs=list(groups.values())
for i,b in enumerate(bs):b['id']=f'X-BRANCH-{i+1:03d}'
d['branches']=bs;p.write_text(json.dumps(s,indent=2,sort_keys=True)+'\n');print(len(bs))
p=r/'tooling/spec-check/src/decisions.rs';s=p.read_text();s=s.replace('lane == "subscription"\n            && (r != "none"', 'lane == "subscription"\n            && st == "not_applicable"\n            && (r != "none"')
s=s.replace('if lane == "subscription" {','if lane == "subscription" && st == "not_applicable" {')
p.write_text(s)
