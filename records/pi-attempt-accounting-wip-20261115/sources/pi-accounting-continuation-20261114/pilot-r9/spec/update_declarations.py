import json,pathlib
p=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1-r9-pilot');f=p/'spec/ck.json';s=json.loads(f.read_text());changed=['LOGICAL-PRIMARY','DEFINITE-LINK','SCHEMA-ACCOUNTING','F-PROJECTION']
for x in s['invariants']:
 if x['id'] in changed:x['revision']+=1
 if x['id']=='SCHEMA-ACCOUNTING':x['text']=json.dumps(json.loads((p/'spec/accounting.schema.json').read_text()),sort_keys=True);x['depends']=sorted(set(x['depends']+['LOGICAL-PRIMARY','DEFINITE-LINK']))
 if x['id']=='LOGICAL-PRIMARY':x['text']+=' C-primary emits primary_memberships one-for-one with PrimaryValidity: disjoint canonical exact-evidence classes, minimum-member representatives and complete usage_origin_refs. Multi-origin classes require admitted exact-copy equivalence, identical raw-byte digest and scope_exact_line scope; amount equality supplies none. Distinct witness refs equal representatives. Independent raw qualification establishes inventory completeness.'
 if x['id']=='DEFINITE-LINK':x['text']+=' InputLink and DefiniteLink carry exact unique link_id; result admitted_input_link_ref names InputLink.link_id preserving directed endpoints. Exact grants carry the independently raw-qualified finite admitted_payload {from,to} or {pairs:[{from,to}]}; extent alone, self-pairs and unknown shapes fail. Coverage cites result link IDs and each released anchor links to a covered usage/primary origin of its selected obligation or genuine completion.'
 if x['id']=='F-PROJECTION':x['depends']=sorted(set(x['depends']+['LOGICAL-PRIMARY','DEFINITE-LINK']));x['text']+=' Accepted budgeted coverage selects exactly matching obligation membership and valid witness; all primary classes are monetary-covered, no other obligation primary occurs, and valuation usage equals selected membership usage. Genuine UnbudgetedCompletion carries complete usage_origin_refs and no dollar obligation; valuation usage equals this set. Whole-unit monetary/anchor coverage is disjoint and exact. Raw-derived qualification remains independent of supplied-set coherence.'
f.write_text(json.dumps(s,indent=2)+'\n')
f=p/'spec/dependencies.json';d=json.loads(f.read_text())
for x in d['consumers']:
 if x.get('id') in ['python-validator','python-corpus-reference-consumer','rust-corpus-reference-consumer','independent-raw-assertions','oracle-cases','view-contract','view-schema']:
  x['dependencies']=sorted(set(x['dependencies']+changed))
f.write_text(json.dumps(d,indent=2)+'\n')
