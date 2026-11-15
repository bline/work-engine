"""Controlled capture assumptions and faults across actual production consumers."""
import io,json,pathlib,sys,time,types,unittest,unittest.mock as mock,subprocess,os
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import realization as z
import test_entrypoint as e
class Custody(unittest.TestCase):
 def test_publication_matrix(self):
  for name,ambiguous,mode in [('create.json',False,'success'),('ambiguous-create-custody.json',True,'success'),('ownership.json',False,'success'),('watchdog-context.json',False,'success'),('watchdog-ownership.json',False,'success'),('watchdog-start.json',False,'watchdog'),('controller-terminal-fact.json',False,'success'),('controller-removal-fact.json',False,'success'),('watchdog-terminal-fact.json',False,'watchdog'),('watchdog-removal-fact.json',False,'watchdog')]:
   with self.subTest(name=name):
    root=e.fixture(dict(fail_fact=[name],ambiguous=ambiguous));code,v=e.execute(root,mode)
    self.assertNotEqual(code,0,(root,name));self.assertEqual(v['container'],'a'*64)
    self.assertTrue(v['custody']['responses']);self.assertTrue(all(p['collected'] for p in v['custody']['children']))
    self.assertFalse(z.read(root/'daemon.json')['present'],(root,v['errors']))
    if name=='watchdog-start.json':
     w=z.read(next(root.glob('evidence/*/*/watchdog-custody.json')));self.assertEqual(w['container'],'a'*64);self.assertTrue(w['custody']['responses'])
    if 'terminal-fact' in name or 'removal-fact' in name:
     actor=name.split('-')[0];out=pathlib.Path(v['binding']['evidence'])
     # Missing fact blocks this actor's own closure unless a validated peer supplied it.
     if (out/(actor+'-closure.json')).exists():
      closure=z.read(out/(actor+'-closure.json'));kind='terminal' if 'terminal' in name else 'removal'
      self.assertTrue(all(f['actor']!=actor for f in closure[kind+'_facts']))
 def test_actual_controller_final_capture(self):
  for fault,expected in [('stdout',True),('all',False)]:
   root=e.fixture();code,v=e.execute(root,'success',fault=fault);self.assertNotEqual(code,0)
   delivered=z.read(root/'outer-delivery.json');self.assertEqual(delivered['delivered'],expected)
   if expected:self.assertEqual(v['container'],'a'*64);self.assertTrue(v['closed']);self.assertTrue(v['custody']['children'])
   self.assertFalse(z.read(root/'daemon.json')['present'])
 def test_accounting_projection_failure(self):
  root=e.fixture(dict(block_budget_publication=True));code,v=e.execute(root,'success');self.assertNotEqual(code,0);self.assertFalse(z.read(root/'daemon.json')['present'])
  out=pathlib.Path(v['binding']['evidence']);import struct
  total=struct.unpack('d',(out/'local-budget.bin').read_bytes())[0];self.assertGreater(total,0);self.assertLessEqual(total,z.Subject(root).t['L']);self.assertTrue(v['custody']['publication_errors'])
 def test_deadline_collection_and_final_capture(self):
  # Actual client acquired, then absolute clock boundary exhausted; no extra reap grace.
  root=e.Q/('capture-'+__import__('uuid').uuid4().hex);root.mkdir(parents=True)
  subject=types.SimpleNamespace(r=dict(owner_label_key='owner',host_executables=dict(docker='/usr/bin/python3.14')),t=dict(L=2,R=.01,S=.01))
  a=z.Actor(subject,dict(evidence=str(root),run_id='capture',invocation_id='capture'),'controller',time.monotonic()+.01);a.establish_budget()
  response=a.rpc('-c','import time;time.sleep(1)');self.assertIsNone(response['returncode']);self.assertTrue(a.children)
  proc,row=a.children[0]
  # The exact same owner collects after recording the production deadline outcome; qualification does not grant production grace.
  before=a.snapshot();proc.wait(timeout=2);proc.stdout.close();proc.stderr.close();after=dict(pid=proc.pid,exit=proc.returncode,method='test owner exact wait; outside production deadline claim')
  z.write(root/'qualification-owner-collection.json',after)
  class Broken:
   name='failed-stream'
   def write(self,*args):raise OSError('injected final output failure')
   def flush(self):raise OSError('injected flush failure')
  captured=io.StringIO()
  with mock.patch.object(z,'write',side_effect=OSError('injected unresolved/final fact failure')),mock.patch.object(sys,'stdout',Broken()),mock.patch.object(sys,'stderr',captured):
   a.save(root/'controller-unresolved.json',dict(container='b'*64,closed=False))
   a.deliver(root/'custody.json',dict(container='b'*64,closed=False))
  delivered=json.loads(captured.getvalue());self.assertEqual(delivered['container'],'b'*64);self.assertEqual(delivered['custody']['children'][0]['pid'],proc.pid)
  z.write(root/'collector-delivered.json',dict(delivered=True,capture_route='stderr buffer, explicit available bounded capture assumption',payload=delivered,before=before,owner_collection=after))
  with mock.patch.object(z,'write',side_effect=OSError('all filesystem delivery failed')),mock.patch.object(sys,'stdout',Broken()),mock.patch.object(sys,'stderr',Broken()):
   attempted=a.deliver(root/'missing.json',dict(container='b'*64,closed=False))
  self.assertFalse((root/'missing.json').exists());self.assertEqual(len(attempted['delivery_attempt_errors']),3)
  z.write(root/'collector-missing.json',dict(delivered=False,claim='UNESTABLISHED',attempted=attempted,reason='all authorized capture channels failed'))
 def test_collection_exception_retained(self):
  root=e.Q/('collection-'+__import__('uuid').uuid4().hex);root.mkdir(parents=True)
  subject=types.SimpleNamespace(r=dict(owner_label_key='owner',host_executables=dict(docker='/usr/bin/python3.14')),t=dict(L=2,R=.1,S=.1))
  a=z.Actor(subject,dict(evidence=str(root),run_id='collection',invocation_id='collection'),'controller',time.monotonic()+2);a.establish_budget()
  original=subprocess.Popen.communicate
  def broken(proc,*args,**kw):raise OSError('injected collection exception')
  with mock.patch.object(subprocess.Popen,'communicate',broken):response=a.rpc('-c','import time;time.sleep(1)')
  proc,row=a.children[0];self.assertFalse(row['collected']);self.assertIn('collection exception',row['collection_error']);captured=io.StringIO()
  with mock.patch.object(sys,'stdout',captured):a.deliver(root/'custody.json',dict(container='c'*64,closed=False))
  self.assertEqual(json.loads(captured.getvalue())['custody']['children'][0]['pid'],proc.pid)
  proc.wait(timeout=2);proc.stdout.close();proc.stderr.close();z.write(root/'qualification-owner-collection.json',dict(pid=proc.pid,exit=proc.returncode))
if __name__=='__main__':unittest.main()
