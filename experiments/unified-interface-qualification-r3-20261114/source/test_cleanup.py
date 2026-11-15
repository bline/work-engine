import unittest,tempfile,pathlib,json,sys,runpy,time,types,os
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
from realization import Actor,sha
BASE=pathlib.Path("/home/bline/cbm-work/unified-interface-sprint-20261114/Q-repair-r3/unit");BASE.mkdir(parents=True,exist_ok=True)
B=dict(ready_sha256="unit-only-not-finalized",closure_sha256="unit-closure",run_id="unit-run",invocation_id="unit-invocation")
def ctx(root,actor):
 a=Actor(types.SimpleNamespace(r=dict(owner_label_key="u2.private.owner",root_session_id="synthetic-root",host_executables={}),t={"L":60}),dict(B,evidence=str(root)),actor,time.monotonic()+60)
 if not (root/'local-budget.bin').exists():a.establish_budget()
 else:
  a.budget_fd=os.open(root/'local-budget.bin',os.O_RDWR);a.budget=__import__('mmap').mmap(a.budget_fd,8)
 return a
def confirm(root):
 a=ctx(root,"controller");cp=root/"controller-closure.json";a.save(root/"cleanup-confirmed.json",dict(container=CID,terminal_removed=True,closure=dict(path=str(cp),sha256=sha(cp))))
sys.path.insert(0,str(pathlib.Path(__file__).parent));import cleanup as c
CID='a'*64;SID='01a1d78f-b6d0-7cc2-a9a8-a56046c4f7d9'
def present(running=False):return {'returncode':0,'stdout':json.dumps([{'Id':CID,'Name':'/private','Config':{'Labels':{c.LABEL:SID,c.LABEL+'.run':B['run_id'],c.LABEL+'.invocation':B['invocation_id']}},'State':{'Running':running,'Pid':8 if running else 0}}]),'stderr':''}
def absent():return {'returncode':1,'stdout':'[]\n','stderr':f'Error response from daemon: No such container: {CID}\n'}
def ok():return {'returncode':0,'stdout':CID+'\n','stderr':''}
BAD=[{'returncode':1,'stdout':'','stderr':'daemon error'},{'returncode':None,'stdout':'','stderr':'timeout','exception':'TimeoutExpired'},{'returncode':0,'stdout':'','stderr':''},{'returncode':0,'stdout':'{}','stderr':''},{'returncode':0,'stdout':'[]','stderr':''},{'returncode':0,'stdout':present()['stdout'].replace(CID,'b'*64),'stderr':''},{'returncode':1,'stdout':'[]\n','stderr':f'Error response from daemon: No such container: other\n'}]
BAD.append(dict(returncode=0,stdout=present()['stdout'].replace(SID,'wrong-owner'),stderr=''))
class Tests(unittest.TestCase):
 def run_case(self,actor,responses):
  root=pathlib.Path(tempfile.mkdtemp(dir=BASE));it=iter(responses);result=c.cleanup(CID,root,actor,SID,lambda *a:next(it),ctx(root,actor));return root,result
 def test_unknown_initial_and_final(self):
  for actor in ['controller','watchdog']:
   for bad in BAD:
    for seq in [[bad],[present(),ok(),bad],[present(True),ok(),bad]]:
     with self.subTest(actor=actor,response=bad,stage=len(seq)):
      root,result=self.run_case(actor,seq);self.assertFalse(result);self.assertFalse(list(root.glob('*-absent.json')));self.assertFalse(list(root.glob('*-closure.json')));self.assertFalse((root/'cleanup-confirmed.json').exists());self.assertIn('container_unknown',(root/(actor+'-unresolved.json')).read_text())
 def test_disarm_requires_positive_evidence(self):
  root,result=self.run_case('controller',[BAD[0]]);self.assertFalse(c.confirmation(root,CID,ctx(root,"controller").b));(root/'cleanup-confirmed.json').write_text(json.dumps({'id':CID,'terminal_removed':True}));self.assertFalse(c.confirmation(root,CID,ctx(root,"controller").b))
  root,result=self.run_case('controller',[present(),ok(),absent()]);confirm(root);self.assertTrue(c.confirmation(root,CID,ctx(root,"controller").b));self.assertFalse(c.confirmation(root,'b'*64,ctx(root,'controller').b))
 def test_normal(self):
  for actor in ['controller','watchdog']:
   root,result=self.run_case(actor,[present(True),ok(),present(),ok(),absent()]);self.assertTrue(result);self.assertTrue((root/(actor+'-closure.json')).exists())
 def test_exact_absence_without_history(self):
  root,result=self.run_case('watchdog',[absent(),absent()]);self.assertFalse(result);self.assertTrue((root/'watchdog-absent.json').exists());self.assertFalse((root/'watchdog-closure.json').exists())
 def test_race(self):
  root,result=self.run_case('controller',[present(),ok(),absent()]);self.assertTrue(result)
  self.assertTrue(c.cleanup(CID,root,'watchdog',SID,lambda *a:absent(),ctx(root,'watchdog')));v=json.loads((root/'watchdog-closure.json').read_text());self.assertIn('controller-removal-fact',v['removal_facts'][0]['path']);self.assertFalse((root/'watchdog-removal-fact.json').exists())
 def test_unknown_remove_without_peer(self):
  for actor in ['controller','watchdog']:
   root,result=self.run_case(actor,[present(),BAD[0],absent()]);self.assertFalse(result);self.assertFalse((root/(actor+'-closure.json')).exists())
 def test_race_lost_remove_with_peer(self):
  root,result=self.run_case('controller',[present(),ok(),absent()]);it=iter([present(),absent(),absent()]);self.assertTrue(c.cleanup(CID,root,'watchdog',SID,lambda *a:next(it),ctx(root,'watchdog')));self.assertFalse((root/'watchdog-removal-fact.json').exists())
 def test_actual_watchdog_dispatch(self):
  from test_entrypoint import fixture,execute
  for scenario,expected in [({},0),({'watchdog_unknown':[1]},2)]:
   root=fixture(scenario);code,v=execute(root,'watchdog');self.assertEqual(v['watchdog_exit'],expected,(root,v))
 def test_peer_binding_and_confirmation_tamper(self):
  root,result=self.run_case('controller',[present(),ok(),absent()]);self.assertTrue(result);confirm(root);self.assertTrue(c.confirmation(root,CID,ctx(root,"controller").b))
  cp=root/'controller-removal-fact.json';value=json.loads(cp.read_text());value['binding']['invocation_id']='other';cp.write_text(json.dumps(value))
  self.assertFalse(c.confirmation(root,CID,ctx(root,"controller").b));self.assertFalse(c.cleanup(CID,root,'watchdog',SID,lambda *a:absent(),ctx(root,'watchdog')))
 def test_wrong_run_label(self):
  r=present(True);r['stdout']=r['stdout'].replace(B['invocation_id'],'other-invocation');root,result=self.run_case('controller',[r]);self.assertFalse(result);self.assertFalse(list(root.glob('*-terminal-fact.json')))
 def test_malformed_identity(self):
  for bad in BAD:self.assertEqual(c.decode(bad,CID,SID)['kind'],'Unknown')
if __name__=='__main__':unittest.main()
