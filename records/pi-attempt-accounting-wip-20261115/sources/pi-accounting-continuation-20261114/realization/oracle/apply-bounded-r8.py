# One-shot attributed corpus edit: fixed authored amounts, not a financial generator.
import copy, hashlib, json
from pathlib import Path
r=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1')
policy='PLAN.md@43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61:sections5,7'
local={'A06-reservation-usage-null-status-no-id':'15000000','P130-R05-conflicting-keeps-valuation-high':'20000000','P133-A06-usage-settlement-status-absent':'15000000','P133-A09-conflicting-reservations-with-settlement':'15000000','P9-claimed-charge-above-reservation':'15000000','S06-string-http-status':'15000000','U05-inconsistent-charge-without-response-id':'15000000','V01-cheaper-response-model-undercharge':'60000000','V02-unsplit-writes-undercharged':'10000000'}
# Independently selected ambiguous-observation examples: no physical-slot grant in raw inputs.
ambiguous={'DUP-response-id-across-attempts':'15000000','VC11-multi-turn-session':'15000000','P132-FG-VC11-gated':'15000000'}
assertions=[]
for p in sorted((r/'corpus/cases').glob('*.json')):
 c=json.loads(p.read_text()); oldex=copy.deepcopy(c['expected']); oldev=copy.deepcopy(c['evaluations']); cid=c['case_id']; c['contract']='pi-accounting-contract/1.7-s1'; c['revisions_r8']=[]
 if cid in local:
  c['expected']['attempts'][0]['exposure']['low']={'basis':'usage_valuation','nano':local[cid]}
 for e in c['evaluations']:
  for o in e['expected']['objectives']:
   if cid in local or cid in ambiguous:
    amount=local.get(cid,ambiguous.get(cid))
    owned={a['attempt'] for a in e['expected']['attribution'] if a['ownership']=='owned' and a['objective']==o['objective']}
    for label,states in [('owned_closed',('closed','none')),('owned_outstanding',('outstanding',))]:
     matching=[a for a in c['expected']['attempts'] if a['key'] in owned and a['obligation']['state'] in states and a['exposure']['pool']=='pi_api']
     expected=amount if matching else '0'
     o[label]['low']=expected
     assertions.append({'claim_id':f'{cid}/{e["name"]}/{o["objective"]}/{label}','case_id':cid,'evaluation':e['name'],'objective':o['objective'],'label':label,'expected_low':expected,'raw_input_sha256':hashlib.sha256(json.dumps(c['inputs'],sort_keys=True,separators=(',',':')).encode()).hexdigest(),'policy_ref':policy,'raw_derivation_refs':['oracle/R8-RAW-DERIVATIONS.md#'+cid],'assumptions':['conditional inherited initial scope; no physical independence frame','scope membership remains separately unestablished beyond selected raw assertion'],'qualification':'bounded exact low only; not full ownership or raw-to-wire conformance'})
 for field,old,new in [('expected',oldex,c['expected']),('evaluations',oldev,c['evaluations'])]:
  if old!=new:c['revisions_r8'].append({'field':field,'from':old,'to':copy.deepcopy(new),'rule':policy+'; independently authored bounded low correction'})
 p.write_text(json.dumps(c,indent=1,sort_keys=True)+'\n')
(r/'oracle/R8-LOW-ASSERTIONS.json').write_text(json.dumps({'kind':'accounting_expected_assertion','version':'1.7-s1','authored_by':'independent oracle worker01a1d81b','scope':'finite named low claims only; other scopes REQUIRED/UNMET','assertions':assertions},indent=2,sort_keys=True)+'\n')
