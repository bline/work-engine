import json,pathlib,copy
R=pathlib.Path('/home/bline/cbm-work/p136-s1r6/s1');C=R/'corpus/cases'
for src,cid,kind in [('P133-A01-incomplete-usage-above-reservation','R6-C01-writer-reservation-floor-control','floor'),('P133-A02-error-status-with-priceable-usage','R6-C02-non200-success-terminal-usage-control','error')]:
 c=json.loads((C/(src+'.json')).read_text());c['case_id']=cid;c['added_in']='1.5-s1';c['group']='R6 independent controls';c['role']='control';c['class']='adversarial_synthetic';c['reachability']='not_established';c['findings']=[];c['controls']=[];c['historical']={};c['provenance']={'origin':'independent raw oracle writer-control construction','source_case':src};
 for key in ['revisions','revisions_r3','revisions_r4','revisions_r5','revisions_r6']:c[key]=[]
 log=c['inputs']['sessions'][0]['logs']['turn-1'];raw=[json.loads(x)for x in log['lines']]
 if kind=='floor':
  raw[0]['input_bound']=1000;raw[1]['requested_tier']='flex';c['notes']='Writer-consistent reservation arithmetic control: configured flex0.5, input bound1000, output cap150 gives (1000*12.5+150*50)*0.5/1e6=.01. Observed default prices logged usage1000/100 at.015; incomplete terminal retains R and supported usage. Local writer arithmetic/branch witness only; no live producer reachability claim.'
 else:
  raw[1]['terminal']='response.completed';raw[1]['response_id']='resp_non200_control';raw[1]['error_head']='data: terminal response fixture';c['notes']='Writer-consistent non200 terminal control: stream parser accepts terminal data regardless of HTTP status; finish logs HTTP429, response.completed and usage while error settlement charges0. Configured default input6000/outputcap14500 gives R.8. Independently supported usage1USD exceedsR. Temporal closure retains AtLeast1USD. No live producer reachability claim.'
 log['lines']=[json.dumps(x)for x in raw];c['rationale']='RootP2 explicit status/terminal conjunction; historical synthetic bytes retained in source case. '+c['notes'];(C/(cid+'.json')).write_text(json.dumps(c,indent=1,sort_keys=True)+'\n')
print('added2writercontrols')
