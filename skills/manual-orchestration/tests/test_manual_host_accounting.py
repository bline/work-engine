"""Host-facing accounting-source contract; real cost_tree, isolated lineage fixtures."""
import json,os,pathlib,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[3]/'metrics/scripts'))
import usage_ledger as L
class CanonicalLineage(unittest.TestCase):
 def observe_tree(self,duplicate=False):
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d);ledger=root/'work-engine/role-host/ledger.jsonl';ledger.parent.mkdir(parents=True)
   ledger.write_text('\n'.join(json.dumps({'event':'launch','host':'codex','session_id':sid,'parent_session_id':'root'}) for sid in (['reviewer','hosted'] if duplicate else ['reviewer']))+'\n')
   record=root/'assignment.json';record.write_text(json.dumps({'scope_budget':{'objective_id':'objective','started_at':'2026-01-01T00:00:00Z','sessions':[{'session_id':'root','provider':'codex','from':'2026-01-01T00:00:00Z'}],'bindings':[{'session_id':'hosted','provider':'codex','parent_session_id':'root'}]}}))
   calls=[dict(provider='codex',thread=sid,ts='2026-01-01T01:00:00Z',model='gpt-6.1-sol',service_tier='default',quota_multiplier=1,usd=1,pools=['codex']) for sid in ['root','reviewer','hosted','outside']]
   with patch.dict(os.environ,{'XDG_STATE_HOME':d}),patch.object(L,'transcript_available',return_value=True),patch.object(L,'build_codex',return_value=(calls,[],[],0)),patch.object(L,'build_claude',return_value=([],[],0)),patch.object(L,'unrecorded_claude',return_value=[]):
    tree,_,_=L.cost_tree(str(record))
   return tree
 def test_external_role_and_host_binding_both_count_once(self):
  tree=self.observe_tree()
  self.assertEqual(tree['status'],'complete',json.dumps(tree.get('issues')))
  self.assertEqual(tree['totals']['codex']['calls'],3);self.assertEqual(tree['totals']['codex']['quota_usd_high'],3)
  self.assertEqual({n['node_id'] for n in tree['nodes']},{'root','reviewer','hosted'})
  self.assertEqual(tree['unattributed']['totals']['codex']['calls'],1)
 def test_conflicting_source_provenance_remains_visible_without_double_count(self):
  tree=self.observe_tree(duplicate=True)
  self.assertEqual(tree['status'],'partial');self.assertEqual(tree['totals']['codex']['calls'],3)
  self.assertTrue(any(i['kind']=='multiple_parents' for i in tree['issues']))
if __name__=='__main__':unittest.main()
