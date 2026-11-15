import sys,json,pathlib,copy,builtins,io,contextlib,os
from unittest.mock import patch
r=pathlib.Path(os.environ['R9_QUALIFICATION_SUBJECT']);sys.path.insert(0,str(r/'validator'));import validate_corpus as v
baseopen=builtins.open;baseread=pathlib.Path.read_text
variants=[]
p='corpus/cases/P9-claimed-charge-above-reservation.json';c=json.loads((r/p).read_text());a=c['expected']['attempts'][0];a['settlement']['charged']='500000000';a['exposure']['high']['nano']='500000000';o=c['evaluations'][0]['expected']['objectives'][0];o['owned_outstanding']['high']['nano']='500000000';o['commitments'][0]['committed']='500000000'
bp='oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json';b=json.loads((r/bp).read_text());next(x for x in b['documents'][1]['retained_components'] if x['role']=='charge')['nano_usd']='500000000';variants.append(('p9_coherent_wrong',{p:c,bp:b}))
reg=json.loads((r/'oracle/r9-pilot/claims.json').read_text());binding=reg['claims'][0]['cases'][0];binding['control_file']='../../'+p;binding['bundle_file']=bp;variants.append(('p9_same_wrong_via_inapplicable_control_dispatch',{p:c,bp:b,'oracle/r9-pilot/claims.json':reg}))
for cid in ['VC01-openai-usage-settlement','A04-undated-settlement-between-dated-neighbours']:
 p='oracle/r9-pilot/cases/model7-'+cid+'.json';c=json.loads((r/p).read_text());a=c['expected']['attempts'][0];a['valuation']['complete']=True;a['replacement']['eligible']=True;variants.append(('model7_complete_'+cid,{p:c}))
observed={}
for name,subs in variants:
 changes={(r/k).resolve():json.dumps(val) for k,val in subs.items()}
 def patched_open(f,*args,**kwargs):
  if isinstance(f,(str,os.PathLike)) and pathlib.Path(f).resolve() in changes:return io.StringIO(changes[pathlib.Path(f).resolve()])
  return baseopen(f,*args,**kwargs)
 def patched_read(p,*args,**kwargs):return changes[p.resolve()] if p.resolve() in changes else baseread(p,*args,**kwargs)
 out=io.StringIO()
 with patch('builtins.open',patched_open),patch.object(pathlib.Path,'read_text',patched_read),contextlib.redirect_stdout(out):code=v.main(['validator','check',str(r/'corpus'),str(r/'oracle/rates'),'--pilot'])
 observed[name]=code
 print(name,'actual main virtual-read exit',code,'output',out.getvalue().strip())
assert observed["p9_coherent_wrong"] == 1
assert observed["p9_same_wrong_via_inapplicable_control_dispatch"] == 0
assert all(code==1 for name,code in observed.items() if name.startswith("model7_complete_"))
print("ROOT_REPRODUCED_EXPECTED_DEFECTS",json.dumps(observed,sort_keys=True))
