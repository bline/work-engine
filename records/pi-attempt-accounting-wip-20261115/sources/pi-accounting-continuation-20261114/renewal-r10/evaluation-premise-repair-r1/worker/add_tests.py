from pathlib import Path
p=Path('subject/validator/test_r10_qualification.py');s=p.read_text();s=s.replace("reason=('model7 unsupported minimum' if kind in ('old15M','no-release')", "reason=('finite premise' if kind in ('missing-evaluation','duplicate-evaluation','missing-objective','duplicate-objective') else 'model7 unsupported minimum' if kind in ('old15M','no-release')")
pos=s.index("if __name__=='__main__'")
s=s[:pos]+'''class PremiseBoundary(Pilot):
 # Only new tests run here; inherited tests remain with Pilot.
 def test_finite_premise_mutants(self):
  gated='corpus/cases/P132-FG-VC11-gated.json'
  eq='oracle/r9-pilot/cases/equal-reservations.json'
  admissions=v.r9_pilot.ADMITTED['admissions']
  mutants=[]
  def add(name,path,change):mutants.append((name,path,change))
  for path in sorted({a['case_file'] for a in admissions.values() if 'case_file' in a}):
   add('start0655:'+path,path,lambda c:c['evaluations'][0]['objectives'][0].update(started_at='2026-11-10T06:55:00Z'))
  for field,value in [('closed_at','2026-11-10T06:20:00Z'),('predecessor','other'),('tree_sessions',[]),('limit','1'),('admissions',[])]:
   add(field,eq,lambda c,k=field,x=value:c['evaluations'][0]['objectives'][0].update({k:x}))
  for field,value in [('session','other'),('source','other'),('admission_id','other')]:
   add('admission-'+field,eq,lambda c,k=field,x=value:c['evaluations'][0]['objectives'][0]['admissions'][0].update({k:x}))
  for field,value in [('session_id','other'),('event','pending'),('at','2026-11-10T07:00:00Z'),('admission_id','other')]:
   add('journal-'+field,eq,lambda c,k=field,x=value:c['evaluations'][0]['objectives'][0]['admissions'][0]['rows'][1].update({k:x}))
  add('row-order',eq,lambda c:c['evaluations'][0]['objectives'][0]['admissions'][0]['rows'].reverse())
  add('max_usd',eq,lambda c:c['evaluations'][0]['objectives'][0]['admissions'][0]['rows'][0].update(max_usd=2.0))
  add('number-bool',eq,lambda c:c['evaluations'][0]['objectives'][0]['admissions'][0]['rows'][0].update(max_usd=True))
  add('request',eq,lambda c:c['evaluations'][0]['admission_request'].update(new='0'))
  for field in ['profile','schema','case_id','role','class','contract','reachability']:
   for value in [None,'other',True,{}]:add(field+str(value),gated,lambda c,k=field,x=value:c.update({k:x}))
  for field in ['inputs','evaluations','profile','trusted_fixture_inputs']:
   add('delete-'+field,gated,lambda c,k=field:c.pop(k))
   for value in [None,[],{},'',False]:add('type-'+field+str(value),gated,lambda c,k=field,x=value:c.update({k:x}))
  witness=v.r9_pilot.ADMITTED['admissions']['oracle/r8-completion-A/P132-FG-VC11-gated.bundle.json']['premise_record']['trusted_fixture_inputs']['gate_verifications']['g-S1']
  for field in witness:
   add('fixture-delete-'+field,gated,lambda c,k=field:c['trusted_fixture_inputs']['gate_verifications']['g-S1'].pop(k))
   add('fixture-change-'+field,gated,lambda c,k=field:c['trusted_fixture_inputs']['gate_verifications']['g-S1'].update({k:'different'}))
  add('fixture-extra',gated,lambda c:c['trusted_fixture_inputs']['gate_verifications']['g-S1'].update(extra=True))
  add('fixture-gate-inventory',gated,lambda c:c['trusted_fixture_inputs']['gate_verifications'].update(extra={}))
  add('fixture-insertion',eq,lambda c:c.update(trusted_fixture_inputs=None))
  for level in ['evaluation','objective','row']:
   for kind in ['delete','null','extra','duplicate']:
    def change(c,level=level,kind=kind):
     ev=c['evaluations'][0];ob=ev['objectives'][0];rows=ob['admissions'][0]['rows']
     target=ev if level=='evaluation' else ob if level=='objective' else rows[0]
     key='name' if level=='evaluation' else 'started_at' if level=='objective' else 'event'
     if kind=='delete':target.pop(key)
     elif kind=='null':target[key]=None
     elif kind=='extra':target['unknown']=True
     elif level=='evaluation':c['evaluations'].append(json.loads(json.dumps(ev)))
     elif level=='objective':ev['objectives'].append(json.loads(json.dumps(ob)))
     else:rows.append(json.loads(json.dumps(rows[0])))
    add(level+'-'+kind,eq,change)
  add('top-extra',eq,lambda c:c.update(unknown=True))
  raw='oracle/r8-controls/VC07-exact.raw.json'
  add('conditional-assumption',raw,lambda c:c['conditional_harness_assumptions']['exact_row_coreference'].update(no_origin_duplication=False))
  add('conditional-original',raw,lambda c:c['original_inputs'].update(reference_time='different'))
  add('rate','oracle/rates/openai_api_rates.json',lambda c:c['models']['gpt-6-astra'].update(input=11))
  for name,path,change in mutants:
   with self.subTest(name=name):
    def mutate(r):
     before=json.loads((r/path).read_text());edit(r,path,change);after=json.loads((r/path).read_text())
     if 'expected' in before:self.assertEqual(before['expected'],after['expected'])
     for a,b in zip(before.get('evaluations',[]),after.get('evaluations',[]) or []):
      if isinstance(b,dict) and 'expected' in a:self.assertEqual(a['expected'],b['expected'])
     proc=subprocess.run([os.environ['R10_FINANCIAL_EXECUTABLE'],'--exact','r9_registered_actual_subject_qualification','--nocapture'],env=dict(os.environ,R9_QUALIFICATION_SUBJECT=str(r)),capture_output=True,text=True)
     self.assertNotEqual(proc.returncode,0,proc.stdout+proc.stderr);self.assertIn('R10 finite premise outside independent admission',proc.stdout+proc.stderr)
     self.assertNotIn('R10 evaluated',proc.stdout+proc.stderr)
    rc,msg=self.run_subject(mutate);self.assertEqual(rc,1,msg);self.assertIn('R10 finite premise outside independent admission',msg);self.assertNotIn('R10 evaluated',msg)
    print('R10 premise both consumers rejected',name)
 def test_malformed_and_duplicate_consumption(self):
  paths=['oracle/r9-pilot/cases/equal-reservations.json','oracle/r8-controls/VC07-exact.raw.json','oracle/rates/openai_api_rates.json','oracle/r9-pilot/cases/equal-reservations.bundle.json','oracle/r9-pilot/claims.json']
  for path in paths:
   for kind in ['malformed','duplicate-top','duplicate-nested']:
    with self.subTest(path=path,kind=kind):
     def mutate(r):
      p=r/path;text=p.read_text()
      if kind=='malformed':text='{'
      elif kind=='duplicate-top':text='{"duplicate":0,"duplicate":1,'+text.lstrip()[1:]
      else:text=text.replace('{','{"duplicate":0,"duplicate":1,',1)
      p.write_text(text)
      proc=subprocess.run([os.environ['R10_FINANCIAL_EXECUTABLE'],'--exact','r9_registered_actual_subject_qualification','--nocapture'],env=dict(os.environ,R9_QUALIFICATION_SUBJECT=str(r)),capture_output=True,text=True)
      self.assertNotEqual(proc.returncode,0);self.assertIn('R10 finite premise outside independent admission',proc.stdout+proc.stderr)
     rc,msg=self.run_subject(mutate);self.assertEqual(rc,1,msg);self.assertIn('R10 finite premise outside independent admission',msg);self.assertNotIn('Traceback',msg)

# Avoid duplicating the inherited legacy battery under the subclass.
for _name in list(Pilot.__dict__):
 if _name.startswith('test_'):setattr(PremiseBoundary,_name,None)
'''+s[pos:]
p.write_text(s)
