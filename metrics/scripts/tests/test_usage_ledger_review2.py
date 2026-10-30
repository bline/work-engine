import json
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L
from test_usage_ledger_review1 import record, rollout, session, T0, T1, T2

class DiscoveredCoverage(unittest.TestCase):
    def test_missing_discovered_role_children(self):
        with tempfile.TemporaryDirectory() as d:
            root=os.path.join(d,'root.jsonl'); rec=os.path.join(d,'record'); ledger=os.path.join(d,'ledger')
            rollout(root,'root'); record(rec,[session('root')])
            with patch.object(L,'CODEX_GLOBS',[root]), patch.object(L,'CLAUDE_GLOB',os.path.join(d,'none')), patch.object(L,'HOME',d), patch.object(L,'thread_index',return_value={}):
                for provider in ('codex','claude'):
                    with self.subTest(provider=provider):
                        with open(ledger,'w') as f:
                            f.write(json.dumps(dict(event='launch',session_id='missing-child',parent_session_id='root',host=provider))+'\n')
                        tree,_,_=L.cost_tree(rec,role_ledger=ledger)
                        self.assertEqual(tree['status'],'partial')
                        self.assertEqual({n['node_id'] for n in tree['nodes']},{'root','missing-child'})
                        missing=[i for i in tree['issues'] if i['kind']=='transcript_missing']
                        self.assertEqual([(i['session_id'],i['provider']) for i in missing],[('missing-child',provider)])
                        self.assertEqual(tree['totals']['codex']['calls'],0)

    def test_subagent_uses_its_own_transcript(self):
        with tempfile.TemporaryDirectory() as d:
            project=os.path.join(d,'.claude','projects','p'); os.makedirs(os.path.join(project,'s','subagents'))
            main=os.path.join(project,'s.jsonl'); agent=os.path.join(project,'s','subagents','agent-x.jsonl')
            with open(main,'w') as f: f.write('{}\n')
            with open(agent,'w') as f: f.write('{}\n')
            with patch.object(L,'HOME',d):
                self.assertTrue(L.transcript_available('claude','s',{},'s/agent:agent-x'))
                os.remove(agent)
                self.assertFalse(L.transcript_available('claude','s',{},'s/agent:agent-x'))
                self.assertTrue(L.transcript_available('claude','s',{}))

class OldMetadata(unittest.TestCase):
    def test_unrelated_and_ancestor_bad_metadata(self):
        with tempfile.TemporaryDirectory() as d:
            root=os.path.join(d,'root.jsonl'); child=os.path.join(d,'child.jsonl'); old=os.path.join(d,'old.jsonl'); rec=os.path.join(d,'record')
            rollout(root,'root',calls=[('root-call','2026-01-02T12:00:00Z')]); record(rec,[session('root')])
            old_time=L.ts_of('2025-12-31T00:00:00Z').timestamp()
            with patch.object(L,'CODEX_GLOBS',[os.path.join(d,'*.jsonl')]),patch.object(L,'CLAUDE_GLOB',os.path.join(d,'none')),patch.object(L,'thread_index',return_value={}):
                for ancestor in (False,True):
                    with self.subTest(ancestor=ancestor):
                        if ancestor: rollout(child,'child','old',calls=[('child-call','2026-01-02T12:00:00Z')])
                        elif os.path.exists(child): os.remove(child)
                        bad={'type':'session_meta','timestamp':T0,'payload':{'id':'old','parent_thread_id':{'bad':'shape'},'agent_path':['bad']}}
                        with open(old,'w') as f: f.write(json.dumps(bad,separators=(',',':'))+'\n')
                        os.utime(old,(old_time,old_time))
                        tree,_,_=L.cost_tree(rec,since_arg=T1,until_arg=T2,role_ledger=os.path.join(d,'absent'))
                        self.assertEqual(tree['status'],'partial')
                        fields={i.get('field') for i in tree['issues'] if i['kind']=='invalid_session_metadata'}
                        self.assertEqual(fields,{'parent_thread_id','agent_path'})
                        self.assertEqual(tree['totals']['codex']['calls'],1)
                        self.assertEqual(tree['window_totals']['codex']['calls'],2 if ancestor else 1)
                        self.assertEqual(tree['unattributed']['totals']['codex']['calls'],1 if ancestor else 0)

    def test_bad_old_id_is_reported(self):
        with tempfile.TemporaryDirectory() as d:
            root=os.path.join(d,'root.jsonl'); old=os.path.join(d,'old.jsonl'); rec=os.path.join(d,'record')
            rollout(root,'root',calls=[('good','2026-01-02T12:00:00Z')]); record(rec,[session('root')])
            with open(old,'w') as f: f.write(json.dumps({'type':'session_meta','payload':{'id':['bad']}})+'\n')
            stamp=L.ts_of('2025-12-31T00:00:00Z').timestamp(); os.utime(old,(stamp,stamp))
            with patch.object(L,'CODEX_GLOBS',[os.path.join(d,'*.jsonl')]),patch.object(L,'CLAUDE_GLOB',os.path.join(d,'none')),patch.object(L,'thread_index',return_value={}):
                tree,_,_=L.cost_tree(rec,since_arg=T1,until_arg=T2,role_ledger=os.path.join(d,'absent'))
            self.assertEqual(tree['totals']['codex']['calls'],1)
            self.assertTrue(any(i['kind']=='invalid_session_metadata' and i.get('field')=='id' for i in tree['issues']))
