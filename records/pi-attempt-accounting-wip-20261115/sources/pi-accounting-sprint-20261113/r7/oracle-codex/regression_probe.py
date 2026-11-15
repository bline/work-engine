import importlib.util,json,pathlib,copy
root=pathlib.Path('/home/bline/cbm-work');old=root/'p136-s1r6/s1';new=root/'p137-s1r7/s1'
def module(name,path):
 s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
baseline=module('baseline',old/'validator/validate_corpus.py');current=module('current',new/'validator/validate_corpus.py')
c=json.loads((new/'corpus/cases/VC02-anthropic-usage-settlement.json').read_text());a=c['expected']['attempts'][0]
rows=[json.loads(x) for x in c['inputs']['sessions'][0]['logs']['turn-1']['lines']];request=next(x for x in rows if x['event']=='request')
for axis,edit in [('tier_default',lambda x:x.update(observed_tier='default')),('inconsistent_lifetimes',lambda x:x['usage'].update(ephemeral_5m_input_tokens=x['usage']['cache_creation_input_tokens']+1))]:
 r=copy.deepcopy(request);edit(r);before=baseline.Failures();after=current.Failures()
 baseline.raw_usage_policy_checks('raw-witness',a,[r],before);current.raw_usage_policy_checks('raw-witness',a,[r],after)
 assert not before,(axis,before);assert after,(axis,after)
 print(json.dumps({'axis':axis,'baseline_accepted':True,'current_rejected':True,'reasons':after}))
print('bounded regression reproduction PASS; no reducer or live experiment')
