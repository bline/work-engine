import os,sys,json,pathlib,hashlib,tempfile,shutil,subprocess
S=pathlib.Path(__file__).parent/'subject'
B=pathlib.Path(__file__).parent/'worker/cargo-target/debug/deps/independent_raw_axes-3ee1ad2ed02bb648'
names=['equal-reservations','model7-VC01-openai-usage-settlement','model7-A04-undated-settlement-between-dated-neighbours']
manifest=lambda:{str(p.relative_to(S)):hashlib.sha256(p.read_bytes()).hexdigest() for p in S.rglob('*') if p.is_file()}
before=manifest();rows=[]
with tempfile.TemporaryDirectory(prefix='r10-eval-premise-') as td:
 T=pathlib.Path(td)/'subject';shutil.copytree(S,T)
 for name in names:
  p=T/('oracle/r9-pilot/cases/'+name+'.json');original=p.read_bytes();c=json.loads(original);inputs=json.dumps(c['inputs'],sort_keys=True);expected=json.dumps(c['evaluations'][0]['expected'],sort_keys=True)
  c['evaluations'][0]['objectives'][0]['started_at']='2026-11-10T06:55:00Z'
  assert json.dumps(c['inputs'],sort_keys=True)==inputs and json.dumps(c['evaluations'][0]['expected'],sort_keys=True)==expected
  p.write_text(json.dumps(c,indent=2)+'\n');env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1',R9_QUALIFICATION_SUBJECT=str(T))
  for consumer,cmd in [('rust',[str(B),'--exact','r9_registered_actual_subject_qualification','--nocapture']),('python',[sys.executable,'validator/validate_corpus.py','check','corpus','--pilot'])]:
   z=subprocess.run(cmd,cwd=T,env=env,capture_output=True,text=True,timeout=60)
   row={'case':name,'mutation':'only objective.started_at 06:00 to06:55','consumer':consumer,'returncode':z.returncode,'raw_inputs_unchanged':True,'expected_outputs_unchanged':True,'stdout':z.stdout,'stderr':z.stderr};rows.append(row);print(json.dumps(row),flush=True)
  p.write_bytes(original)
assert before==manifest()
print(json.dumps({'executions':len(rows),'all_falsely_accepted':all(x['returncode']==0 for x in rows),'source_files_unchanged':len(before)}))
assert len(rows)==6 and all(x['returncode']==0 for x in rows)
