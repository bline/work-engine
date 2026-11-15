#!/usr/bin/env python3
"""Bounded read-only source inventory. No financial/attribution reduction or consumer execution.
Writes only this script's directory. Numeric tokens are recorded lexically, never priced.
"""
from pathlib import Path
import json,hashlib,collections,ast,datetime
OUT=Path(__file__).resolve().parent
SRC=OUT.parents[1]/'evaluation-premise-repair-r1/subject'
PLAN=OUT.parent/'planning'
MANIFEST=OUT.parents[1]/'evaluation-premise-repair-r1/worker/freeze/manifest.json'
H=lambda b:hashlib.sha256(b).hexdigest()
def write(n,d): (OUT/n).write_text(json.dumps(d,indent=2,ensure_ascii=False)+'\n')
def unique(pairs):
 d={}
 for k,v in pairs:
  if k in d:raise ValueError('duplicate JSON member '+k)
  d[k]=v
 return d
def read(p):return json.loads(p.read_bytes(),object_pairs_hook=unique)
def typ(v):return 'null' if v is None else 'boolean' if type(v)is bool else 'object' if isinstance(v,dict) else 'array' if isinstance(v,list) else 'string' if isinstance(v,str) else 'number'
def state(d,k):return {'presence':'present','json_type':typ(d[k]),'value':d[k]}if k in d else {'presence':'absent'}
def input_hash(c):return H(json.dumps(c['inputs'],sort_keys=True,separators=(',',':')).encode())
# Lexical coordinates bind exact UTF-8 source tokens; object order and duplicate members remain visible.
def coordinates(p):
 text=p.read_text(); dec=json.JSONDecoder();nodes={};dups=[]
 offsets=[0]
 for ch in text:offsets.append(offsets[-1]+len(ch.encode()))
 def ws(i):
  while i<len(text) and text[i].isspace():i+=1
  return i
 def val(i,ptr):
  i=ws(i);start=i;ch=text[i];children=[]
  if ch=='{':
   i=ws(i+1);names=[]
   while text[i]!='}':
    name,kend=dec.raw_decode(text,i);names.append(name);i=ws(kend);assert text[i]==':';cp=ptr+'/'+name.replace('~','~0').replace('/','~1');i=val(i+1,cp);children.append(cp);i=ws(i)
    if text[i]==',':i=ws(i+1)
    else:break
   assert text[i]=='}';i+=1;t='object'
   if len(names)!=len(set(names)):dups.append(ptr)
  elif ch=='[':
   i=ws(i+1);n=0
   while text[i]!=']':
    cp=ptr+'/'+str(n);i=val(i,cp);children.append(cp);n+=1;i=ws(i)
    if text[i]==',':i=ws(i+1)
    else:break
   assert text[i]==']';i+=1;t='array'
  else:
   v,i=dec.raw_decode(text,i);t=typ(v)
  b=text[start:i].encode();node={'pointer':ptr,'byte_start':offsets[start],'byte_end_exclusive':offsets[i],'token_sha256':H(b),'json_type':t}
  if t in ['array','object']:node['children']=children
  if t=='number':node['number_lexeme']=b.decode()
  nodes[ptr]=node;return i
 end=val(0,'');assert ws(end)==len(text)
 return nodes,dups
manifest=read(MANIFEST);entries=manifest['entries'];initial=[]
for e in entries:
 p=SRC/e['path']
 if e['type']=='file':
  b=p.read_bytes();initial.append({'path':e['path'],'sha256':H(b),'matches_manifest':H(b)==e['sha256'],'length_matches':len(b)==e['length']})
 else:initial.append({'path':e['path'],'type':e['type'],'unverified':'not regular file'})
assert all(x.get('matches_manifest',True)and x.get('length_matches',True)for x in initial)
baseline=read(PLAN/'CASE-SCOPE-INVENTORY.json');basecases={c['case_id']:c for c in baseline['cases']};basescopes={x['scope']for x in baseline['scopes']}
# Discover declared input proofs/provenance independently by source-tree inventory; supplied outputs never select the required universe.
proofs=collections.defaultdict(list);provenances=collections.defaultdict(list);externals={};evidence_paths=set();json_failures=[]
for p in sorted((SRC/'oracle').rglob('*.json')):
 try:d=read(p)
 except (ValueError,UnicodeError)as exc:json_failures.append({'path':str(p),'error':str(exc)});continue
 rel=str(p.relative_to(SRC))
 if isinstance(d,dict)and'case_id'in d and'grant_payloads'in d:provenances[d['case_id']].append((p,d))
 docs=d.get('documents',[])if isinstance(d,dict)else[]
 if isinstance(d,dict)and d.get('kind')=='accounting_input_proof':docs=[d]
 for idx,doc in enumerate(docs):
  if isinstance(doc,dict)and doc.get('kind')=='accounting_input_proof':proofs[doc.get('lineage_id')].append((p,idx,doc))
# Consumer literal-read evidence is a static AST extraction, not an executed call trace.
reads=[]
for rel in ['validator/validate_corpus.py','validator/r9_pilot.py']:
 p=SRC/rel;evidence_paths.add(str(p));tree=ast.parse(p.read_bytes())
 for f in tree.body:
  if not isinstance(f,ast.FunctionDef):continue
  keys=[]
  for n in ast.walk(f):
   if isinstance(n,ast.Subscript)and isinstance(n.slice,ast.Constant)and isinstance(n.slice.value,str):keys.append({'line':n.lineno,'receiver':ast.unparse(n.value),'key':n.slice.value,'kind':'subscript'})
   if isinstance(n,ast.Call)and isinstance(n.func,ast.Attribute)and n.func.attr=='get'and n.args and isinstance(n.args[0],ast.Constant)and isinstance(n.args[0].value,str):keys.append({'line':n.lineno,'receiver':ast.unparse(n.func.value),'key':n.args[0].value,'kind':'get'})
  reads.append({'path':rel,'sha256':H(p.read_bytes()),'function':f.name,'start':f.lineno,'end':f.end_lineno,'literal_reads':keys})
write('CONSUMER-READS.json',{'standing':'static literal reads only; no dynamic/exhaustive call-graph claim','functions':reads})
financial=['namespace_grants','primary_membership_validity_exact_F','valuation_retained_amounts','T_ownership_candidates_lifecycle_placement','gate_commitment_provisionality','finite_continuity_order']
classify={
 'inputs':('governing_premise','Complete raw/lifecycle/gate/rate/reference-time premise; no leaf excluded.'),
 'evaluations':('governing_premise','Ordered complete configurations minus each supplied expected member; objectives create required scopes.'),
 'trusted_fixture_inputs':('governing_premise','Separate conditional gate trust channel; exact presence/type/inventory; no live trust inferred.'),
 'profile':('governing_premise','closure_state167 and check_case264 select different gate/closure semantics.'),
 'schema':('governing_premise','check_case256 exact schema selector.'),'contract':('governing_premise','check_case256 exact contract selector.'),
 'case_id':('governing_premise','Equality subject selector in actual qualifiers; naming has no financial semantics.'),
 'role':('governing_premise','check_case260/valid_admitted960 changes selected coverage controls.'),
 'class':('governing_premise','check_case257/1020 coverage and future profile consistency.'),
 'reachability':('governing_premise','check_case258/271 profile-consistency and corpus standing.'),
 'controls':('governing_premise','check_case264/check_corpus976 control relation must resolve outside output inventory.'),
 'invariants':('governing_premise','check_case261/check_corpus1018 invariant coverage selectors; no truth from labels.'),
 'provenance':('governing_premise','check_case260 requires provenance.origin; all content retained conservatively, labels confer no grants.'),
 'added_in':('governing_premise','check_case276 historical revision applicability selector; absence retained.'),
 'twin_of':('governing_premise','check_corpus1030 cross-case presentation/order relation.'),
 'evidence_pair':('governing_premise','check_corpus987 subset/superset/permitted_release finite continuity relation.'),
 'rationale':('governing_premise','check_corpus984 requires nonempty explanation for historical decision differences. Qualification sidecondition only; cannot derive money.'),
 'expected':('supplied_outcome','Current attempt/inventory/lifecycle results under challenge; never nominal truth.'),
 'historical':('supplied_outcome','Historical reader outcomes; not raw truth. Its presence/type/emptiness also constrain profile qualification, separately bound below.'),
 'findings':('supplied_outcome','Historical finding annotations; checked by revisions route, not authority for financial derivation.'),
 'ambiguity':('noncausal_metadata','Required top-level presence in check_case; value is not read by inspected current financial consumer or adopted monetary policy.'),
 'group':('noncausal_metadata','Required top-level presence; grouping label does not select current financial predicates.'),
 'notes':('noncausal_metadata','Required top-level presence; value is not an inspected financial input.')}
for k in ['revisions','revisions_r3','revisions_r4','revisions_r5','revisions_r6','revisions_r7','revisions_r8']:classify[k]=('supplied_outcome','Output-change annotations; checked for shape/history, not nominal expectation authority. Their qualification must remain active separately.')
case_records=[];scope_records=[];frontiers=[];raw_records=[];all_scopes=[];attempt_count=0;unknownkeys=[];rawdup=[];topcounts=collections.Counter();origin_count=0
identity_fields=['model','response_model','authorized_model','provider','requested_tier','observed_tier','response_id']
for p in sorted((SRC/'corpus/cases').glob('*.json')):
 c=read(p);cid=c['case_id'];rel=str(p.relative_to(SRC));evidence_paths.add(str(p));nodes,dups=coordinates(p);assert not dups;digest=H(p.read_bytes());ih=input_hash(c)
 top=[];kept=[]
 for k in c:
  topcounts[k]+=1;cl,reason=classify.get(k,('governing_premise','UNREVIEWED additional channel retained conservatively; root must settle classification.'))
  if k not in classify:unknownkeys.append([cid,k])
  top.append({'key':k,'classification':cl,'reason':reason,'coordinate':nodes['/'+k]})
  if cl=='governing_premise':kept.append(k)
 gnodes=[]
 for ptr,n in nodes.items():
  if not ptr:continue
  root=ptr.split('/')[1]
  if root not in kept:continue
  if root=='evaluations'and len(ptr.split('/'))>=4 and ptr.split('/')[3]=='expected':continue
  if root=='evaluations'and ptr.count('/')==2:
   n=dict(n);n['children']=[v for v in n.get('children',[])if not v.endswith('/expected')]
  # Container bytes may include supplied expected; only child inventory governs projected evaluation containers.
  if root=='evaluations'and ptr.count('/')<=2:n={k:v for k,v in n.items()if k not in ['token_sha256','byte_start','byte_end_exclusive']}
  gnodes.append(n)
 origins=[];events=collections.defaultdict(list);raw_identity_anomalies=[];usage_origins=[];raw_res=[];raw_usd=[]
 for si,s in enumerate(c['inputs']['sessions']):
  for turn,log in s['logs'].items():
   for li,line in enumerate(log['lines']):
    oid=f'{s["alias"]}/{turn}:{li+1}';pointer=f'/inputs/sessions/{si}/logs/{turn}/lines/{li}';origin_count+=1;origins.append(oid)
    try:row=json.loads(line,object_pairs_hook=unique);parse_state='object'if isinstance(row,dict)else'non_object'
    except (ValueError,TypeError)as exc:row=None;parse_state='unreadable:'+type(exc).__name__
    rec={'case_id':cid,'origin_id':oid,'session_id':s['session_id'],'session_index':si,'source_origin_id':f'{s["alias"]}/{turn}','original_position':li+1,'case_file':rel,'case_sha256':digest,'coordinate':nodes[pointer],'raw_utf8_sha256':H(line.encode())if isinstance(line,str)else None,'raw_utf8_length':len(line.encode())if isinstance(line,str)else None,'raw_text':line,'parse_state':parse_state}
    if isinstance(row,dict):
     event=row.get('event');events[str(event)].append(oid);rec['event']=state(row,'event');rec['field_inventory']=list(row);rec['identity_states']={k:state(row,k)for k in identity_fields};rec['usage_state']=state(row,'usage');rec['raw_time']=state(row,'at');rec['rid_state']=state(row,'rid');rec['n_state']=state(row,'n');rec['usd_state']=state(row,'usd');rec['reservation_state']=state(row,'reserved_usd')
     if 'usage'in row:usage_origins.append(oid)
     if event=='usd_reserved':raw_res.append(oid)
     if isinstance(row.get('usd'),dict):raw_usd.append(oid)
     for k in ['model','response_model','authorized_model']:
      if k in row and row[k]is not None and (not isinstance(row[k],str)or not row[k]):raw_identity_anomalies.append({'origin':oid,'field':k,'state':state(row,k)})
    raw_records.append(rec)
 # Grants are candidate premise declarations, never trusted by discovery, hash or matching lineage alone.
 sources=[];payloads=[]
 for pp,idx,doc in proofs.get(cid,[]):
  evidence_paths.add(str(pp));sources.append({'path':str(pp.relative_to(SRC)),'sha256':H(pp.read_bytes()),'document_index':idx,'initial_contracts':doc.get('initial_contracts',[]),'grants':doc.get('grants',[]),'exact_links':doc.get('exact_links',[]),'identity_bindings':doc.get('identity_bindings',[]),'independence_frame_presence':'present'if'independence_frame'in doc else'absent','standing':'DECLARED_NOT_INDEPENDENTLY_ADMITTED'})
  for grant in doc.get('grants',[]):
   ref=grant.get('exact_payload_ref');pr,_,frag=(ref or '').partition('#');target=(SRC/pr).resolve()
   record={'ref':ref,'fragment_selector':frag,'status':'MISSING'}
   if pr and target.is_relative_to(SRC)and target.is_file():
    evidence_paths.add(str(target));external=read(target);nd,dd=coordinates(target);record.update(path=str(target.relative_to(SRC)),sha256=H(target.read_bytes()),status='LOCATED_NOT_ADMITTED',top_inventory=list(external)if isinstance(external,dict)else None)
    fragments=[(j,x)for j,x in enumerate(external.get('grant_payloads',[]))if x.get('source_origin_id')==frag]if isinstance(external,dict)else[]
    record['matching_grant_payloads']=[{'pointer':'/grant_payloads/'+str(j),'coordinate':nd['/grant_payloads/'+str(j)],'value':x}for j,x in fragments]
    record['fragment_resolution']='unique source_origin_id match'if len(fragments)==1 else'UNRESOLVED fragment needs typed external resolver'
   payloads.append(record)
 for pp,doc in provenances.get(cid,[]):evidence_paths.add(str(pp))
 scopes=[]
 for ei,e in enumerate(c['evaluations']):
  for oi,o in enumerate(e['objectives']):
   for label in ['owned_closed','owned_outstanding']:
    sid=f'{cid}/{e["name"]}/{o["objective"]}/{label}';all_scopes.append(sid);scopes.append(sid)
    scope_records.append({'scope_id':sid,'case_id':cid,'evaluation_index':ei,'objective_index':oi,'label':label,'required_from':'raw evaluation objective configuration + adopted T/R closed/outstanding scope policy, independently of supplied output existence','objective_coordinate':nodes[f'/evaluations/{ei}/objectives/{oi}'],'usage_observation_universe_to_adjudicate':usage_origins,'U_seed_slots_alternatives_low':{'state':'UNEXAMINED','required':'Independently classify every listed raw usage origin into T admitted/candidate/excluded for this exact scope; then establish Seed, all U, lawful slots/distinctness, V8 alternatives, low and tag. Listed usage inventory is a superset to adjudicate, not asserted U.','owner':'independent oracle; root for missing grants/policy'},'actual_python_consumption':'UNEXECUTED','independent_rust_consumption':'UNEXECUTED'})
 attempts=len(c['expected']['attempts']);attempt_count+=attempts
 claims={k:{'state':'UNEXAMINED','required':f'Independent {k} derivation for this exact case raw origins/evaluation frontier; no expected output used as truth.','owner':'independent oracle; root decides missing authority'}for k in financial}
 claims['raw_E_identity_cardinality']={'state':'RAW_FACTS_ESTABLISHED_ONLY','evidence':'RAW-ORIGINS.json exact original line inventory and lexical identity states; raw duplicate text is not an admitted evidence equivalence class','origin_count':len(origins),'raw_reservation_origins':raw_res,'raw_usd_origins':raw_usd,'identity_anomalies':raw_identity_anomalies,'logical_classification':'UNEXAMINED'}
 claims['namespace_grants']['declared_proof_count']=len(sources);claims['namespace_grants']['question']='Which located declaration/guarantee is independently admitted for these exact origins, namespace and payload? A located dollar scope cannot grant admission namespace, linkage, slots or live trust.'
 claims['primary_membership_validity_exact_F']['raw_primary_role_candidates']={'usd_reserved':raw_res,'usd_object':raw_usd};claims['primary_membership_validity_exact_F']['question']='Derive complete C classes and local checks, exact usage/anchor membership, F links/endpoints/whole coverage and retained constituents; candidate role list is lexical only.'
 claims['valuation_retained_amounts']['usage_origin_candidates']=usage_origins;claims['T_ownership_candidates_lifecycle_placement']['required_scopes']=scopes
 claims['gate_commitment_provisionality']['trusted_fixture_inputs']=state(c,'trusted_fixture_inputs');claims['finite_continuity_order']['declared_cross_case_relations']={k:c[k]for k in ['twin_of','evidence_pair']if k in c}
 case_records.append({'case_id':cid,'case_file':rel,'case_sha256':digest,'input_canonical_sha256':ih,'matches_navigation_case':digest==basecases[cid]['case_sha256'],'scope_ids':scopes,'raw_origin_ids':origins,'raw_event_inventory':dict(events),'usage_observation_origins':usage_origins,'profile':state(c,'profile'),'policy_binding':'POLICY-BINDINGS.json + DERIVATIONS.md; applicability and nominal financial truths unresolved unless explicit authored derivation names this exact case','claim_families':claims,'nominal_derivations':[],'actual_attempt_count_baseline_only':attempts})
 frontiers.append({'case_id':cid,'case_file':rel,'case_sha256':digest,'top_inventory':list(c),'top_channels':top,'trusted_fixture_inputs_presence':state(c,'trusted_fixture_inputs'),'governing_nodes':gnodes,'historical_profile_sidecondition':{'presence':'present'if'historical'in c else'absent','type':typ(c.get('historical')),'is_empty_object':c.get('historical')=={}},'located_input_proofs':sources,'referenced_payloads':payloads,'external_admission':'REQUIRED_UNMET: declarations and matching snapshots alone do not establish independent trust','frontier_status':'SOURCE_INVENTORIED; complete for selected case-tree channels; semantic closure over external policy and all Rust dependencies remains UNEXAMINED'})
assert len(all_scopes)==len(set(all_scopes))
old=set(basecases);new={c['case_id']for c in case_records};universe={'case_count':len(case_records),'evaluation_count':sum(len(read(SRC/x['case_file'])['evaluations'])for x in case_records),'objective_configurations':len(all_scopes)//2,'scope_count':len(all_scopes),'required_scope_ids':all_scopes,'case_additions':sorted(new-old),'case_deletions':sorted(old-new),'scope_additions':sorted(set(all_scopes)-basescopes),'scope_deletions':sorted(basescopes-set(all_scopes)),'scope_identity_source':'Complete raw evaluations/objectives plus adopted closed/outstanding policy. Baseline used for subsequent comparison only.','output_attempt_count_baseline':attempt_count,'output_attempt_count_is_cap':False}
write('UNIVERSE.json',universe);write('RAW-ORIGINS.json',{'standing':'raw lexical inventory only; not logical C classes/financial truth','origin_count':origin_count,'origins':raw_records});write('FRONTIERS.json',{'standing':'exact source-token frontier inventory + unadmitted external declarations; no applicability qualification','cases':frontiers})
write('LEDGER.json',{'schema':'full-s1-independent-derivation-ledger/1-candidate','standing':'BOUNDED_COMPLETE_OBLIGATION_ACCOUNTING; FINANCIAL_DERIVATION_INCOMPLETE; NOT_QUALIFIED','source':str(SRC),'manifest':str(MANIFEST),'manifest_sha256':H(MANIFEST.read_bytes()),'packet_sha256':H((OUT/'PACKET.md').read_bytes()),'universe':universe,'cases':case_records,'scopes':scope_records,'summary':{'case_obligations':len(case_records),'scope_obligations':len(scope_records),'raw_origins':origin_count,'fully_derived_cases':0,'fully_derived_scopes':0,'financial_case_family_entries_unexamined':len(financial)*len(case_records),'scope_low_entries_unexamined':len(scope_records),'unknown_top_channels':unknownkeys,'top_channel_counts':dict(topcounts),'actual_consumer_runs':0},'all11_historical_findings':'OPEN','remaining_omissions':'OBLIGATIONS.json'})
# Bind hashes/read evidence without publishing a source copy.
policies=[Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md'),Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/design-r7/PLAN-AMENDMENT.md'),Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/PLAN-AMENDMENT-R3.md'),Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/design-evaluation-premise-r1/PLAN-R2.md'),Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/design-low-target-r1/PLAN-R2.md'),Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/policies/ROOT-POLICY-R5.md'),Path('/home/bline/cbm-work/p134-module-a/ARCHITECTURE-GUARD.md')]
policyrecords=[]
for p in policies:
 if p.exists():policyrecords.append({'path':str(p),'sha256':H(p.read_bytes())});evidence_paths.add(str(p))
 else:policyrecords.append({'path':str(p),'state':'MISSING_PATH_DO_NOT_ASSUME_POLICY'})
rates=[]
for p in sorted((SRC/'oracle/rates').glob('*.json')):rates.append({'path':str(p.relative_to(SRC)),'sha256':H(p.read_bytes()),'standing':'pinned rate premise, not live pricing'});evidence_paths.add(str(p))
write('POLICY-BINDINGS.json',{'policies':policyrecords,'rates':rates,'P1_P6_D3':'P1–P6 retain base plan and canonical clause authority; no raw reducer, general financial or grant interpreter, partition solver, producer/live grant or S2a is implemented.'})
write('EVIDENCE-PATHS.json',sorted(evidence_paths));write('INTEGRITY.json',{'manifest_sha256':H(MANIFEST.read_bytes()),'manifest_entry_count':manifest['entry_count'],'initial_file_checks':initial,'json_oracle_parse_failures':json_failures,'initial_status':'PASS','final_status':'PENDING'})
print(json.dumps({'source':str(SRC),'cases':len(case_records),'evaluations':universe['evaluation_count'],'objectives':universe['objective_configurations'],'scopes':len(all_scopes),'origins':origin_count,'baseline_attempts':attempt_count,'universe_differences':{k:universe[k]for k in ['case_additions','case_deletions','scope_additions','scope_deletions']},'proof_paths':sum(len(x['located_input_proofs'])for x in frontiers),'external_fragments':sum(len(x['referenced_payloads'])for x in frontiers),'evidence_paths':len(evidence_paths),'unknown_top_keys':unknownkeys,'policy_missing':[x['path']for x in policyrecords if x.get('state')]}))
