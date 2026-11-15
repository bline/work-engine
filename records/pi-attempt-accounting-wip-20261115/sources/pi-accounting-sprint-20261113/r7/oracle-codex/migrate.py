import json,copy,pathlib
r=pathlib.Path('/home/bline/cbm-work/p137-s1r7/s1')
def unresolved(a): return a['obligation']['state']=='outstanding' or a['exposure'].get('high',{}).get('kind')=='at_least'
changes=[]
for p in sorted((r/'corpus/cases').glob('*.json')):
 c=json.loads(p.read_text()); c['contract']='pi-accounting-contract/1.6-s1'; c['revisions_r7']=[]
 ats={a['key']:a for a in c['expected']['attempts']}
 for e in c['evaluations']:
  for o in e['expected']['objectives']:
   name=o['objective']; attrib=e['expected']['attribution']
   want=any(unresolved(ats[t['attempt']]) for t in attrib if t['ownership']=='owned' and t['objective']==name) or o['predecessor_obligations']['high']['kind']=='at_least' or any(cm['state']!='invalid' and cm.get('kind')=='at_least' for cm in o['commitments']) or any(ats[t['attempt']]['exposure']['pool']=='pi_api' and unresolved(ats[t['attempt']]) for t in attrib if t['ownership'] in ('uncertain','unadmitted') and name in t.get('candidates',[]))
   if o['provisional']!=want:
    c['revisions_r7'].append(dict(rule='R7-root-financial-provisionality: owned unresolved billing/predecessor AtLeast/noninvalid AtLeast commitment/unresolved PiApi candidate; lifecycle cannot clear money',field=f"{e['name']}/{name}.provisional",**{'from':o['provisional'],'to':want}));o['provisional']=want
 if c['case_id'] in ('U08-missing-observed-tier','P130-R02-rounded-up-charge-admitted'):
  for k,v in [('class','adversarial_synthetic'),('reachability','producer_unreachable')]:
   if c[k]!=v:c['revisions_r7'].append(dict(rule='R7-pinned-writer-floor-witness',field=k,**{'from':c[k],'to':v}));c[k]=v
  old=c['notes']; c['notes']=old+' R7 independent pinned-writer witness: exact raw floor0 is synthetic; missing observed tier requires floor>=2 (U08), requested null/default with observed flex requires floor>=1 (P130-R02). pi_container.py:964-966,1007-1020; original raw bytes preserved. New controls do not establish these old bytes reachable.'
  c['revisions_r7'].append(dict(rule='R7-pinned-writer-floor-witness',field='notes',**{'from':old,'to':c['notes']}))
 changes.extend([{'case_id':c['case_id'],**x} for x in c['revisions_r7']]);p.write_text(json.dumps(c,indent=1,sort_keys=True)+'\n')
(r/'corpus/REVISION-7.json').write_text(json.dumps({'version':'1.6-s1','revision':7,'author':'independent oracle Codex SID01a1d743','authority':'oracle-codex packet b428dc44/root policy and provisional clarification','changes':changes},indent=2)+'\n')
p=r/'validator/validate_corpus.py';s=p.read_text().replace('1.5-s1, revision 6','1.6-s1, revision 7').replace("'pi-accounting-contract/1.5-s1', 'pi-accounting-contract/1.4-s1'","'pi-accounting-contract/1.6-s1', 'pi-accounting-contract/1.5-s1'")
s=s.replace("'1.4-s1', '1.5-s1'}","'1.4-s1', '1.5-s1', '1.6-s1'}")
s=s.replace("'revisions_r6', 'profile')","'revisions_r6', 'revisions_r7', 'profile')")
s=s.replace("+ c.get('revisions_r6', []):","+ c.get('revisions_r6', []) + c.get('revisions_r7', []):")
s=s.replace("R5_ADDED = ('contract', 'revisions_r6', 'rationale')","R5_ADDED = ('contract', 'revisions_r7')")
s=s.replace("reversed(u.get('revisions_r6', []))","reversed(u.get('revisions_r7', []))").replace("c.get('added_in') != '1.5-s1'","c.get('added_in') != '1.6-s1'").replace('not revision 5','not revision 6').replace('not marked added_in 1.5-s1','not marked added_in 1.6-s1')
needle="        owned = [at for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name]"
s=s.replace(needle,needle+"\n        want_provisional = financial_provisional(attempts, ee['attribution'], oe)\n        if type(oe.get('provisional')) is not bool or oe['provisional'] != want_provisional: F.add(cid, f'{e[\"name\"]}/{name}: provisional violates financial unresolved-billing predicate')")
pos=s.index('def admission_decision(')
s=s[:pos]+'''def financial_provisional(attempts, attribution, objective):
    """Bounded assertion over declared facts; no money reduction or lifecycle inference."""
    def unresolved(a): return a['obligation']['state'] == 'outstanding' or a['exposure'].get('high', {}).get('kind') == 'at_least'
    name = objective['objective']
    return (any(unresolved(attempts[t['attempt']]) for t in attribution if t['ownership'] == 'owned' and t['objective'] == name)
            or objective['predecessor_obligations']['high']['kind'] == 'at_least'
            or any(cm['state'] != 'invalid' and cm.get('kind') == 'at_least' for cm in objective['commitments'])
            or any(attempts[t['attempt']]['exposure']['pool'] == 'pi_api' and unresolved(attempts[t['attempt']]) for t in attribution
                   if t['ownership'] in ('uncertain', 'unadmitted') and name in t.get('candidates', [])))


'''+s[pos:];p.write_text(s)
p=r/'tooling/spec-check/tests/independent_raw_axes.rs';s=p.read_text().replace('let mut vals = vec![];','let mut vals = vec![];\n    let mut counts_complete = true;').replace('!matches!(s, "standard" | "default")','s != "standard"').replace('let base = i * pr("input")','counts_complete &= w5 + w1 <= cw;\n            let base = i * pr("input")').replace('terminal(row, f)))','terminal(row, f) && counts_complete))');p.write_text(s)
print('migrated',len(changes),'attributed changes')
