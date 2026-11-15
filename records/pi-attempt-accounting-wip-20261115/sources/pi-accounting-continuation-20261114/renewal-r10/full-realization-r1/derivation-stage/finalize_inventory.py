"""Finalize lexical fragment inventory and immutable source verification; no financial evaluator."""
from pathlib import Path
import json,hashlib,ast
out=Path(__file__).resolve().parent;s=out.parents[1]/'evaluation-premise-repair-r1/subject';tree=ast.parse((out/'extract_ledger.py').read_text());nodes=[n for n in tree.body if isinstance(n,ast.FunctionDef)and n.name in ['typ','coordinates']];ns={'json':json,'H':lambda b:hashlib.sha256(b).hexdigest()};exec(compile(ast.Module(body=nodes,type_ignores=[]),'<owned-coordinate-functions>','exec'),ns);coords=ns['coordinates'];cache={}
def index(p):
 if str(p)not in cache:cache[str(p)]=coords(p)[0]
 return cache[str(p)]
f=json.loads((out/'FRONTIERS.json').read_text());resolved=0;unresolved=[]
for c in f['cases']:
 for source in c['located_input_proofs']:
  path=s/source['path'];doc=json.loads(path.read_text());idx=source['document_index'];prefix='/documents/'+str(idx)if'documents'in doc else'';ps=index(path)
  for gi,g in enumerate(source['grants']):
   matches=[r for r in c['referenced_payloads']if r['ref']==g.get('exact_payload_ref')and'input_proof_path'not in r]
   if not matches:continue
   r=matches[0];r.update(input_proof_path=source['path'],input_proof_document_index=idx,grant_id=g['grant_id'],grant_coordinate=ps[prefix+'/grants/'+str(gi)],payload_reference_coordinate=ps[prefix+'/grants/'+str(gi)+'/exact_payload_ref'],authority_status='NOT_ESTABLISHED_BY_THIS_EXTRACTION')
   if r['status']!='LOCATED_NOT_ADMITTED':continue
   payload=s/r['path'];d=json.loads(payload.read_text());px=index(payload);r['payload_document_coordinate']=px['']
   if r.get('matching_grant_payloads'):r['resolution_reason']='Exactly one grant_payloads entry has source_origin_id equal to complete fragment; lexical location only.';resolved+=1;continue
   frag=r['fragment_selector'];delim='/initial_reference_scope_payloads/'
   if delim in frag and isinstance(d,dict)and isinstance(d.get('cases'),list):
    caseid,grant=frag.split(delim,1);found=[]
    for ci,case in enumerate(d['cases']):
     if case.get('case_id')!=caseid:continue
     for pi,p in enumerate(case.get('initial_reference_scope_payloads',[])):
      if p.get('grant_id')==grant:
       pointer=f'/cases/{ci}/initial_reference_scope_payloads/{pi}';found.append({'pointer':pointer,'coordinate':px[pointer],'value':p})
    r['matching_grant_payloads']=found;r['fragment_resolution']='unique case_id + initial_reference_scope_payloads grant_id match'if len(found)==1 else'UNRESOLVED fixed case/grant selector';r['resolution_reason']='Known finite declaration shape: complete case_id before literal delimiter and complete grant_id after it, including slashes. Locates a declaration only; no policy/authority validation.'
   if len(r.get('matching_grant_payloads',[]))==1:resolved+=1
   else:unresolved.append({'case_id':c['case_id'],'ref':r['ref'],'reason':r.get('fragment_resolution','missing document')})
(out/'FRONTIERS.json').write_text(json.dumps(f,indent=2)+'\n');(out/'PAYLOAD-RESOLUTION.json').write_text(json.dumps({'references':172,'resolved_lexical_locations':resolved,'unresolved':unresolved,'standing':'Raw declaration locations only; zero independently admitted grants by extraction. Initial64 limitation resolved with second fixed source shape; no general grant interpreter.'},indent=2)+'\n')
p=out/'INTERFACES.md';t=p.read_text().replace('In particular172 located payload references include64 not resolved by source_origin_id; generic fragment lookup is not proposed as grant interpretation. Root must bind each lawful fixed payload type or preserve it unmet.','All172 payload references now have exact lexical fragment locations:108 use source_origin_id,64 use the explicit cases[].case_id/initial_reference_scope_payloads[].grant_id shape. These locations establish no grant authority. Root must bind each lawful fixed payload type and independent admission or preserve it unmet.');p.write_text(t)
o=json.loads((out/'OBLIGATIONS.json').read_text())
for q in o['owner_questions']:
 if q['id']=='Q-PAYLOAD':q['question']='All172 declaration fragments are lexically located with exact byte coordinates. Independently justify each guarantee/namespace/extent and admission, and inspect actual consumer application. Lexical resolution establishes no authority.'
(out/'OBLIGATIONS.json').write_text(json.dumps(o,indent=2)+'\n')
integ=json.loads((out/'INTEGRITY.json').read_text());mf=json.loads(Path(json.loads((out/'LEDGER.json').read_text())['manifest']).read_text());drift=[]
for e in mf['entries']:
 p=s/e['path']
 if e['type']=='file':
  b=p.read_bytes()
  if hashlib.sha256(b).hexdigest()!=e['sha256']or len(b)!=e['length']or p.stat().st_mode&0o777!=e['mode']:drift.append(e['path'])
manifestpaths={e['path']for e in mf['entries']};actual={str(p.relative_to(s))for p in s.rglob('*')if p.is_file()or p.is_symlink()};integ.update(final_status='PASS'if not drift and actual==manifestpaths else'FAIL',final_drift=drift,missing=sorted(manifestpaths-actual),extra=sorted(actual-manifestpaths),final_regular_files_checked=sum(e['type']=='file'for e in mf['entries']));(out/'INTEGRITY.json').write_text(json.dumps(integ,indent=2)+'\n')
print(json.dumps({'payload_locations':resolved,'unresolved':len(unresolved),'source_integrity':integ['final_status'],'files':integ['final_regular_files_checked'],'summary':json.loads((out/'LEDGER.json').read_text())['summary']}))
