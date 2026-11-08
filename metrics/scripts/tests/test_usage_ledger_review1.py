import argparse
import base64
import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L

T0='2026-01-01T00:00:00Z'
T1='2026-01-02T00:00:00Z'
T2='2026-01-03T00:00:00Z'

def record(path, sessions, bindings=()):
    with open(path, 'w') as f:
        json.dump({'scope_budget':{'objective_id':'obj','started_at':T0,'sessions':sessions,'bindings':list(bindings)}}, f)

def session(sid, provider='codex', start=T0):
    return {'session_id':sid,'provider':provider,'from':start}

def rollout(path, sid, parent=None, calls=()):
    rows=[{'type':'session_meta','timestamp':T0,'payload':{'id':sid,'parent_thread_id':parent}},
          {'type':'turn_context','timestamp':T0,'payload':{'model':'gpt-6.1-sol'}},
          {'type':'event_msg','timestamp':T0,'payload':{'type':'thread_settings_applied','thread_id':sid,'thread_settings':{'service_tier':'default'}}}]
    for rid, ts in calls:
        rows.append({'type':'token_usage_record','timestamp':ts,'payload':{'thread_id':sid,'response_id':rid,'usage':{'input_tokens':100,'output_tokens':10}}})
    with open(path,'w') as f:
        for row in rows: f.write(json.dumps(row,separators=(',',':'))+'\n')

class Sources(unittest.TestCase):
    def test_missing_root_child_and_observed_empty(self):
        with tempfile.TemporaryDirectory() as d:
            root=os.path.join(d,'root.jsonl'); rec=os.path.join(d,'record'); missing=os.path.join(d,'absent')
            rollout(root,'root'); record(rec,[session('root')])
            with patch.object(L,'CODEX_GLOBS',[root]),patch.object(L,'CLAUDE_GLOB',os.path.join(d,'none')),patch.object(L,'thread_index',return_value={}):
                tree,_,_=L.cost_tree(rec,role_ledger=missing)
                self.assertEqual(tree['status'],'complete')
                self.assertEqual(tree['totals']['codex']['calls'],0)
                self.assertIn('role_host_ledger_absent',[i['kind'] for i in tree['issues']])
                record(rec,[session('root')],[dict(session_id='child',provider='claude',parent_session_id='root')])
                tree,_,_=L.cost_tree(rec,role_ledger=missing)
                self.assertEqual(tree['status'],'partial')
                self.assertTrue(any(i['kind']=='transcript_missing' and i['session_id']=='child' for i in tree['issues']))
                os.remove(root)
                tree,_,_=L.cost_tree(rec,role_ledger=missing)
                self.assertEqual(tree['status'],'not_established')
                self.assertTrue(any(i['kind']=='transcript_missing' and i['session_id']=='root' for i in tree['issues']))

    def test_unreadable_and_non_object_ledger(self):
        with tempfile.TemporaryDirectory() as d:
            root=os.path.join(d,'root.jsonl'); rec=os.path.join(d,'record'); ledger=os.path.join(d,'ledger')
            rollout(root,'root'); record(rec,[session('root')])
            with open(ledger,'w') as f: f.write('[]\nnull\n{"event":"retire"}\n')
            with patch.object(L,'CODEX_GLOBS',[root]),patch.object(L,'CLAUDE_GLOB',os.path.join(d,'none')),patch.object(L,'thread_index',return_value={}):
                tree,_,_=L.cost_tree(rec,role_ledger=ledger)
                self.assertEqual(tree['status'],'partial')
                self.assertEqual(sum(i['kind']=='malformed_role_host_row' for i in tree['issues']),2)
                orig=open
                def denied(path,*a,**kw):
                    if path==ledger: raise PermissionError('denied')
                    return orig(path,*a,**kw)
                with patch('builtins.open',side_effect=denied): tree,_,_=L.cost_tree(rec,role_ledger=ledger)
                self.assertEqual(tree['status'],'partial')
                self.assertIn('role_host_ledger_unreadable',[i['kind'] for i in tree['issues']])

    def test_old_intermediary_and_undated_disjoint_windows(self):
        with tempfile.TemporaryDirectory() as d:
            root=os.path.join(d,'root.jsonl'); parent=os.path.join(d,'parent.jsonl'); child=os.path.join(d,'child.jsonl'); rec=os.path.join(d,'record')
            rollout(root,'root'); rollout(parent,'parent','root'); rollout(child,'child','parent', [('call','2026-01-02T12:00:00Z'),('undated','bad')])
            old=L.ts_of('2025-12-31T00:00:00Z').timestamp()
            for p in (root,parent): os.utime(p,(old,old))
            record(rec,[session('root'),session('successor',start='2026-01-02T20:00:00Z')])
            # Root-only record makes the old parent test independent of the missing successor.
            record(rec,[session('root')])
            with patch.object(L,'CODEX_GLOBS',[os.path.join(d,'*.jsonl')]),patch.object(L,'CLAUDE_GLOB',os.path.join(d,'none')),patch.object(L,'thread_index',return_value={}):
                tree,_,_=L.cost_tree(rec,since_arg=T1,until_arg=T2,role_ledger=os.path.join(d,'missing'))
                self.assertEqual(tree['totals']['codex']['calls'],1)
                self.assertEqual(tree['unattributed']['totals']['codex']['calls'],0)
                self.assertEqual(tree['undated']['counts']['codex'],1)
                self.assertEqual(tree['status'],'partial')
                self.assertEqual(next(n for n in tree['nodes'] if n['node_id']=='child')['parent_node_id'],'parent')
                self.assertEqual(tree['conservation_scope'],'dated calls in window')
                other,_,_=L.cost_tree(rec,since_arg=T0,until_arg=T1,role_ledger=os.path.join(d,'missing'))
                self.assertEqual(other['window_totals']['codex']['calls'],0)
                self.assertEqual(other['undated']['counts']['codex'],1)

class Control(unittest.TestCase):
    def setUp(self):
        self.sb={'objective_id':'obj','limits':{'max_weekly_percent':90},'cost_limits':[{'pool':'codex','node':None,'set_by':'user','marks_usd':[1], 'max_usd':2}]}
        self.spend=L.pools(); self.spend['codex'].update(quota_usd_low=3,quota_usd_high=3)
        self.doc={'snapshot_at':'2026-01-01T01:00:00Z','status':'complete','totals':self.spend,'nodes':[]}
        self.quota=[{'ts':self.doc['snapshot_at'],'window_minutes':10080,'used_percent':95,
                     'resets_at':L.ts_of(self.doc['snapshot_at']).timestamp()+3600}]
        self.args=argparse.Namespace(record='none',cursor=None,limit_fractions='0.5,0.8,1.0',role_host_ledger=None)
    def run_control(self):
        out=io.StringIO()
        with patch.object(L,'cost_tree',return_value=(self.doc,self.quota,self.sb)),contextlib.redirect_stdout(out): L.cmd_control(self.args)
        return json.loads(out.getvalue())
    def consume(self, doc): self.args.cursor=doc['next_budget_cursor']
    def test_unpriced_subtotal_and_recovery_keeps_states(self):
        first=self.run_control(); self.assertEqual(len(first['events']),5); self.consume(first)
        self.spend['codex'].update(quota_usd_low=0,quota_usd_high=0,unpriced_calls=1); self.doc['status']='partial'; self.quota=[]
        unknown=self.run_control(); self.assertEqual([x['kind'] for x in unknown['events']],['coverage_changed']); self.consume(unknown)
        self.spend['codex'].update(quota_usd_low=3,quota_usd_high=3,unpriced_calls=0); self.doc['status']='complete'; self.quota=[{'ts':self.doc['snapshot_at'],'window_minutes':10080,'used_percent':95,'resets_at':L.ts_of(self.doc['snapshot_at']).timestamp()+3600}]
        recovered=self.run_control(); self.assertEqual([x['kind'] for x in recovered['events']],['coverage_changed']); self.consume(recovered)
        self.doc['status']='not_established'; self.quota=[]
        missing=self.run_control(); self.assertEqual([x['kind'] for x in missing['events']],['coverage_changed']); self.consume(missing)
        self.doc['status']='complete'; self.quota=[{'ts':self.doc['snapshot_at'],'window_minutes':10080,'used_percent':95,'resets_at':L.ts_of(self.doc['snapshot_at']).timestamp()+3600}]
        self.assertEqual([x['kind'] for x in self.run_control()['events']],['coverage_changed'])
    def test_known_subtotal_crosses_and_node_disappears(self):
        self.sb['limits']={}; self.sb['cost_limits']=[{'pool':'codex','node':'n','set_by':'user','marks_usd':[1],'max_usd':2}]
        self.spend['codex']['unpriced_calls']=1
        self.doc['nodes']=[{'node_id':'n','inclusive':self.spend}]
        first=self.run_control(); self.assertTrue(any(e['kind']=='node_mark_crossed' for e in first['events'])); self.assertTrue(any(e['kind']=='limit_reached' for e in first['events'])); self.consume(first)
        self.doc['nodes']=[]
        missing=self.run_control(); self.consume(missing)
        self.doc['nodes']=[{'node_id':'n','inclusive':self.spend}]; self.doc['status']='partial'
        self.assertFalse(any(e['kind'] in ('node_mark_crossed','limit_reached','limit_fraction') for e in self.run_control()['events']))
    def test_setters_coexist_and_legacy_coordinator(self):
        self.sb['limits']={}; self.sb['cost_limits']=[{'pool':'codex','node':None,'set_by':'user','marks_usd':[1],'max_usd':2}, {'pool':'codex','node':None,'set_by':'coordinator','marks_usd':[1],'max_usd':2}]
        events=self.run_control()['events']
        marks=[e for e in events if e['kind']=='mark_crossed']
        self.assertEqual({(e['set_by'],e['level']) for e in marks},{('user',1),('supervisor',1),('supervisor',2)})
        self.assertTrue(all(e['set_by']=='user' for e in events if e['kind'] in ('limit_fraction','limit_reached')))
    def test_bad_cursor_shape_cli_exit_2(self):
        with tempfile.TemporaryDirectory() as d:
            rec=os.path.join(d,'record'); record(rec,[])
            raw=base64.urlsafe_b64encode(json.dumps({'objective_id':'obj','states':[]}).encode()).decode()
            script=os.path.join(os.path.dirname(os.path.dirname(__file__)),'usage_ledger.py')
            env=dict(os.environ, HOME=d, XDG_STATE_HOME=d)
            proc=subprocess.run([sys.executable,'-I',script,'control','--record',rec,'--cursor',raw],env=env,capture_output=True,text=True)
            self.assertEqual(proc.returncode,2); self.assertIn('invalid cursor',proc.stderr)

    def test_undated_calls_never_cross_successor_boundary(self):
        # An undated successor call belongs to neither query window nor node.
        with tempfile.TemporaryDirectory() as d:
            rec=os.path.join(d,'record')
            record(rec,[session('root'),session('successor',start='2026-01-02T12:00:00Z')])
            call=dict(provider='codex',thread='successor',ts=None,model='gpt-6.1-sol',service_tier='default',quota_multiplier=1,usd=None,pools=['codex'])
            with patch.object(L,'transcript_available',return_value=True),patch.object(L,'build_codex',return_value=([call],[],[],0)),patch.object(L,'build_claude',return_value=([],[],0)):
                for start,end in [(T0,T1),(T1,T2)]:
                    tree,_,_=L.cost_tree(rec,since_arg=start,until_arg=end,role_ledger=os.path.join(d,'missing'))
                    self.assertEqual(tree['window_totals']['codex']['calls'],0)
                    self.assertEqual(tree['totals']['codex']['calls'],0)
                    self.assertEqual(tree['undated']['counts']['codex'],1)
                    self.assertEqual(tree['status'],'partial')

    def test_changed_level_is_new_trigger_and_duplicate_mark_is_single(self):
        self.sb['limits']={}; self.sb['cost_limits']=[{'pool':'codex','set_by':'supervisor','marks_usd':[1], 'max_usd':1}]
        first=self.run_control(); self.assertEqual([e['level'] for e in first['events']],[1]); self.consume(first)
        self.sb['cost_limits'][0]['marks_usd']=[2]; self.sb['cost_limits'][0]['max_usd']=2
        self.assertEqual([e['level'] for e in self.run_control()['events']],[2])
