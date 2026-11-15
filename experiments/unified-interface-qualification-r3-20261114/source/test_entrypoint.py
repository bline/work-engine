"""Contract expectations, real entrypoint and independent processes; no real Docker."""
import ctypes, json, os, pathlib, shutil, subprocess, sys, time, unittest, uuid
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import realization as z
SOURCE=pathlib.Path(__file__).resolve().parent
Q=pathlib.Path('/home/bline/cbm-work/unified-interface-sprint-20261114/Q-repair-r3/qualification')
# The test outer supervisor can collect a watchdog orphan after intentional failed parent collection.
ctypes.CDLL(None).prctl(36,1,0,0,0)
def seal(root):
    bindings={p.name:z.sha(p) for p in root.iterdir() if p.is_file() and p.name in set(z.REQUIRED_SOURCE)|{'closure.json'}}
    r=z.read(root/'closure.json')['realization']
    z.write(root/'READY.json',dict(version=1,status='finalized',bindings=bindings,pins={p:z.sha(p) for p in r['host_executables'].values()},preparation_evidence=[]))
def fixture(scenario=None,scale=.15):
    root=Q/('case-'+uuid.uuid4().hex);root.mkdir(parents=True)
    for name in z.REQUIRED_SOURCE:shutil.copyfile(SOURCE/name,root/name)
    for n in ['fake_docker.py','fake_fact.py']:(root/n).chmod(0o755)
    (root/'sentinel').write_text('isolated sentinel, never installed-host subject\n');(root/'sentinel').chmod(0o644)
    z.write(root/'scenario.json',scenario or {})
    r=dict(destination_aliases={},version=1,subject_root=str(root),receipt_relpath='READY.json',root_session_id='synthetic-root',controller_session_id='synthetic-controller-'+uuid.uuid4().hex,fixture_run_id='sentinel-'+uuid.uuid4().hex,owner_label_key='u2.private.owner',runtime_root=str(root/'runtime'),release_path=str(root.parent/(root.name+'-run-release.json')),consumed_fence_relpath='fixture-consumed.json',run_layout='runs/{run}/{mode}-{invocation}',evidence_layout='evidence/{run}/{mode}-{invocation}',entrypoints=dict(controller='controller.py',cleanup='cleanup.py',container_python='/sentinel/python',pid1='/sentinel/pid1',preflight='/sentinel/preflight',observer='/sentinel/observer',closure='/sentinel/closure.json'),host_executables=dict(python=str(pathlib.Path(sys.executable).resolve()),docker=str(root/'fake_docker.py'),fact_hook=str(root/'fake_fact.py')),modes=z.MODES,timing={k:v*scale for k,v in z.CEILINGS.items()},timing_scale=scale,qualification_class='isolated-controlled')
    entries=[dict(source=str(root/'sentinel'),destination='/sentinel/'+x,sha256=z.sha(root/'sentinel'),mode=0o644,bytes=(root/'sentinel').stat().st_size) for x in ['python','pid1','preflight','observer']]
    entries.append(dict(entries[0],destination='/bin/kill'))
    # scenario and fake executable identities are immutable runtime inputs as well as pins.
    for name in ['scenario.json','fake_docker.py','fake_fact.py']:
        p=root/name;entries.append(dict(source=str(p),destination='/sentinel/'+name,sha256=z.sha(p),mode=p.stat().st_mode&0o777,bytes=p.stat().st_size))
    source_ready=SOURCE/'READY.json'
    source_binding=dict(path=str(source_ready),sha256=z.sha(source_ready)) if source_ready.exists() else None
    c=dict(qualification_source=source_binding,image='sha256:'+'0'*64,environment={},mounts=[dict(source=e['source'],target=e['destination'],readonly=True) for e in entries],entries=entries,realization=r)
    z.write(root/'closure.json',c);seal(root);return root

def release(root,**changes):
    s=z.Subject(root);now=time.time();d=dict(issuer_session_id=s.r['root_session_id'],ready_path=str(s.ready),ready_sha256=z.sha(s.ready),closure_sha256=z.sha(s.closure),fixture_run_id=s.r['fixture_run_id'],permitted_mode='fixture',consumed_fence_path=str(s.fence),permitted_runs=1,execution_authorized=True,not_before=now-10,expires_at=now+1000,outer_collection_deadline=now+1000,collection_reserve=30*s.r['timing_scale'])
    d.update(changes);z.write(s.release,d)
def command(root,mode,inv=None,outer=None):
    return [str(pathlib.Path(sys.executable).resolve()),'-I',str(root/'controller.py'),mode,'--subject',str(root),'--invocation',inv or uuid.uuid4().hex,'--outer-deadline',str(outer or time.time()+1000)]
def execute(root,mode,inv=None,outer=None,timeout=60,fault=None):
    argv=command(root,mode,inv,outer)
    if fault:
        script="import sys,pathlib;sys.path.insert(0,"+repr(str(root))+");import realization as z,controller as c;original=z.write\n"+"def broken_write(p,v):\n if pathlib.Path(p).name=='custody.json':raise OSError('injected final file failure')\n return original(p,v)\n"+"class Broken:\n name='injected-stream'\n def write(self,*a):raise OSError('injected final stream failure')\n def flush(self):pass\n"+"z.write=broken_write;c.write=broken_write;sys.stdout=Broken()\n"+("sys.stderr=Broken()\n" if fault=='all' else '')+"raise SystemExit(c.main("+repr(str(root))+","+repr(mode)+","+repr(argv[argv.index('--invocation')+1])+","+argv[-1]+"))"
        argv=[argv[0],'-I','-c',script]
    with (root/'outer.stdout').open('w') as out,(root/'outer.stderr').open('w') as err:
        p=subprocess.Popen(argv,stdout=out,stderr=err)
        try:code=p.wait(timeout=timeout)
        except BaseException as error:
            z.write(root/'outer-collection-unresolved.json',dict(pid=p.pid,argv=argv,exit=p.returncode,collected=False,error=repr(error),owner_pid=os.getpid()))
            if p.poll() is None:p.kill()
            code=p.wait(timeout=5)
            z.write(root/'outer-collection-recovery.json',dict(pid=p.pid,exit=code,collected=True,scope='test controller forced collection; physical closure unestablished'))
            raise
    z.write(root/'outer-collection.json',dict(pid=p.pid,exit=code,argv=argv,ready_sha256=z.sha(root/'READY.json') if (root/'READY.json').exists() else None,clock='host CLOCK_MONOTONIC, real waits',scope='isolated controlled executable',source_ready=z.read(root/'closure.json').get('qualification_source')))
    rows=list(root.glob('evidence/*/*/custody.json'));v=z.read(rows[0]) if rows else None
    if v is None:
        for stream in ['outer.stdout','outer.stderr']:
            for line in (root/stream).read_text().splitlines():
                try:
                    candidate=json.loads(line)
                    if candidate.get('actor')=='controller' and 'custody' in candidate:v=candidate
                except ValueError:pass
    z.write(root/'outer-delivery.json',dict(delivered=v is not None,container=v.get('container') if v else None,captured=v,assumptions='bounded file/stdout/stderr capture available; no universal all-channels claim'))
    if v:
        assert v['attach_pid'] is None or v['attach_exit'] is not None, v
        assert v['watchdog_collected'] and (v['watchdog_pid'] is None or v['watchdog_exit'] is not None),v
    return code,v
class Entry(unittest.TestCase):
    def test_modes(self):
        for mode in z.MODES:
            with self.subTest(mode=mode):
                root=fixture(dict(missing=mode=='missing'))
                if mode=='fixture':release(root)
                code,v=execute(root,mode);self.assertEqual(code,0,(root,v,(root/'outer.stderr').read_text()));self.assertTrue(v['closed'])
                if mode=='fixture':self.assertTrue((root/'fixture-consumed.json').exists())
    def test_release_refusals(self):
        for change in [dict(issuer_session_id='wrong'),dict(ready_sha256='0'*64),dict(closure_sha256='0'*64),dict(fixture_run_id='wrong'),dict(permitted_mode='success'),dict(consumed_fence_path='/tmp/wrong'),dict(permitted_runs=2),dict(execution_authorized=False),dict(not_before=time.time()+500),dict(expires_at=time.time()-1),dict(outer_collection_deadline=time.time()+.1),dict(ready_path='/tmp/PREPARATION-READY.json')]:
            with self.subTest(change=change):
                root=fixture();release(root,**change);code,v=execute(root,'fixture');self.assertNotEqual(code,0);self.assertFalse((root/'rpc.log').exists());self.assertFalse((root/'fixture-consumed.json').exists())
        root=fixture();release(root);(root/'fixture-consumed.json').write_text('malformed');code,v=execute(root,'fixture');self.assertNotEqual(code,0);self.assertFalse((root/'rpc.log').exists())
        root=fixture();code,v=execute(root,'fixture');self.assertNotEqual(code,0);self.assertFalse((root/'rpc.log').exists())
    def test_binding_refusals(self):
        for mutation in ['missing-ready','old-ready','source-mismatch','unknown-version','aliased-root','mode-map','missing-pin','runtime-digest','dynamic-env','entrypoint','escaping-receipt']:
            with self.subTest(mutation=mutation):
                root=fixture();c=z.read(root/'closure.json')
                if mutation=='missing-ready':(root/'READY.json').unlink()
                elif mutation=='old-ready':(root/'READY.json').rename(root/'PREPARATION-READY.json')
                elif mutation=='source-mismatch':(root/'cleanup.py').write_text('changed')
                else:
                    if mutation=='unknown-version':c['realization']['version']=2
                    if mutation=='aliased-root':c['realization']['subject_root']=str(root)+'/../'+root.name
                    if mutation=='mode-map':c['realization']['modes']['watchdog']='success'
                    if mutation=='runtime-digest':c['entries'][0]['sha256']='0'*64
                    if mutation=='dynamic-env':c['environment']['TMPDIR']='/tmp'
                    if mutation=='entrypoint':c['realization']['entrypoints']['pid1']='/missing'
                    if mutation=='escaping-receipt':c['realization']['receipt_relpath']='../READY.json'
                    z.write(root/'closure.json',c);seal(root)
                    if mutation=='missing-pin':d=z.read(root/'READY.json');d['pins'].pop(str(root/'fake_docker.py'));z.write(root/'READY.json',d)
                code,v=execute(root,'fixture');self.assertNotEqual(code,0);self.assertFalse((root/'rpc.log').exists())
    def test_deadline_duplicate_and_consumption(self):
        root=fixture();code,v=execute(root,'success',outer=time.time()+.1);self.assertNotEqual(code,0);self.assertFalse((root/'rpc.log').exists())
        root=fixture();inv=uuid.uuid4().hex;self.assertEqual(execute(root,'success',inv)[0],0);n=len((root/'rpc.log').read_text().splitlines());self.assertNotEqual(execute(root,'success',inv)[0],0);self.assertEqual(len((root/'rpc.log').read_text().splitlines()),n)
        root=fixture(dict(ambiguous=True));release(root);self.assertNotEqual(execute(root,'fixture')[0],0);self.assertTrue((root/'fixture-consumed.json').exists());n=len((root/'rpc.log').read_text().splitlines());self.assertNotEqual(execute(root,'fixture')[0],0);self.assertEqual(len((root/'rpc.log').read_text().splitlines()),n)
    def test_ambiguity_wronglabels_unknown_and_io_failures(self):
        for scenario in [dict(ambiguous=True),dict(wronglabels=True),dict(controller_unknown=[1]),dict(controller_unknown=[2,3]),dict(controller_unknown=[4]),dict(watchdog_unknown=[1],controller_unknown=[3]),dict(lost_remove=True),dict(no_pre_effect=True),dict(fail_fact=['controller-terminal-fact.json']),dict(fail_fact=['cleanup-confirmed.json']),dict(fail_fact=['watchdog-ownership.json']),dict(fail_fact=['create-argv.json']),dict(hold_running=True,controller_unknown=[3],watchdog_unknown=[2]),dict(hold_running=True,controller_unknown=[3],watchdog_unknown=[3]),dict(hold_running=True,controller_unknown=[3],watch_start_delay=4)]:
            with self.subTest(scenario=scenario):
                root=fixture(scenario);mode='timeout' if scenario.get('no_pre_effect') else 'success';code,v=execute(root,mode);self.assertNotEqual(code,0,(root,v))
                if scenario.get('wronglabels'):
                    ops=[z.read(root/'daemon.json')];self.assertTrue(ops[0]['present']);self.assertNotIn('"kill"',(root/'rpc.log').read_text());self.assertNotIn('"rm"',(root/'rpc.log').read_text())
                for cp in root.glob('evidence/*/*/cleanup-confirmed.json'):
                    # Every confirmation remains independently validated, including peer provenance.
                    self.assertTrue(__import__('cleanup').confirmation(cp.parent,'a'*64,z.read(cp)['binding']))
    def test_scaled_delayed_and_pre_tw(self):
        for mode in ['success','watchdog']:
            root=fixture(dict(hold_running=True,controller_unknown=[3],watch_rpc_delay=.6,watch_local_delay=.5,pre_tw_delay=.5),scale=.1)
            code,v=execute(root,mode);self.assertEqual(v['watchdog_exit'],0,(root,v));self.assertTrue((pathlib.Path(v['binding']['evidence'])/'watchdog-closure.json').exists())
            own=z.read(pathlib.Path(v['binding']['evidence'])/'watchdog-ownership.json');self.assertGreater(own['Tw']-own['T0'],.2)
            self.assertAlmostEqual(own['cleanup_collection_deadline']-own['T0'],z.limits(z.Subject(root).t,mode)['watch'])
    def test_actual_removal_race_with_peer(self):
        root=fixture(dict(controller_rpc_delay=1.6,watch_rpc_delay=.05,attach_delay=5,controller_terminal_delay=5),scale=.2)
        code,v=execute(root,'success');self.assertNotEqual(code,0);self.assertTrue(v['closed']);self.assertEqual(v['watchdog_exit'],0)
        out=pathlib.Path(v['binding']['evidence']);cl=z.read(out/'controller-closure.json')
        self.assertTrue(any(f['actor']=='watchdog' for f in cl['removal_facts']),(root,cl))
        self.assertFalse((out/'controller-removal-fact.json').exists())
    def test_exhaustion(self):
        root=fixture(dict(pre_tw_delay=4));code,v=execute(root,'success');self.assertNotEqual(code,0);self.assertFalse((root/'rpc.log').exists())
    def test_failed_watchdog_collection_outer_custody(self):
        root=fixture(dict(hold_running=True,controller_unknown=[3],watch_rpc_delay=.2));out=(root/'outer.stdout').open('w');err=(root/'outer.stderr').open('w');p=subprocess.Popen(command(root,'success'),stdout=out,stderr=err)
        stop=None
        try:
            end=time.monotonic()+30
            while time.monotonic()<end:
                rows=list(root.glob('evidence/*/*/watchdog-ownership.json'))
                log=root/'rpc.log'
                if rows and log.exists() and any(json.loads(line)['actor']=='watchdog' for line in log.read_text().splitlines()):
                    # Stop while the watchdog is waiting for RPC, outside its shared local-budget lock.
                    stop=z.read(rows[0])['pid'];os.kill(stop,19);break
                time.sleep(.01)
            self.assertIsNotNone(stop);self.assertNotEqual(p.wait(timeout=50),0)
            v=z.read(next(root.glob('evidence/*/*/custody.json')));self.assertFalse(v['watchdog_collected']);self.assertIsNone(v['watchdog_exit']);self.assertFalse(v['closed'])
        finally:
            if stop:
                os.kill(stop,18)
                if p.poll() is None:p.wait(timeout=50)
                try:pid,status=os.waitpid(stop,0)
                except ChildProcessError:pid,status=stop,None
                z.write(root/'outer-watchdog-collection.json',dict(pid=pid,wait_status=status,exit=os.waitstatus_to_exitcode(status) if status is not None else None,method='subreaper exact waitpid after CONT; no false disarm'))
            out.close();err.close()
            if p.poll() is None:p.kill();p.wait()
        z.write(root/'outer-collection.json',dict(pid=p.pid,exit=p.returncode,ready_sha256=z.sha(root/'READY.json')))

def production_delayed():
    # Exact review counterexample: W90 + 5*9.9 RPC + 10 local seconds, plus pre-Tw work.
    root=fixture(dict(hold_running=True,controller_unknown=[3],watch_rpc_delay=9.9,watch_local_delay=10,pre_tw_delay=1),scale=1)
    code,v=execute(root,'success',timeout=215);assert v['watchdog_exit']==0,(root,v)
    out=pathlib.Path(v['binding']['evidence']);own=z.read(out/'watchdog-ownership.json');end=z.read(out/'watchdog-result.json')['monotonic'];assert end-own['T0']>145.2
    assert end<=own['cleanup_collection_deadline'];assert (out/'watchdog-closure.json').exists()
    z.write(root/'production-valued-assertions.json',dict(W=90,five_rpc_total=49.5,local_work=10,pre_Tw=1,elapsed=end-own['T0'],watchdog_exit=v['watchdog_exit'],full_budget_claim=False,scope='one production-valued delayed ordinary case, isolated executables only'))
    print(root)
if __name__=='__main__':
    if '--production-delayed' in sys.argv:production_delayed()
    else:unittest.main()
