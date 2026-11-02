"""Native adapter contract tests; no backend/model launch."""
import contextlib,json,pathlib,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts'))
import role_host
SID='00000000-0000-0000-0000-000000000001'
class Connection:
 def __init__(self,pages=None,thread=None):self.calls=[];self.pages=pages or {};self.thread=thread or {'id':SID,'status':{'type':'idle'}}
 def __enter__(self):return self
 def __exit__(self,*a):pass
 def call(self,method,params):
  self.calls.append((method,params))
  if method=='thread/start':return {'thread':self.thread,'sandbox':{'type':'readOnly'}}
  if method=='thread/read':return {'thread':self.thread}
  if method=='thread/name/set':return {}
  if method=='thread/turns/list':return self.pages.get(params.get('cursor'),{'data':[]})
  if method=='turn/start':return {'turn':{'id':'started'}}
  raise AssertionError(method)
class AdapterTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.h=role_host.Codex(codex_home=self.tmp.name);self.ledger=role_host.Ledger(pathlib.Path(self.tmp.name)/'state')
 def test_prepare_never_starts_inference_and_keeps_realization(self):
  c=Connection()
  with patch.object(self.h,'_conn',return_value=c):v=self.h.prepare(cwd=self.tmp.name,name='role',host_args=['-s','read-only'],ledger=self.ledger,parent=SID)
  self.assertFalse(any(m=='turn/start' for m,_ in c.calls));self.assertEqual(v['realized']['sandbox']['type'],'readOnly');self.assertEqual(self.ledger.rows()[0]['session_id'],SID)
 def test_naming_failure_preserves_created_identity(self):
  c=Connection();original=c.call
  def call(m,p):
   if m=='thread/name/set':raise role_host.codex_app.AppServerError('naming failed')
   return original(m,p)
  c.call=call
  with patch.object(self.h,'_conn',return_value=c):v=self.h.prepare(cwd=self.tmp.name,name='role',host_args=[],ledger=self.ledger)
  self.assertEqual(v['session_id'],SID);self.assertIsNotNone(v['naming_error'])
 def test_start_refuses_existing_turn_even_when_idle(self):
  c=Connection(pages={None:{'data':[{'id':'old','status':'completed'}]}})
  with patch.object(self.h,'_conn',return_value=c),self.assertRaises(role_host.HostError):self.h.start_prepared(SID,message='work',ledger=self.ledger)
  self.assertFalse(any(m=='turn/start' for m,_ in c.calls))
 def test_first_turn_records_exact_id(self):
  with patch.object(self.h,'_conn',return_value=Connection()):v=self.h.start_prepared(SID,message='work',ledger=self.ledger,effort='medium')
  self.assertEqual(v['turn_id'],'started')
 def test_exact_turn_pages_beyond_twenty_and_ignores_latest(self):
  c=Connection(pages={None:{'data':[{'id':str(i),'status':'completed'} for i in range(100)],'nextCursor':'older'},'older':{'data':[{'id':'target','status':'failed'}]}})
  with patch.object(self.h,'_conn',return_value=c):v=self.h.exact_turn(SID,'target')
  self.assertEqual(v['status'],'failed');self.assertEqual(v['history_count'],101)
 def test_absent_and_duplicate_turn_remain_unestablished(self):
  for data in [[],[{'id':'x'},{'id':'x'}]]:
   with patch.object(self.h,'_conn',return_value=Connection(pages={None:{'data':data}})):self.assertEqual(self.h.exact_turn(SID,'x')['status'],'not_established')
 def test_repeating_cursor_cannot_claim_history_complete(self):
  c=Connection(pages={None:{'data':[],'nextCursor':'same'},'same':{'data':[],'nextCursor':'same'}})
  with patch.object(self.h,'_conn',return_value=c),self.assertRaises(role_host.HostError):self.h.exact_turn(SID,'target')
 def test_rollout_requires_actual_matching_meta(self):
  p=pathlib.Path(self.tmp.name)/'sessions'/f'rollout-{SID}.jsonl';p.parent.mkdir();p.write_text(json.dumps({'type':'session_meta','payload':{'id':SID}})+'\n')
  with patch.object(self.h,'_conn',return_value=Connection()):self.assertEqual(self.h.identity(SID)['rollout'],str(p.resolve()))
  p.write_text('{}\n')
  with patch.object(self.h,'_conn',return_value=Connection()),self.assertRaises(role_host.HostError):self.h.identity(SID)
if __name__=='__main__':unittest.main()

class TurnSettingsTests(unittest.TestCase):
 def read(self,records,turn):
  with tempfile.TemporaryDirectory() as d:
   p=pathlib.Path(d)/'rollout.jsonl';p.write_text('\n'.join(json.dumps(v) for v in [{'type':'session_meta','payload':{'id':SID}},*records])+'\n')
   return role_host.codex_settings(SID,p,turn)
 def context(self,turn,model='model'):
  return {'type':'turn_context','payload':{'turn_id':turn,'model':model,'effort':'medium'}}
 def setting(self,model='model'):
  return {'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':model,'reasoning_effort':'medium','service_tier':'default'}}}
 def test_later_turn_matching_settings_cannot_replace_target_mismatch(self):
  v=self.read([self.context('A','wrong'),self.setting('wrong'),self.context('B'),self.setting()],'A')
  self.assertEqual(v['model'],'wrong');self.assertEqual(v['turn_id'],'A');self.assertEqual(v['turn_attribution'],'exact_turn_interval')
 def test_retained_target_cannot_reuse_old_session_event(self):
  with self.assertRaises(role_host.HostError):self.read([self.context('A'),self.setting(),self.context('B')],'B')
 def test_unattributed_and_midturn_changing_settings_refuse(self):
  for records in [[self.setting()],[self.context('A'),self.setting(),self.setting('changed')]]:
   with self.assertRaises(role_host.HostError):self.read(records,'A')
