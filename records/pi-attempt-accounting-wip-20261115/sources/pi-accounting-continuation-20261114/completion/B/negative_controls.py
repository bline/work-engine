"""Finite actual-consumer controls. A private copy isolates mutations from peer work."""
import pathlib,json,shutil,importlib.util,copy
R=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114');S=R/'realization/s1';T=R/'completion/B/control-subject'
T.mkdir(exist_ok=True)
for rel in ['validator','spec']:
 shutil.copytree(S/rel,T/rel,dirs_exist_ok=True)
(T/'oracle').mkdir(exist_ok=True)
for rel in ['oracle/R8-LOW-ASSERTIONS.json','oracle/r8-completion-B/LOW-ASSERTIONS.json']:
 dest=T/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(S/rel,dest)
spec=importlib.util.spec_from_file_location('actual_partition_validator',T/'validator/validate_corpus.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
rows=json.loads((R/'completion/PARTITIONS.json').read_text())['partitions']['B'];claims=json.loads((S/'oracle/r8-completion-B/LOW-ASSERTIONS.json').read_text())['assertions'];by={}
for x in claims:by.setdefault(x['case_id'],[]).append(x)
results=[]
for row in rows:
 cid=row['case_id'];c=json.loads((S/row['path']).read_text());rel=f'oracle/r8-completion-B/{cid}.bundle.json';bundle=json.loads((S/rel).read_text());dest=T/rel;dest.write_text(json.dumps(bundle));f=v.Failures();v.check_accounting_references(c,f)
 assert not f,(cid,f)
 for claim in by[cid]:
  # Wrong local output, matching wrong R wire plus matching wrong supplied record.
  bad=copy.deepcopy(c);b=copy.deepcopy(bundle);scope=claim['claim_id'];wrong=str(int(claim['expected_low'])+1)
  ev=next(e for e in bad['evaluations'] if e['name']==claim['evaluation']);o=next(o for o in ev['expected']['objectives'] if o['objective']==claim['objective']);o[claim['label']]['low']=wrong
  for a in bad['expected']['attempts']:
   if a['key'] in claim['owned_unit_refs']:a['exposure']['low']['nano']=wrong
  low=next(l for l in b['documents'][1]['low_rollups'] if l['scope_ref']==scope);low['single_floor']=low['selected_low']=wrong;low['basis']='usage_valuation';low['credit_mode']='single_ambiguous'
  dest.write_text(json.dumps(b));f=v.Failures();v.check_accounting_references(bad,f)
  assert any('independent consistent-wrong low/result/rollup' in str(x) for x in f),(scope,f)
  results.append({'claim_id':scope,'consistent_wrong_rejected':True})
 dest.write_text(json.dumps(bundle))
 # Drop one whole scope, keep otherwise matching output.
 b=copy.deepcopy(bundle);b['documents'][1]['low_rollups'].pop();dest.write_text(json.dumps(b));f=v.Failures();v.check_accounting_references(c,f);assert f
 # Drop/mispoint every active attempt's result reference; scalar amounts do not change.
 for a in c['expected']['attempts']:
  bad=copy.deepcopy(c);next(x for x in bad['expected']['attempts'] if x['key']==a['key'])['accounting_result_ref']=rel+'#input:'+cid
  dest.write_text(json.dumps(bundle));f=v.Failures();v.check_accounting_references(bad,f);assert any('accounting_result_ref' in str(x) for x in f)
 dest.write_text(json.dumps(bundle))
print(json.dumps({'cases':len(rows),'consistent_wrong_controls':len(results),'all_rejected':True,'scope_omission_controls':len(rows),'active_attempt_wrong_kind_controls':sum(len(json.loads((S/r['path']).read_text())['expected']['attempts'])for r in rows)}))
(R/'completion/B/NEGATIVE-RESULTS.json').write_text(json.dumps({'results':results},indent=2)+'\n')
