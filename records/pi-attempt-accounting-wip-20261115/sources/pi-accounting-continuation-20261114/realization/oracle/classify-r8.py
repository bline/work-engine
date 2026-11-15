# Inventory/classification only. Does not calculate financial results.
import hashlib,json
from pathlib import Path
r=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1'); old=Path('/home/bline/cbm-work/p137-s1r7/s1')
items=[]
for p in sorted((r/'corpus/cases').glob('*.json')):
 c=json.loads(p.read_text()); before=json.loads((old/'corpus/cases'/p.name).read_text()); rows={}; bad=[]
 for s in c['inputs']['sessions']:
  for t,l in s['logs'].items():
   for i,line in enumerate(l['lines'],1):
    ref=f'{s["alias"]}/{t}:{i}'
    try: row=json.loads(line)
    except (TypeError,ValueError):bad.append(ref); continue
    if isinstance(row,dict):rows[ref]=row
 anchors=[]; unit_changes=[]; conflicts=[]; dollar_lane=[]; malformed=[]
 for a in before['expected']['attempts']:
  for aux in a['correlation'].get('auxiliary',[]):
   anchors.append({'origin_ref':aux['evidence'],'old_unit':a['key'],'old_attachment':aux['rule'],'required_projection':'separate unresolved AtLeast0 anchor unless affirmative exact co-reference; context remains candidate for primary usage; numeric primary highs retained','standing':'bounded projection amended; full raw-to-wire REQUIRED/UNMET' if any(x['evidence']==[aux['evidence']] and x['exposure'].get('high',{}).get('kind')=='at_least' for x in c['expected']['attempts']) else 'REQUIRED/UNMET'})
  if a['exposure']['pool']=='codex' and any(rows.get(ref,{}).get('event')=='usd_reserved' or isinstance(rows.get(ref,{}).get('usd'),dict) for ref in a['evidence']):dollar_lane.append(a['key'])
  for ref in a['evidence']:
   row=rows.get(ref,{})
   if 'model' in row and not isinstance(row['model'],str):malformed.append(ref)
  groups={}
  for ref in a['evidence']:
   row=rows.get(ref,{})
   if row.get('event')=='usd_reserved':groups.setdefault(json.dumps(row,sort_keys=True),[]).append(ref)
  for refs in groups.values():
   if len(refs)>1 and len({next(s['logs'][t]['lines'][int(ref.rsplit(':',1)[1])-1] for s in c['inputs']['sessions'] for t in s['logs'] if ref.startswith(s['alias']+'/'+t+':')) for ref in refs})>1:conflicts.append(refs)
 primary={}
 for a in before['expected']['attempts']:
  for ref in a['evidence']:
   row=rows.get(ref,{}); rid=(row.get('usd') or {}).get('rid') if isinstance(row.get('usd'),dict) else row.get('rid')
   if row.get('event')=='usd_reserved' or isinstance(row.get('usd'),dict):
    if isinstance(rid,int) and not isinstance(rid,bool): primary.setdefault((ref.split(':')[0],rid),set()).add(a['key'])
 for (source,rid),keys in primary.items():
  if len(keys)>1:unit_changes.append({'scope':source,'rid':rid,'old_unit_keys':sorted(keys),'required_projection':'immutable initial TicketClaim under conditional scope; logical primary group separate from conservative high units','standing':'REQUIRED/UNMET'})
 changed=[x['field'] for x in c['revisions_r8']]
 items.append({'case_id':c['case_id'],'raw_input_sha256':hashlib.sha256(json.dumps(c['inputs'],sort_keys=True,separators=(',',':')).encode()).hexdigest(),'raw_unchanged':c['inputs']==before['inputs'],'historical_unchanged':c['historical']==before['historical'],'revised_fields':changed,'extra_zero_anchor_projections':anchors,'initial_base_projection_changes':unit_changes,'normalized_json_cardinality_conflicts':conflicts,'dollar_evidence_in_codex_pool':dollar_lane,'malformed_model_origins':sorted(set(malformed)),'unreadable_origins_preserved':bad,'local_vs_rollup':'local supported lows are usage evidence; aggregate independence is separately admitted; matching/different IDs and source positions are insufficient','attribution':'new anchors require copied possible-owner inventory; existing owner assignments not independently rederived here','full_raw_to_wire':'REQUIRED/UNMET; inventory and finite claims do not establish full conformance'})
(r/'oracle/R8-CORPUS-CLASSIFICATION.json').write_text(json.dumps({'schema':'r8-corpus-policy-classification/1','policy_ref':'PLAN43d0e7c8','cases':items,'count':len(items)},indent=2,sort_keys=True)+'\n')
print(json.dumps({'count':len(items),'changed':sum(bool(x['revised_fields']) for x in items),'anchor_cases':[x['case_id'] for x in items if x['extra_zero_anchor_projections']],'primary_base_cases':[x['case_id'] for x in items if x['initial_base_projection_changes']],'dollar_lane_cases':[x['case_id'] for x in items if x['dollar_evidence_in_codex_pool']]}))
