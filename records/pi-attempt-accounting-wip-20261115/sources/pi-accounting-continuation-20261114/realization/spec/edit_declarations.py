import json,pathlib
p=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1/spec')
def dump(n,v): (p/n).write_text(json.dumps(v,indent=2,sort_keys=True)+'\n')
s=json.loads((p/'ck.json').read_text());s['contract_version']='pi-accounting-contract/1.7-s1'
# S has one reference forwarding constructor, regardless of logical validity.
s['types'].append({'id':'SettlementForward','revision':1,'variants':[{'name':'forward','fields':{'validity_ref':'String'}}]})
s['projections'].append({'id':'PROJECT-SettlementForward-forward','revision':1,'type_id':'SettlementForward','variant':'forward','external_state':'forward','wire_tag':'variant','wire_value':'forward','fields':{'validity_ref':{'type':'String','source':'validity_ref'}},'refs':['SETTLEMENT-WITNESS']})
for r in s['rules']:
 if r['partition']=='S-DECISION':
  r['revision']+=1;r['outputs']=[{'type_id':'SettlementForward','variant':'forward','fields':['validity_ref'],'bindings':{'validity_ref':'C-validity.validity_ref'}}];r['text']='S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.'
s['stages'][2]['outputs']=['validity_ref','temporal_obligation','execution','forwarding']
# Permit unresolved PiApi orphan/anchor with no settlement witness.
import itertools,copy
expanded=[]
for b in s['decisions'][0]['branches']:
 if b['guard']['lane']==['pi_api'] and b['guard']['settlement']==['not_applicable']:
  keys=list(b['guard'])
  for idx,vs in enumerate(itertools.product(*(b['guard'][k] for k in keys))):
   t=copy.deepcopy(b);t['id']+='-anchor-'+str(idx);t['guard']={k:[v]for k,v in zip(keys,vs)};expanded.append(t)
 else:expanded.append(b)
s['decisions'][0]['branches']=expanded
for b in s['decisions'][0]['branches']:
 g=b['guard']
 if g['lane']==['pi_api'] and g['settlement']==['not_applicable']:
  # Existing compressed branches are per tuple; split if needed is asserted.
  assert all(len(v)==1 for v in g.values()),g
  r,v,c=g['reservation'][0],g['valuation'][0],g['charge'][0]
  low='usage_low' if v in ('complete','incomplete') else 'supported_low' if v=='unpriceable' else 'zero'
  comps=(['reservation'] if r=='retained' else [])+(['charge'] if c=='present' else [])+(['usage_high'] if v in ('complete','incomplete') else ['priced_high'] if v=='unpriceable' else [])
  b['exclusion']=None;b['result']={'pool':'pi_api','low':low,'high':'at_least','components':comps or ['zero'],'billing':'unresolved','witnesses':[]};b['revision']+=1
# Policy records are declarations with explicit causal read sets.
texts={
'INITIAL-REFERENCE':'E inventories immutable origins (lineage, source origin, original position, raw bytes SHA256). Same-origin reobservation is idempotent. InitialReferenceContract fixes admitted namespace/scope/duplicate rule per origin; source revision and producer identity cannot replace it. Exact-copy equivalence is scoped raw-line-byte equivalence under its own admitted guarantee; decoded JSON or equal amounts are insufficient.',
'ATOMIZATION':'C Atomize(B) iff any non-base admitted identity binding, independently distinct-origin fact or retained contradiction affects immutable B. BreakCopies(B) iff distinctness/contradiction separates initially equivalent copies; implies Atomize. Without Atomize one base hold; Atomize one immutable evidence-atom hold; BreakCopies one original-origin hold. Final immutable fact presence controls predicates regardless of arrival order. Matching P labels never remerge. Parent inactive and every origin has one active high owner.',
'LOGICAL-PRIMARY':'C-primary logical obligations are separate from high units. Count distinct reservation/settlement evidence under admitted exact equivalence, never distinct amounts. C-validity owns immutable witness; S forwards validity_ref exactly and has no writable SettlementDecision constructor. Auxiliary attachment never modifies primary validity. Late source-earlier R may complete missing prerequisites; malformed/order/cardinality/undercharge failures remain.',
'FINANCIAL-CANDIDATES':'C admits a context candidate on same logical source/usable n, exact ticket/measurement/producer link, or same source and missing n absent immutable qualified unrelatedness. Later display attachment or narrowed identities never remove a previously justified priced high. Context is read-only pricing evidence, not another bill.',
'DEFINITE-LINK':'Exact row co-reference consumes only an admitted exact_row_coreference grant for the exact origins/extent. RS/n uniqueness, row adjacency, labels and writer hashes are insufficient. Independence, scope, copy, continuity and co-reference guarantees cannot substitute for one another.',
'WHOLE-COVERAGE':'F alone releases exactly complete current hold units with complete origin coverage, C-validity and complete V witnesses and admitted exact links for all auxiliary anchors. Retain known charge/context; no partial-overlap release. Independent physical bills cannot be merged. Conflicting overlapping coverage retains original/refined holds. Disjoint/identical exact coverage composes/idempotent. Complete local unbudgeted usage cannot discharge an unlinked AtLeast0 anchor.',
'SUPPORTED-LOW':'X local usage lows survive rejected/conflicting settlement. R uses separate physical-credit lows, never sum(attempt lows). For scope U and positive usage seed, single_floor=V8Low(U); each admitted independently billed owned slot p has Alternatives(p)=all U not affirmatively physically distinct. independent_floor=sum(V8Low(Alternatives(p))); R.low=max(single_floor, independent_floor). V8Low is min across complete alternatives, zero if unsupported. Contradictory/unestablished frame admits no slot addition. T provides owned/possible memberships; R recomputes for each scope. Ties prefer independent_slots if nonempty. Positive basis usage_valuation, otherwise none.',
'CONTINUITY':'Successor preserves every predecessor origin, immutable initial contract, trigger fact and conflict under exact origin mapping. Omission/rewriting/unsupported many-to-one fails admission; last admitted financial subject retained. Digest rename cannot create fresh bases. Live continuity remains unestablished.',
'EXACT-WIRE':'Actual external projections: exposure.pool; high.kind/nano/basis with flattened Bound; decimal nano strings; valuation low/high as decimal strings and derivations from splits; reason strings qualified. Obligation none preserves cause/amount_source:none and closed preserves by. Per-attempt accounting_result_ref and rollup supported_low_evidence_ref refer to computed results, never input grants or expected assertions.',
'RAW-FIELD-PREDICATES':'Usage presence, JSON type, count cardinality, malformed identity/tier, exact duplicate identity, atomization, financial candidates, exact linkage, low alternatives, full origin coverage and continuity are semantic predicates. Their actual consumers must invalidate on revisions, independently of whole-file byte drift.'}
for id,text in texts.items(): s['invariants'].append({'id':id,'revision':1,'text':text,'refs':[],'depends':[]})
links={'ATOMIZATION':['INITIAL-REFERENCE'],'LOGICAL-PRIMARY':['INITIAL-REFERENCE','ATOMIZATION'],'FINANCIAL-CANDIDATES':['INITIAL-REFERENCE','RAW-FIELD-PREDICATES'],'DEFINITE-LINK':['INITIAL-REFERENCE'],'WHOLE-COVERAGE':['ATOMIZATION','LOGICAL-PRIMARY','DEFINITE-LINK','VALUATION','USAGE-PRESENCE'],'SUPPORTED-LOW':['DEFINITE-LINK','FINANCIAL-CANDIDATES','USAGE-PRESENCE','VALUATION','RAW-FIELD-PREDICATES'],'CONTINUITY':['INITIAL-REFERENCE','ATOMIZATION'],'EXACT-WIRE':['WHOLE-COVERAGE','SUPPORTED-LOW','CONTINUITY']}
for n in s['invariants']:
 if n['id'] in links:n['depends']=links[n['id']]
 replacements={'CORRELATION':'INITIAL-REFERENCE','SETTLEMENT-WITNESS':'LOGICAL-PRIMARY','X-COMPONENTS':'ATOMIZATION','BOUNDARY-CONSUMERS':'SUPPORTED-LOW','R-FINANCIAL-PROVISIONAL':'SUPPORTED-LOW','CORPUS-ENVELOPE':'EXACT-WIRE','SERIALIZATION':'EXACT-WIRE'}
 if n['id'] in replacements:n['depends']=list(set(n['depends']+[replacements[n['id']]]));n['revision']+=1
 if n['id']=='INV-F3': n['text']='Correlation/refinement cannot lower held high or remove unresolvedness. Complete affirmative F coverage alone may replace exactly covered holds regardless of which prerequisite arrives last. This explicitly amends former blanket transition prohibitions.'
 if n['id']=='INV-F11':n['text']='X local supported usage lows remain; R computes the separate conservative supported-credit floor per scope under SUPPORTED-LOW. A financial unit is not necessarily an independent physical bill.'
 if n['id']=='CORRELATION':n['text']=texts['INITIAL-REFERENCE']+'\n'+texts['ATOMIZATION']+'\n'+texts['LOGICAL-PRIMARY']+'\n'+texts['FINANCIAL-CANDIDATES']
 if n['id']=='BOUNDARY-CONSUMERS':n['text']='T owns origin/unit/slot attribution and admitted/possible memberships, never money. R sums unique active highs (max per unit; AtLeast0 preserves unresolvedness), recomputes SUPPORTED-LOW within each scope, and owns commitment/provisionality. Findings do not determine money.'
 if n['id']=='SETTLEMENT-WITNESS':n['text']=texts['LOGICAL-PRIMARY']
i=json.loads((p/'interface.json').read_text());i['contract']=s['contract_version'];i['revision']=8;i['accounting_evidence']={'schema':'spec/accounting.schema.json','kinds':['accounting_input_proof','accounting_result','accounting_expected_assertion'],'attempt_reference':'accounting_result_ref','rollup_reference':'supported_low_evidence_ref','low_semantics':'scope supported credits; never sum local attempt lows'}
for n in s['invariants']:
 if n['id']=='SERIALIZATION':n['text']=json.dumps(i,sort_keys=True)
s['owner_policies'][0]['text']='Accepted PLAN43d0e7c8 and ROOT-ADOPTION-R6 bind revision8, retaining P1–P6/D3. Immutable reference bases, final-fact atomization, supported low credits and exact F coverage explicitly amend former policies. No reducer/solver or live qualification.'
dump('interface.json',i);dump('ck.json',s)
q=json.loads((p/'ck.schema.json').read_text());q['properties']['contract_version']={'const':s['contract_version']};q['$defs']['StageId']={'enum':['E','C','S','V','F','X','K']};q['$defs']['BoundaryId']={'enum':['C-primary','C-validity','C-auxiliary','S-consume']};q['$defs']['Stage']['properties']['id']={'$ref':'#/$defs/StageId'}
import copy
q['$defs']['Boundary']=copy.deepcopy(q['$defs']['Stage']);q['$defs']['Boundary']['properties']['id']={'$ref':'#/$defs/BoundaryId'};q['properties']['boundaries']['items']={'$ref':'#/$defs/Boundary'}
dump('ck.schema.json',q)
d=json.loads((p/'dependencies.json').read_text());semantic=list(texts)
for c in d['consumers']:
 if c['id'] in ['view-contract','view-schema','view-transitions','oracle-cases','consumer-T','consumer-R']:
  c['dependencies']=sorted(set(c['dependencies']+semantic+['USAGE-PRESENCE','VALUATION','SETTLEMENT-WITNESS']))
for id,owner in [('python-validator','validator/validate_corpus.py'),('independent-raw-assertions','tooling/spec-check/tests/independent_raw_axes.rs'),('corpus-wire','corpus/cases')]:d['consumers'].append({'id':id,'owner':owner,'dependencies':semantic+['USAGE-PRESENCE','VALUATION','X-DECISION','SETTLEMENT-WITNESS'],'acknowledgement':None})
dump('dependencies.json',d)
