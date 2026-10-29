import contextlib, fcntl, importlib.util, io, json, os, shlex, shutil, signal, subprocess, sys, tempfile, threading, time, unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
SCRIPTS=Path(__file__).resolve().parents[1]/'scripts'; sys.path.insert(0,str(SCRIPTS))
import auto_continue as a

SID='00000000-0000-4000-8000-000000000001'; SOURCE='00000000-0000-4000-8000-000000000002'; COMPACT='00000000-0000-4000-8000-000000000003'; WAKE='00000000-0000-4000-8000-000000000004'
def record_compact(host, item_id='compact-item'):
    records=[{'type':'event_msg','payload':{'type':'task_started','turn_id':COMPACT}},
             {'type':'compacted','payload':{}},
             {'type':'turn_context','payload':{'turn_id':COMPACT}},
             {'type':'event_msg','payload':{'type':'item_completed','turn_id':COMPACT,'thread_id':SID,'item':{'type':'ContextCompaction','id':item_id}}},
             {'type':'event_msg','payload':{'type':'task_complete','turn_id':COMPACT}}]
    with host.rollout.open('a') as f:
        for row in records: f.write(json.dumps(row)+'\n')

class Conn:
    def __init__(self,host): self.h=host
    def __enter__(self): return self
    def __exit__(self,*args): pass
    def call(self,method,params):
        h=self.h; h.calls.append((method,params))
        if method=='thread/read': return {'thread':{'id':h.read_sid}}
        if method=='thread/turns/list': return {'data':h.turns,'nextCursor':None}
        if method=='thread/compact/start':
            if h.compact_loss: raise a.codex_app.AppServerError('lost compact response')
            h.turns.append({'id':COMPACT,'status':'completed','items':[{'id':'compact-item','type':'contextCompaction'}]}); record_compact(h); return {}
        if method=='thread/resume': return {'thread':{'id':h.resume_sid}}
        if method=='turn/start':
            item={'id':'message','type':'userMessage','clientId':params['clientUserMessageId'],'content':params['input']}
            h.turns.append({'id':WAKE,'status':'inProgress','items':[item]})
            if h.wake_loss: raise a.codex_app.AppServerError('lost wake response')
            return {'turn':{'id':WAKE}}
        raise AssertionError(method)
class Host:
    def __init__(self):
        self.turns=[{'id':SOURCE,'status':'inProgress','items':[]}]; self.calls=[]; self.read_sid=SID; self.resume_sid=SID; self.compact_loss=False; self.wake_loss=False
    def _conn(self): return Conn(self)
class AutoTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.dir=Path(self.tmp.name); self.env=patch.dict(os.environ,{'XDG_STATE_HOME':str(self.dir/'state')}); self.env.start()
        self.rollout=self.dir/'rollout.jsonl'; self.cont=self.dir/'continuation'; self.wind=self.dir/'SKILL.md'; self.cont.write_text('continuation'); self.wind.write_text('wind')
        self.rollout.write_text(json.dumps({'type':'session_meta','payload':{'id':SID,'cli_version':'test'}})+'\n'+json.dumps({'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'m','reasoning_effort':'high','service_tier':'default'}}})+'\n')
        binary=self.dir/'binary'; binary.write_text('#!/usr/bin/env python3\nimport json, pathlib\nprint(pathlib.Path(__file__).with_name("report.json").read_text())\n'); binary.chmod(0o755)
        self.h=Host(); self.h.rollout=self.rollout; self.store=a.Store(str(self.dir/'op')); self.args=SimpleNamespace(operation=str(self.store.root),sid=SID,rollout=str(self.rollout),source_turn=SOURCE,continuation=str(self.cont),wind_walker=str(self.wind),context_binary=str(binary),context_sha256=a.digest(binary),author='actor',collector='fixture supervisor with stdout collection')
    def tearDown(self): self.env.stop(); self.tmp.cleanup()
    def prep(self):
        op=a.prepare(self.args,self.h)
        with self.store.lock():
            op=self.store.read(); op['job_reference']='exec-session-123'; self.store.save(op,'job_bound',job_reference=op['job_reference'])
        return op
    def finish(self):
        self.h.turns[0]['status']='completed'; return a.run(self.store,self.h,1,.001)
    def test_happy_and_duplicate_fence(self):
        op=self.prep(); other=SimpleNamespace(**vars(self.args)); other.operation=str(self.dir/'other')
        with self.assertRaisesRegex(a.Refusal,'another operation'): a.prepare(other,self.h)
        self.assertEqual(self.finish()['turn_id'],WAKE); state=self.store.read()
        self.assertEqual([x['event'] for x in state['journal']],['prepared','job_bound','controller_started','compact_intent','compact_acknowledged','compaction_collected','wakeup_intent','wakeup_acknowledged','wakeup_reconciled'])
        self.assertEqual(len([x for x in self.h.calls if x[0]=='turn/start']),1)
        self.assertEqual(a.run(self.store,self.h,1,.001)['turn_id'],WAKE)
        self.assertIn('not human authorization',state['message']); self.assertIn('pinned Wind Walker',state['message']); self.assertEqual(op['operation_id'],state['operation_id'])
    def test_prepare_cli_requires_and_verifies_binary_pin(self):
        args=['prepare','--operation',str(self.store.root),'--sid',SID,'--rollout',str(self.rollout),
              '--source-turn',SOURCE,'--continuation',str(self.cont),'--wind-walker',str(self.wind),
              '--context-binary',str(self.dir/'binary'),'--context-sha256','0'*64,'--author','actor','--collector',self.args.collector]
        rc,out=self.cli(args); self.assertEqual((rc,out['status']),(1,'failure')); self.assertIn('digest mismatch',out['detail']); self.assertFalse(self.store.path.exists())
        args[args.index('--context-sha256')+1]=self.args.context_sha256
        rc,out=self.cli(args); self.assertEqual(rc,0); self.assertEqual(out['context_binary'],str(self.dir/'binary')); self.assertEqual(out['context_sha256'],self.args.context_sha256)
    def test_wakeup_projects_actual_commands_pins_and_effect_semantics(self):
        op=self.prep(); self.finish(); op=self.store.read(); text=op['message']
        command=text.split('Replace the quoted placeholders with actual values, choosing continue, pause or cancel from current direction: ',1)[1].split('. Only ',1)[0]
        args=shlex.split(command); self.assertEqual(args[:4],['python3','-B',str(SCRIPTS/'auto_continue.py'),'admit']); self.assertEqual(args[args.index('--direction')+1],'<CURRENT_DIRECTION>')
        for key,value in [('--operation',str(self.store.root)),('--context-binary',op['context_binary']),('--context-sha256',op['context_sha256']),('--sid',SID),('--rollout',str(self.rollout))]:
            self.assertEqual(args[args.index(key)+1],value)
        self.assertIn('Only an admit command returning status admitted for this invocation',text)
        self.assertIn('all other results leave it unperformed',text); self.assertIn(self.args.collector,text); self.assertIn('Already admitted or completed means do not replay',text)
        self.assertIn('leave both unchanged while this operation remains eligible for admission',text); self.assertIn('including a retryable refusal or pause',text); self.assertIn('write working/recovery notes elsewhere',text)
        self.assertIn('entry_not_reconciled',text); self.assertIn('collect or wait for the existing job first',text); self.assertIn('only after it exits',text); self.assertIn('without resending compact or wakeup',text)
        self.assertIn(shlex.quote(str(SCRIPTS/'auto_continue.py')),text)
    def cli(self,argv):
        out=io.StringIO()
        with patch.object(a.role_host,'Codex',return_value=self.h), contextlib.redirect_stdout(out): rc=a.main(argv)
        return rc,json.loads(out.getvalue())
    def admit_cli(self):
        return ['admit','--operation',str(self.store.root),'--sid',SID,'--rollout',str(self.rollout),'--turn',WAKE,
                '--context-binary',str(self.dir/'binary'),'--context-sha256',self.args.context_sha256,
                '--direction','continue','--reason','direction reconciled','--collection','owned job collected']
    def test_cli_distinguishes_entry_pause_terminal_and_collector(self):
        self.prep(); rc,out=self.cli(self.admit_cli()); self.assertEqual((rc,out['status'],out['collector']),(1,'entry_not_reconciled',self.args.collector))
        a.change_control(self.store,'pause'); rc,out=self.cli(self.admit_cli()); self.assertEqual((rc,out['status']),(1,'paused'))
        a.change_control(self.store,'run'); self.finish()
        args=self.admission_args(); boundary=self.store.read()['replacement_line']
        with self.rollout.open('a') as f: f.write(json.dumps({'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'m','reasoning_effort':'high','service_tier':'default'}}})+'\n')
        (self.dir/'report.json').write_text(json.dumps({'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':10,'line':boundary+3},'context_window_tokens':1000}))
        rc,out=self.cli(self.admit_cli()); self.assertEqual((rc,out['status']),(0,'admitted'))
        self.cont.write_text('legitimate working continuation changed after admission')
        rc,out=self.cli(['run','--operation',str(self.store.root)]); self.assertEqual((rc,out['status']),(0,'admitted'))
        rc,out=self.cli(self.admit_cli()); self.assertEqual((rc,out['status'],out['collector']),(1,'already_admitted',self.args.collector))
        rc,out=self.cli(['complete','--operation',str(self.store.root),'--sid',SID,'--turn',WAKE,'--outcome','done']); self.assertEqual((rc,out['status']),(0,'completed'))
        rc,out=self.cli(['run','--operation',str(self.store.root)]); self.assertEqual((rc,out['status']),(0,'completed'))
        rc,out=self.cli(self.admit_cli()); self.assertEqual((rc,out['status'],out['collector']),(1,'completed',self.args.collector))
    def test_uncertain_wakeup_deadline_has_reconciliation_status(self):
        self.prep(); self.h.wake_loss=True
        with self.assertRaises(a.codex_app.AppServerError): self.finish()
        self.h.turns.pop(); rc,out=self.cli(['run','--operation',str(self.store.root),'--timeout','0.02','--interval','0.001'])
        self.assertEqual((rc,out['status'],out['phase'],out['collector']),(1,'entry_not_reconciled','wakeup_intent',self.args.collector))
        self.assertEqual(sum(c[0]=='turn/start' for c in self.h.calls),1)
    def test_cli_superseded_telemetry_settings_and_cancel_classes(self):
        self.prep(); self.finish(); args=self.admit_cli()
        rc,out=self.cli(args); self.assertEqual((rc,out['status'],out['collector']),(1,'telemetry_unestablished',self.args.collector))
        boundary=self.store.read()['replacement_line']; (self.dir/'report.json').write_text(json.dumps({'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':10,'line':boundary+2},'context_window_tokens':1000}))
        rc,out=self.cli(args); self.assertEqual((rc,out['status']),(1,'settings_unestablished'))
        a.change_control(self.store,'cancel'); rc,out=self.cli(args); self.assertEqual((rc,out['status']),(1,'cancelled'))
        with self.store.lock(): op=self.store.read(); op['control']='run'; self.store.save(op,'test_fixture_reset')
        self.cont.write_text('changed'); rc,out=self.cli(args); self.assertEqual((rc,out['status']),(1,'superseded'))
    def test_bad_native_shape_journals_failure(self):
        self.prep(); self.h.turns[0]['status']='completed'; self.h.turns.append({'id':COMPACT,'status':'inProgress','items':None})
        with self.store.lock(): op=self.store.read(); op['phase']='compact_intent'; self.store.save(op,'test_intent')
        rc,out=self.cli(['run','--operation',str(self.store.root),'--timeout','1'])
        self.assertEqual((rc,out['status'],out['collector']),(1,'failure',self.args.collector))
        row=self.store.read()['journal'][-1]; self.assertEqual((row['event'],row['phase'],row['collector']),('controller_failure','compact_intent',self.args.collector))
    def test_reservation_namespace_and_content_free_baseline(self):
        self.prep(); self.h.turns[0]['items']=[{'id':'msg','type':'agentMessage','text':'SECRET-COMMENTARY'}]; self.finish(); op=self.store.read()
        self.assertEqual(op['reservation_path'],str(self.dir/'state/work-engine/auto-continuation'/f'{SID}.json'))
        self.assertEqual(op['reservation_namespace'],str(Path(op['reservation_path']).parent))
        self.assertNotIn('SECRET-COMMENTARY',json.dumps(op['compact_baseline'])); self.assertEqual(op['compact_baseline'][0]['item_ids'],['msg'])
        other=SimpleNamespace(**vars(self.args)); other.operation=str(self.dir/'another')
        with self.assertRaisesRegex(a.Refusal,op['reservation_path']): a.prepare(other,self.h)
    def test_delayed_partial_boundary_waits_without_reentry(self):
        self.prep(); self.h.turns[0]['status']='completed'; h=self.h
        class Delayed(Conn):
            def call(self,method,params):
                if method=='thread/compact/start':
                    h.calls.append((method,params)); h.turns.append({'id':COMPACT,'status':'completed','items':[{'id':'compact-item','type':'contextCompaction'}]})
                    with h.rollout.open('ab') as f: f.write(b'{"type":')
                    return {}
                if method=='thread/turns/list' and len(h.turns)>1 and h.turns[-1]['id']==COMPACT:
                    h.polls=getattr(h,'polls',0)+1
                    if h.polls==3:
                        with h.rollout.open('rb+') as f: f.seek(-8,2); f.truncate()
                        record_compact(h)
                return super().call(method,params)
        h._conn=lambda:Delayed(h)
        self.assertEqual(a.run(self.store,h,1,.001)['turn_id'],WAKE)
        self.assertGreaterEqual(h.polls,3); self.assertEqual(sum(c[0]=='thread/compact/start' for c in h.calls),1)
    def test_complete_uses_bound_namespace_from_real_cli(self):
        self.prep(); op=self.store.read(); reservation=Path(op['reservation_path'])
        with self.store.lock(): op=self.store.read(); op['phase']='admitted'; op['wakeup_turn']=WAKE; self.store.save(op,'test_admitted')
        other=self.dir/'unrelated-state'; env=os.environ.copy(); env['XDG_STATE_HOME']=str(other); env.pop('PYTHONDONTWRITEBYTECODE',None)
        argv=[sys.executable,str(SCRIPTS/'auto_continue.py'),'complete','--operation',str(self.store.root),'--sid',SID,'--turn',WAKE,'--outcome','actual']
        proc=subprocess.run(argv,env=env,capture_output=True,text=True); out=json.loads(proc.stdout)
        self.assertEqual(proc.returncode,0,proc.stderr); self.assertEqual(out['reservation_release']['status'],'released')
        self.assertEqual(out['reservation_release']['path'],str(reservation)); self.assertFalse(reservation.exists()); self.assertFalse(other.exists())
        proc=subprocess.run(argv,env=env,capture_output=True,text=True); self.assertEqual(json.loads(proc.stdout)['reservation_release']['status'],'absent'); self.assertFalse(other.exists())
    def test_complete_refuses_other_reservation_and_retries_only_cleanup(self):
        self.prep(); op=self.store.read(); reservation=Path(op['reservation_path']); expected=json.loads(reservation.read_text())
        reservation.write_text(json.dumps({'operation_id':'other','operation':'different'}))
        with self.store.lock(): op=self.store.read(); op['phase']='admitted'; op['wakeup_turn']=WAKE; self.store.save(op,'test_admitted')
        argv=[sys.executable,str(SCRIPTS/'auto_continue.py'),'complete','--operation',str(self.store.root),'--sid',SID,'--turn',WAKE,'--outcome','effect once']
        proc=subprocess.run(argv,capture_output=True,text=True); out=json.loads(proc.stdout)
        self.assertEqual((proc.returncode,out['status'],out['reservation_release']['status']),(0,'completed','mismatch')); self.assertTrue(reservation.exists())
        reservation.write_text(json.dumps(expected)); proc=subprocess.run(argv,capture_output=True,text=True)
        self.assertEqual(json.loads(proc.stdout)['reservation_release']['status'],'released'); self.assertFalse(reservation.exists())
        self.assertEqual(sum(j['event']=='completed' for j in self.store.read()['journal']),1)
    def test_actual_cli_help_and_prepare_leave_no_input_cache(self):
        copied=self.dir/'copied scripts'; copied.mkdir()
        for name in ('auto_continue.py','role_host.py','codex_app.py','child_context.py'): shutil.copy2(SCRIPTS/name,copied/name)
        env=os.environ.copy(); env.pop('PYTHONDONTWRITEBYTECODE',None); env.pop('PYTHONPYCACHEPREFIX',None); env.pop('PYTHONPATH',None)
        script=str(copied/'auto_continue.py'); p=subprocess.run([sys.executable,script,'--help'],env=env,capture_output=True,text=True); self.assertEqual(p.returncode,0,p.stderr)
        argv=[sys.executable,script,'prepare','--operation',str(self.store.root),'--sid',SID,'--rollout',str(self.rollout),'--source-turn',SOURCE,'--continuation',str(self.cont),'--wind-walker',str(self.wind),'--context-binary',str(self.dir/'binary'),'--context-sha256','0'*64,'--author','actor','--collector','collector']
        p=subprocess.run(argv,env=env,capture_output=True,text=True); self.assertEqual(p.returncode,1); self.assertIn('digest mismatch',json.loads(p.stdout)['detail'])
        self.assertFalse((copied/'__pycache__').exists())
    def test_busy_actual_cli_does_not_journal_controller_failure(self):
        self.prep(); lock=self.store.root/'controller.lock'
        with lock.open('a') as f:
            fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB); before=self.store.read()['journal'][:]
            p=subprocess.run([sys.executable,str(SCRIPTS/'auto_continue.py'),'run','--operation',str(self.store.root),'--timeout','1'],capture_output=True,text=True)
        out=json.loads(p.stdout); self.assertEqual((p.returncode,out['status'],out['collector']),(1,'already_controlled',self.args.collector))
        self.assertEqual(self.store.read()['journal'],before)
    def test_binding_job_keeps_pinned_continuation_immutable(self):
        op=a.prepare(self.args,self.h); before=a.digest(self.cont)
        with patch.object(sys,'argv',['auto_continue','bind-job','--operation',str(self.store.root),'--job-reference','exec-42']):
            self.assertEqual(a.main(),0)
        self.assertEqual(a.digest(self.cont),before); self.assertEqual(self.store.read()['job_reference'],'exec-42')
        self.assertEqual(self.finish()['turn_id'],WAKE)
        self.cont.write_text('changed'); self.assertNotEqual(a.digest(self.cont),op['continuation_sha256'])
        with self.assertRaisesRegex(a.Refusal,'continuation changed'): a.run(self.store,self.h,1,.001)
    def test_pending_empty_native_turn_becomes_compaction(self):
        self.prep(); self.h.turns[0]['status']='completed'; h=self.h
        class AsyncConn(Conn):
            def call(self,method,params):
                if method=='thread/compact/start':
                    h.calls.append((method,params)); h.turns.append({'id':COMPACT,'status':'inProgress','items':[]}); return {}
                if method=='thread/turns/list' and len(h.turns)>1 and h.turns[-1]['id']==COMPACT:
                    h.polls=getattr(h,'polls',0)+1
                    if h.polls==3:
                        h.turns[-1].update(status='completed',items=[{'id':'compact-item','type':'contextCompaction'}]); record_compact(h)
                return super().call(method,params)
        h._conn=lambda:AsyncConn(h)
        self.assertEqual(a.run(self.store,h,1,.001)['turn_id'],WAKE)
        self.assertGreaterEqual(h.polls,3); self.assertEqual(sum(c[0]=='thread/compact/start' for c in h.calls),1)
    def test_pending_empty_turn_with_user_input_supersedes(self):
        self.prep(); self.h.turns[0]['status']='completed'; h=self.h
        class InputConn(Conn):
            def call(self,method,params):
                if method=='thread/compact/start':
                    h.calls.append((method,params)); h.turns.append({'id':COMPACT,'status':'inProgress','items':[]}); return {}
                if method=='thread/turns/list' and len(h.turns)>1:
                    h.polls=getattr(h,'polls',0)+1
                    if h.polls==2: h.turns[-1]['items']=[{'id':'human','type':'userMessage','content':[{'type':'text','text':'stop'}]}]
                return super().call(method,params)
        h._conn=lambda:InputConn(h)
        with self.assertRaisesRegex(a.Refusal,'intervening native input'): a.run(self.store,h,1,.001)
        self.assertEqual(sum(c[0]=='thread/compact/start' for c in h.calls),1)
        self.assertFalse(any(c[0]=='turn/start' for c in h.calls))
    def test_lost_compact_reconciles_without_replay(self):
        self.prep(); self.h.turns[0]['status']='completed'; self.h.compact_loss=True
        with self.assertRaises(a.codex_app.AppServerError): a.run(self.store,self.h,.05,.001)
        self.assertEqual(self.store.read()['phase'],'compact_intent')
        self.h.turns.append({'id':COMPACT,'status':'completed','items':[{'id':'i','type':'contextCompaction'}]}); record_compact(self.h,'i')
        self.h.compact_loss=False; self.assertEqual(a.run(self.store,self.h,1,.001)['turn_id'],WAKE)
        self.assertEqual(sum(c[0]=='thread/compact/start' for c in self.h.calls),1)
    def test_lost_wakeup_reconciles_without_replay(self):
        self.prep(); self.h.wake_loss=True
        with self.assertRaises(a.codex_app.AppServerError): self.finish()
        self.assertEqual(self.store.read()['phase'],'wakeup_intent')
        self.h.wake_loss=False; self.assertEqual(a.run(self.store,self.h,1,.001)['turn_id'],WAKE)
        self.assertEqual(sum(c[0]=='turn/start' for c in self.h.calls),1)
    def test_source_interruption_supersession_and_mismatch(self):
        self.prep(); self.h.turns[0]['status']='interrupted'
        with self.assertRaisesRegex(a.Refusal,'interrupted'): a.run(self.store,self.h,1,.001)
        self.h.turns[0]['status']='completed'; self.h.turns.append({'id':'other','status':'completed','items':[]})
        with self.assertRaisesRegex(a.Refusal,'superseded'): a.run(self.store,self.h,1,.001)
        self.h.turns.pop(); self.h.read_sid='wrong'
        with self.assertRaisesRegex(a.Refusal,'identity mismatch'): a.run(self.store,self.h,1,.001)
    def test_wrong_resume_and_changed_continuation(self):
        self.prep(); self.h.resume_sid='wrong'
        with self.assertRaisesRegex(a.Refusal,'resume identity'): self.finish()
        self.cont.write_text('changed')
        with self.assertRaisesRegex(a.Refusal,'continuation changed'): a.run(self.store,self.h,1,.001)
    def test_pause_and_cancel_before_ready(self):
        self.prep(); self.h.turns[0]['status']='completed'; a.change_control(self.store,'pause')
        self.assertEqual(a.run(self.store,self.h,1,.001)['status'],'pause'); self.assertFalse(any(c[0]=='thread/compact/start' for c in self.h.calls))
        a.change_control(self.store,'run'); a.change_control(self.store,'cancel'); self.assertEqual(a.run(self.store,self.h,1,.001)['status'],'cancel')
        with self.assertRaises(a.Refusal): a.change_control(self.store,'run')
    def admission_args(self):
        binary=self.dir/'binary'; binary.write_text('#!/usr/bin/env python3\nimport json, pathlib\nprint(pathlib.Path(__file__).with_name("report.json").read_text())\n'); binary.chmod(0o755)
        return SimpleNamespace(sid=SID,rollout=str(self.rollout),turn=WAKE,direction='continue',reason='current direction remains in force',collection='own exec job retained',context_binary=str(binary),context_sha256=a.digest(binary),settings_resolution='matched',settings_evidence=None,outcome='done')
    def test_post_boundary_settings_before_collector_tail_are_valid(self):
        self.prep(); self.h.turns[0]['status']='completed'; h=self.h
        class SettingsConn(Conn):
            def call(self,method,params):
                if method=='thread/compact/start':
                    h.calls.append((method,params)); h.turns.append({'id':COMPACT,'status':'completed','items':[{'id':'compact-item','type':'contextCompaction'}]})
                    records=[{'type':'event_msg','payload':{'type':'task_started','turn_id':COMPACT}},
                             {'type':'compacted','payload':{}}, {'type':'turn_context','payload':{'turn_id':COMPACT}},
                             {'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'m','reasoning_effort':'high','service_tier':'default'}}},
                             {'type':'event_msg','payload':{'type':'item_completed','turn_id':COMPACT,'thread_id':SID,'item':{'type':'ContextCompaction','id':'compact-item'}}},
                             {'type':'event_msg','payload':{'type':'task_complete','turn_id':COMPACT}}]
                    with h.rollout.open('a') as f:
                        for row in records: f.write(json.dumps(row)+'\n')
                    return {}
                return super().call(method,params)
        h._conn=lambda:SettingsConn(h); self.assertEqual(a.run(self.store,h,1,.001)['turn_id'],WAKE)
        op=self.store.read(); boundary=op['replacement_boundary']; lines=self.rollout.read_bytes().splitlines(keepends=True)
        self.assertEqual(json.loads(lines[boundary['line']-1])['type'],'compacted')
        self.assertEqual(boundary['event_sha256'],a.hashlib.sha256(lines[boundary['line']-1]).hexdigest())
        self.assertLess(boundary['line'],a.role_host.codex_settings(SID,self.rollout)['source']['line'])
        self.assertLess(a.role_host.codex_settings(SID,self.rollout)['source']['line'],len(lines))
        args=self.admission_args(); report={'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':10,'line':len(lines)+1},'context_window_tokens':1000}
        (self.dir/'report.json').write_text(json.dumps(report)); self.assertEqual(a.admit(self.store,h,args)['status'],'admitted')
    def test_only_pre_boundary_settings_are_rejected(self):
        self.prep(); self.finish(); op=self.store.read(); args=self.admission_args()
        report={'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':10,'line':op['replacement_line']+2},'context_window_tokens':1000}
        (self.dir/'report.json').write_text(json.dumps(report))
        with self.assertRaisesRegex(a.Refusal,'applied settings unestablished'): a.admit(self.store,self.h,args)
    def test_missing_ambiguous_and_truncated_compaction_boundary_refused(self):
        self.prep(); baseline=self.rollout.stat().st_size; record_compact(self.h)
        original=self.rollout.read_bytes(); op={'rollout':str(self.rollout),'compact_rollout_bytes':baseline,'sid':SID}; compact={'id':COMPACT,'items':[{'id':'compact-item','type':'contextCompaction'}]}
        self.assertEqual(a.replacement_boundary(op,compact)['turn_id'],COMPACT)
        rows=original.splitlines(keepends=True); marker=next(i for i,row in enumerate(rows) if json.loads(row)['type']=='compacted')
        self.rollout.write_bytes(b''.join(rows[:marker]+rows[marker+1:]))
        with self.assertRaisesRegex(a.Refusal,'order invalid'): a.replacement_boundary(op,compact)
        self.rollout.write_bytes(b''.join(rows[:marker+1]+[rows[marker]]+rows[marker+1:]))
        with self.assertRaisesRegex(a.Refusal,'ambiguous or contradictory'): a.replacement_boundary(op,compact)
        self.rollout.write_bytes(original[:-1])
        with self.assertRaises(a.PendingBoundary): a.replacement_boundary(op,compact)
        altered=original.replace(b'compact-item',b'other-item')
        self.rollout.write_bytes(altered)
        with self.assertRaisesRegex(a.Refusal,'contradicts native identity'): a.replacement_boundary(op,compact)
    def test_telemetry_repair_and_irreversible_claim(self):
        self.prep(); self.finish(); args=self.admission_args(); boundary=self.store.read()['replacement_line']
        with self.rollout.open('a') as f: f.write(json.dumps({'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'m','reasoning_effort':'high','service_tier':'default'}}})+'\n')
        report={'session_id':SID,'rollout':str(self.rollout),'latest_request':{},'context_window_tokens':None}
        (self.dir/'report.json').write_text(json.dumps(report))
        with self.assertRaisesRegex(a.Refusal,'telemetry'): a.admit(self.store,self.h,args)
        report.update(latest_request={'input_tokens':30,'line':boundary+3},latest_request_status='latest_valid_observation',context_window_tokens=1000,last_record_gap_line=boundary+1)
        (self.dir/'report.json').write_text(json.dumps(report))
        self.assertEqual(a.admit(self.store,self.h,args)['status'],'admitted')
        with self.assertRaises(a.Refusal) as denied: a.admit(self.store,self.h,args)
        self.assertEqual(denied.exception.status,'already_admitted')
        self.assertEqual(a.complete(self.store,args)['status'],'completed')
    def test_process_termination_records_failure(self):
        self.prep()
        code = """import sys
sys.path.insert(0,sys.argv[1]); sys.path.insert(0,sys.argv[2])
import auto_continue as a
from test_auto_continue import Host
class ReadyHost(Host):
 def _conn(self):
  open(sys.argv[4],'w').write('ready')
  return super()._conn()
a.role_host.Codex=lambda:ReadyHost()
raise SystemExit(a.main(['run','--operation',sys.argv[3],'--timeout','30','--interval','0.02']))
"""
        proc=subprocess.Popen([sys.executable,'-c',code,str(SCRIPTS),str(Path(__file__).parent),str(self.store.root),str(self.dir/'ready')],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            deadline=time.monotonic()+5
            while not (self.dir/'ready').exists() and time.monotonic()<deadline and proc.poll() is None: time.sleep(.01)
            self.assertTrue((self.dir/'ready').exists()); self.assertIsNone(proc.poll()); proc.send_signal(signal.SIGTERM)
            out,err=proc.communicate(timeout=5)
            self.assertEqual(proc.returncode,1,err); self.assertEqual(json.loads(out)['status'],'failure')
            self.assertEqual(self.store.read()['journal'][-1]['event'],'controller_failure')
            self.assertIn('signal',self.store.read()['journal'][-1]['detail'])
        finally:
            if proc.poll() is None: proc.kill(); proc.wait()
    def test_pause_alone_denies_otherwise_ready_recovery(self):
        self.prep(); self.finish(); args=self.admission_args(); boundary=self.store.read()['replacement_line']
        with self.rollout.open('a') as f: f.write(json.dumps({'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'m','reasoning_effort':'high','service_tier':'default'}}})+'\n')
        report={'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':10,'line':boundary+2},'context_window_tokens':1000}
        (self.dir/'report.json').write_text(json.dumps(report))
        before=(a.digest(self.cont),a.digest(self.wind))
        a.change_control(self.store,'pause')
        with self.assertRaisesRegex(a.Refusal,'paused'): a.admit(self.store,self.h,args)
        self.assertEqual((a.digest(self.cont),a.digest(self.wind)),before)
        self.assertEqual(self.store.read()['phase'],'wakeup_entered')
        a.change_control(self.store,'run')
        self.assertEqual(a.admit(self.store,self.h,args)['status'],'admitted')
    def test_listener_failure_reports_collector_and_journal(self):
        self.prep()
        class Unreachable:
            def _conn(self): raise a.codex_app.AppServerError('listener unavailable')
        output=io.StringIO()
        with patch.object(a.role_host,'Codex',return_value=Unreachable()), contextlib.redirect_stdout(output):
            rc=a.main(['run','--operation',str(self.store.root),'--timeout','1'])
        result=json.loads(output.getvalue()); self.assertEqual(rc,1); self.assertEqual(result['status'],'failure')
        self.assertEqual(result['collector'],self.args.collector); self.assertIn('listener unavailable',result['detail'])
        state=self.store.read(); self.assertEqual(state['phase'],'prepared'); self.assertEqual(state['journal'][-1]['event'],'controller_failure')
        self.assertFalse(any(call[0] in ('thread/compact/start','turn/start') for call in self.h.calls))
    def test_wrong_recovery_identity_refused(self):
        self.prep(); self.finish(); args=self.admission_args(); args.sid='wrong'
        with self.assertRaisesRegex(a.Refusal,'recovery SID'): a.admit(self.store,self.h,args)
        args.sid=SID; args.turn='wrong'
        with self.assertRaisesRegex(a.Refusal,'recovery turn'): a.admit(self.store,self.h,args)
    def test_missing_optional_client_id_and_duplicate_message(self):
        self.prep(); self.h.wake_loss=True
        with self.assertRaises(a.codex_app.AppServerError): self.finish()
        self.h.turns[-1]['items'][0]['clientId']=None
        self.assertEqual(a.run(self.store,self.h,1,.001)['turn_id'],WAKE)
        self.h.turns.append({'id':'duplicate','status':'completed','items':self.h.turns[-1]['items']})
        with self.assertRaisesRegex(a.Refusal,'duplicate native wakeups'): a.wake_match(self.h.turns,self.store.read())
    def test_two_recoveries_cannot_claim(self):
        self.prep(); self.finish(); args=self.admission_args(); boundary=self.store.read()['replacement_line']
        with self.rollout.open('a') as f: f.write(json.dumps({'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'m','reasoning_effort':'high','service_tier':'default'}}})+'\n')
        report={'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':3,'line':boundary+2},'context_window_tokens':1000}; results=[]
        def attempt():
            try: results.append(a.admit(self.store,self.h,args)['status'])
            except a.Refusal: results.append('refused')
        with patch.object(a.child_context,'run_json',return_value=report):
            threads=[threading.Thread(target=attempt) for _ in range(2)]
            for t in threads: t.start()
            for t in threads: t.join()
        self.assertCountEqual(results,['admitted','refused'])
    def test_post_compaction_turn_supersedes_before_wakeup(self):
        self.prep(); self.h.resume_sid='bad'
        with self.assertRaisesRegex(a.Refusal,'resume identity'): self.finish()
        self.h.resume_sid=SID; self.h.turns.append({'id':'intervening','status':'completed','items':[]})
        with self.assertRaisesRegex(a.Refusal,'intervening'): a.run(self.store,self.h,1,.001)
        self.assertFalse(any(call[0]=='turn/start' for call in self.h.calls))
    def test_settings_mismatch_requires_current_user_change(self):
        self.prep(); self.finish(); args=self.admission_args(); boundary=self.store.read()['replacement_line']
        with self.rollout.open('a') as f: f.write(json.dumps({'type':'event_msg','payload':{'type':'thread_settings_applied','thread_id':SID,'thread_settings':{'model':'new','reasoning_effort':'high','service_tier':'default'}}})+'\n')
        report={'session_id':SID,'rollout':str(self.rollout),'latest_request':{'input_tokens':3,'line':boundary+2},'context_window_tokens':1000}
        with patch.object(a.child_context,'run_json',return_value=report):
            with self.assertRaisesRegex(a.Refusal,'settings mismatch'): a.admit(self.store,self.h,args)
            args.settings_resolution='user-change'; args.settings_evidence='human selected new model in current direction'
            self.assertEqual(a.admit(self.store,self.h,args)['settings_mismatches']['model'],['m','new'])
    def test_concurrent_controller_lock(self):
        self.prep(); result=[]
        def runner():
            try: a.run(self.store,self.h,.15,.01)
            except Exception as e: result.append(str(e))
        t=threading.Thread(target=runner); t.start(); time.sleep(.03)
        with self.assertRaisesRegex(a.Refusal,'already running'): a.run(self.store,self.h,.1,.01)
        t.join(); self.assertIn('deadline expired',result[0])
if __name__=='__main__': unittest.main()
