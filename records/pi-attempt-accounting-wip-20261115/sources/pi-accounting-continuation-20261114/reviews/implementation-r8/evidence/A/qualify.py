"""Private finite authoring qualification against actual consumers; no financial evaluator."""
import json,pathlib,hashlib,tarfile,copy,importlib.util
from unittest.mock import patch
R=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114');S=R/'realization/s1';parts=json.load(open(R/'completion/PARTITIONS.json'))['partitions']['A'];spec=importlib.util.spec_from_file_location('actual',S/'validator/validate_corpus.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v);rates=v.load_rates(S/'oracle/rates');load=json.load
# Exact rollback to immutable source, preserving every old revision and historical field.
with tarfile.open(R/'completion/baseline-freeze/subject.tar') as tar:
 members={x.name.lstrip('./'):x for x in tar.getmembers() if x.isfile()}
 for p in parts:
  key=next(k for k in members if k.endswith(p['path']));raw=tar.extractfile(members[key]).read();assert hashlib.sha256(raw).hexdigest()==p['baseline_sha256'];baseline=json.loads(raw);c=load(open(S/p['path']));rollback=copy.deepcopy(c)
  owned=[x for x in rollback['revisions_r8'] if x['rule'].startswith('R8 finite completion A')]
  for amendment in reversed(owned):
   node,k=v._locate(rollback,amendment['field']);assert node[k]==amendment['to'];assert amendment['from']=='__absent__';del node[k]
  rollback['revisions_r8']=[x for x in rollback['revisions_r8'] if not x['rule'].startswith('R8 finite completion A')];assert rollback==baseline,p['path']
print('PASS69 exact reversible source amendments/raw/history preserved')
def run(cid,kind):
 c=load(open(S/f'corpus/cases/{cid}.json'));bundlepath=S/f'oracle/r8-completion-A/{cid}.bundle.json';b=load(open(bundlepath));result=b['documents'][1]
 if kind=='consistent-wrong-low':
  for a in c['expected']['attempts']:
   if a['exposure']['pool']=='pi_api':a['exposure']['low']={'basis':'none','nano':'0'};a['valuation']['low']='0'
  for e in c['evaluations']:
   for o in e['expected']['objectives']:
    for lab in ['owned_closed','owned_outstanding']:o[lab]['low']='0'
  for low in result['low_rollups']:low.update(selected_low='0',single_floor='0',basis='none',credit_mode='none')
 elif kind=='omitted-result':b['documents'].pop()
 elif kind=='dangling-attempt':c['expected']['attempts'][0]['accounting_result_ref']+='-wrong'
 elif kind=='same-low-wrong-scope':
  o=c['evaluations'][0]['expected']['objectives'][0];o['owned_closed']['supported_low_evidence_ref']=o['owned_outstanding']['supported_low_evidence_ref']
 elif kind=='wrong-unit':result['units'][0]['unit_id']='wrong-unit'
 elif kind=='wrong-kind':b['documents'][0]['kind']='accounting_result'
 elif kind=='missing-coverage':result['coverage_decisions']=[]
 elif kind=='switched-owned-membership':
  c['evaluations'][0]['expected']['attribution'][0]['ownership']='uncertain'
 else:raise ValueError(kind)
 def amended_load(f,*args,**kwargs):
  if pathlib.Path(getattr(f,'name',''))==bundlepath:return copy.deepcopy(b)
  return load(f,*args,**kwargs)
 F=v.Failures()
 with patch.object(v.json,'load',amended_load):v.check_accounting_references(c,F);v.check_case(c,rates,F)
 assert F,(cid,kind,'accepted consistent wrong candidate')
 print('REJECT',cid,kind, F[0])
controls=[('P133-A09-conflicting-reservations-with-settlement','consistent-wrong-low'),('P130-R05-conflicting-keeps-valuation-high','consistent-wrong-low'),('P132-FG-VC11-gated','consistent-wrong-low')]
controls += [('P130-A5-unreadable-line-added',k) for k in ['omitted-result','dangling-attempt','same-low-wrong-scope','wrong-unit','wrong-kind','missing-coverage','switched-owned-membership']]
for cid,kind in controls:run(cid,kind)
print('PASS10 finite wrong controls rejected by actual Python entry functions')
