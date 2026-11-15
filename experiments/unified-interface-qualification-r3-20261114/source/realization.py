"""The closure realization is the single definition; READY is an acyclic seal."""
import mmap, struct, datetime, fcntl, hashlib, json, math, os, pathlib, re, subprocess, sys, time
Path = pathlib.Path
MODES = {'success':'success','failure':'failure','timeout':'timeout','attachment':'attachment',
         'watchdog':'timeout','missing':'success','observer-sentinel':'observer-sentinel','fixture':'fixture'}
CEILINGS = dict(C=20,R=10,S=5,E=45,P=.2,I=240,O=420,J=5,L=40,A=20,A_timeout=5,A_fixture=270,W=90,W_watchdog=60,W_fixture=300)
REQUIRED_SOURCE = ('controller.py','cleanup.py','realization.py','pid1.py','preflight.py','test_cleanup.py','test_entrypoint.py','fake_docker.py','fake_fact.py','test_custody.py')
def require(ok, message):
    if not ok: raise ValueError(message)
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def read(p): return json.loads(Path(p).read_text())
def exact(p):
    p=Path(p); require(p.is_absolute() and str(p)==str(p.resolve()), 'noncanonical/aliased path: '+str(p)); return p
def child(root, rel):
    require(isinstance(rel,str) and rel and not Path(rel).is_absolute(), 'relative path required')
    p=exact(root/rel); require(p.is_relative_to(root) and p!=root, 'escaping path: '+rel); return p
def write(p,v):
    """Atomic durable fact, including directory persistence; errors remain errors."""
    p=Path(p); tmp=p.with_name(p.name+'.'+str(os.getpid())+'.tmp')
    try:
        with tmp.open('x') as f:
            json.dump(v,f,indent=2); f.write('\n'); f.flush(); os.fsync(f.fileno())
        os.replace(tmp,p)
        fd=os.open(p.parent,os.O_RDONLY|os.O_DIRECTORY)
        try: os.fsync(fd)
        finally: os.close(fd)
    finally:
        if tmp.exists(): tmp.unlink()
def limits(t,mode):
    R,S,K=t['R'],t['S'],5*t['R']; W=t['W_fixture'] if mode=='fixture' else t['W_watchdog'] if mode=='watchdog' else t['W']
    D=W+t['P']+K
    M=(D+S+R+S if mode=='watchdog' else R+t['E']+t['A_timeout']+S+R+R+S if mode=='timeout' else R+t['E']+S+R+R+S if mode=='attachment' else R+(t['A_fixture'] if mode=='fixture' else t['A'])+S+R+S)
    return dict(W=W,watch=t['C']+D+t['L'],parent=t['C']+max(M+K,D)+t['J']+t['L'],outer=t['O'])
class Subject:
    def __init__(self,root):
        self.root=exact(root); self.closure=child(self.root,'closure.json'); self.c=read(self.closure); self.r=self.c['realization']
        r=self.r; require(r['version']==1,'unknown realization version'); require(exact(r['subject_root'])==self.root,'subject root mismatch')
        require(r['receipt_relpath']=='READY.json','exact READY.json required'); self.ready=child(self.root,r['receipt_relpath'])
        seal=read(self.ready); require(seal['version']==1 and seal['status']=='finalized','unfinalized READY')
        require(seal['preparation_evidence']==[], 'unsupported preparation evidence graph')
        bindings=seal['bindings']; require(set(REQUIRED_SOURCE)|{'closure.json'} <= set(bindings),'missing required READY bindings')
        for rel,digest in bindings.items():
            require(rel!='READY.json' and not rel.startswith('evidence/'),'cyclic/dependent READY binding')
            require(sha(child(self.root,rel))==digest,'READY binding mismatch: '+rel)
        self.pins=seal['pins']; require(set(self.pins)==set(r['host_executables'].values()),'incomplete executable pins')
        for p,digest in self.pins.items(): require(sha(exact(p))==digest and os.access(p,os.X_OK),'executable pin mismatch: '+p)
        require(r['host_executables']['python']==str(Path(sys.executable).resolve()),'host interpreter mismatch')
        require(r['entrypoints']['controller']=='controller.py' and r['entrypoints']['cleanup']=='cleanup.py','consumer path mismatch')
        require(r['modes']==MODES,'unsupported mode mapping')
        require(r['entrypoints']['closure']=='/u2-control/closure.json' if r['qualification_class']=='production' else r['entrypoints']['closure']=='/sentinel/closure.json','closure destination mismatch')
        for k in ['root_session_id','controller_session_id','fixture_run_id','owner_label_key']:
            require(isinstance(r[k],str) and re.fullmatch('[A-Za-z0-9_.-]+',r[k]),'invalid identity: '+k)
        self.runtime=exact(r['runtime_root']); self.release=exact(r['release_path']); self.fence=child(self.root,r['consumed_fence_relpath'])
        require(r['consumed_fence_relpath']=='fixture-consumed.json','unstable fixture fence')
        require(self.release.parent==self.root.parent and self.release.name==self.root.name+'-run-release.json','release path mismatch')
        require(r['run_layout']=='runs/{run}/{mode}-{invocation}' and r['evidence_layout']=='evidence/{run}/{mode}-{invocation}','layout mismatch')
        self.t=r['timing']; require(set(self.t)==set(CEILINGS),'timing keys')
        scale=r['timing_scale']; require(type(scale) in (int,float) and 0<scale<=1,'timing scale')
        require(all(type(self.t[k]) in (int,float) and self.t[k]==v*scale for k,v in CEILINGS.items()),'timing ceiling mismatch')
        require(r['qualification_class'] in ['production','isolated-controlled'],'qualification class')
        if r['qualification_class']=='production':
            require(scale==1 and r['host_executables'].keys()=={'python','docker'},'production dependencies/timing')
        else:
            require('/Q-repair-r3/qualification/' in str(self.root) and self.runtime.is_relative_to(self.root),'synthetic isolation')
            require(all(exact(e['source']).is_relative_to(self.root) for e in self.c['entries']),'synthetic runtime input escaped')
            require(all(exact(m['source']).is_relative_to(self.root) for m in self.c['mounts']),'synthetic mount escaped')
            origin=self.c.get('qualification_source')
            if origin is not None:
                original=exact(origin['path']);require(original.name=='READY.json' and sha(original)==origin['sha256'],'qualification source READY mismatch')
                original_seal=read(original)
                require(all(original_seal['bindings'][name]==sha(self.root/name) for name in REQUIRED_SOURCE),'qualification source code differs')
        require(not {'TMPDIR','UI_TEST_EVIDENCE_DIR','U2_OUTER_OUTPUT','U2_CLOSURE'} & self.c['environment'].keys(),'dynamic environment duplicated')
        require(isinstance(self.c['image'],str) and re.fullmatch('sha256:[0-9a-f]{64}',self.c['image']),'image must be pinned')
        require(all(isinstance(k,str) and k and '=' not in k and '\x00' not in k and isinstance(v,str) and '\x00' not in v for k,v in self.c['environment'].items()),'invalid static environment')
        destinations={}; destination_rows={}
        for e in self.c['entries']:
            require(isinstance(e['destination'],str) and e['destination'].startswith('/') and os.path.normpath(e['destination'])==e['destination'],'invalid destination')
            p=exact(e['source']); require(p.is_file() and sha(p)==e['sha256'],'runtime digest: '+str(p))
            require(p.stat().st_size==e['bytes'] and p.stat().st_mode&0o777==e['mode'],'runtime metadata: '+str(p))
            destination_rows.setdefault(e['destination'],[]).append(e); destinations[e['destination']]=e
        aliases=r['destination_aliases']
        require(set(aliases)=={dest for dest,rows in destination_rows.items() if len(rows)>1},'unqualified destination aliases')
        for dest,alias in aliases.items():
            rows=destination_rows[dest]
            require(sorted(alias['sources'])==sorted(e['source'] for e in rows),'alias source identities')
            require(all((e['sha256'],e['mode'],e['bytes'])==(rows[0]['sha256'],rows[0]['mode'],rows[0]['bytes']) for e in rows),'alias bytes/metadata disagree')
            require(any(m['target']==dest and m['source']==alias['selected_source'] for m in self.c['mounts']),'alias resolution must be an exact nested mount')
            destinations[dest]=next(e for e in rows if e['source']==alias['selected_source'])
        for key in ['pid1','preflight','observer','container_python']:
            require(r['entrypoints'][key] in destinations,'entrypoint not in closure: '+key)
        for e in self.c['entries']:
            require(any(e['destination']==m['target'] and e['source']==m['source'] or e['destination'].startswith(m['target']+'/') and e['source']==m['source']+e['destination'][len(m['target']):] for m in self.c['mounts']),'unmapped runtime entry: '+e['destination'])
        for key in ['pid1','preflight']:
            require(destinations[r['entrypoints'][key]]['source']==str(self.root/(key+'.py')) if r['qualification_class']=='production' else True,'consumer source mismatch: '+key)
        if r['qualification_class']=='production':require(self.c['environment']['U2_FIXTURE_BINARY']==r['entrypoints']['observer'],'observer environment mismatch')
        for m in self.c['mounts']:
            require(isinstance(m['target'],str) and m['target'].startswith('/') and os.path.normpath(m['target'])==m['target'],'invalid mount destination')
            exact(m['source'])
            require(m['readonly'] is True and Path(m['source']).exists(),'invalid static mount')
            require(any(e['destination']==m['target'] or e['destination'].startswith(m['target']+'/') for e in self.c['entries']),'unbound mount')
        self.binding=dict(ready_path=str(self.ready),ready_sha256=sha(self.ready),closure_path=str(self.closure),closure_sha256=sha(self.closure),run_id=r['fixture_run_id'],controller_session_id=r['controller_session_id'])
    def invocation(self,mode,invocation):
        require(mode in self.r['modes'],'unsupported mode'); require(re.fullmatch('[A-Za-z0-9-]{8,80}',invocation or '') is not None,'invocation identity')
        run=self.r['fixture_run_id']; suffix=f'{mode}-{invocation}'
        runtime=child(self.runtime,f'runs/{run}/{suffix}'); out=child(self.root,f'evidence/{run}/{suffix}')
        return dict(self.binding,mode=mode,invocation_id=invocation,runtime=str(runtime),output=str(runtime/'output'),tmp=str(runtime/'tmp'),evidence=str(out))
    def release_check(self,b,envelope):
        d=read(self.release); now=time.time()
        require(all(type(d[k]) in (int,float) and math.isfinite(d[k]) for k in ['not_before','expires_at','outer_collection_deadline','collection_reserve']),'invalid release time')
        expected=dict(issuer_session_id=self.r['root_session_id'],ready_path=str(self.ready),ready_sha256=sha(self.ready),closure_sha256=sha(self.closure),fixture_run_id=self.r['fixture_run_id'],permitted_mode='fixture',consumed_fence_path=str(self.fence),permitted_runs=1,execution_authorized=True)
        require(all(d.get(k)==v and type(d.get(k))==type(v) for k,v in expected.items()),'concrete release binding mismatch')
        require(d['not_before']<=now<d['expires_at'],'release interval')
        require(d['collection_reserve']>=30*self.r['timing_scale'],'collection reserve')
        require(now+envelope+d['collection_reserve']<=min(d['outer_collection_deadline'],d['expires_at']),'release deadline insufficient')
        require(not os.path.lexists(self.fence),'consumed/uncertain fence')
        return d
    def consume(self,b):
        # Revalidate immediately before irreversible create intent; no retry or deletion.
        self.release_check(b,limits(self.t,'fixture')['parent'])
        with self.fence.open('x') as f:
            json.dump(dict(binding=b,release_path=str(self.release),release_sha256=sha(self.release),consumed_at=time.time()),f); f.flush(); os.fsync(f.fileno())
        fd=os.open(self.fence.parent,os.O_DIRECTORY|os.O_RDONLY)
        try:os.fsync(fd)
        finally:os.close(fd)
class Actor:
    """Retained operational custody; publication is an independently failing projection."""
    def __init__(self,subject,binding,actor,deadline,start=None,budget_fd=None):
        self.s=subject; self.b=binding; self.actor=actor; self.deadline=deadline
        self.last=time.monotonic() if start is None else start
        self.accounting_errors=[];self.publication_errors=[];self.children=[];self.responses=[];self.observations=[]
        self.budget_fd=budget_fd;self.budget=None;self.last_projection=0
        if budget_fd is not None:self.budget=mmap.mmap(budget_fd,8)
    def establish_budget(self):
        # Established before effects. Binary accounting is separate from ordinary facts.
        p=Path(self.b['evidence'])/'local-budget.bin'
        self.budget_fd=os.open(p,os.O_CREAT|os.O_EXCL|os.O_RDWR,0o600)
        os.write(self.budget_fd,struct.pack('d',0.0));self.budget=mmap.mmap(self.budget_fd,8)
    def labels(self):
        key=self.s.r['owner_label_key'];return {key+'.run':self.b['run_id'],key+'.invocation':self.b['invocation_id']}
    def remaining(self,ceiling):
        v=min(ceiling,self.deadline-time.monotonic());require(v>0,'absolute deadline exhausted');return v
    def charge(self):
        now=time.monotonic();delta=max(0,now-self.last);self.last=now
        try:
            require(self.budget is not None,'aggregate budget unavailable')
            # POSIX record locks are process owned, including across inherited fd.
            fcntl.lockf(self.budget_fd,fcntl.LOCK_EX)
            try:
                total=struct.unpack('d',self.budget[:])[0];require(math.isfinite(total) and total>=0,'invalid aggregate budget')
                total+=delta;self.budget[:]=struct.pack('d',total)
            finally:fcntl.lockf(self.budget_fd,fcntl.LOCK_UN)
            require(total<=self.s.t['L'],'shared aggregate local overhead exhausted')
            if now-self.last_projection>=1:
                self.last_projection=now
                try:write(Path(self.b['evidence'])/'local-budget.json',dict(binding=self.b,total=total,limit=self.s.t['L'],accounting='shared binary mapping; projection only'))
                except BaseException as e:self.publication_errors.append(dict(path='local-budget.json',error=repr(e)))
            return total
        except BaseException as e:
            self.accounting_errors.append(repr(e));raise
    def acquire(self,proc,argv,kind):
        row=dict(pid=proc.pid,argv=list(argv),kind=kind,owner_pid=os.getpid(),binding=self.b,started=time.monotonic(),exit=None,collected=False,collection_error=None)
        self.children.append((proc,row));return proc
    def collected(self,proc):
        for p,row in self.children:
            if p is proc:row.update(exit=proc.returncode,collected=proc.returncode is not None,completed=time.monotonic())
    def wait(self,process,ceiling):
        try:self.charge()
        except BaseException:pass
        try:
            v=process.wait(timeout=self.remaining(ceiling));self.collected(process);return v
        except BaseException as e:
            for p,row in self.children:
                if p is process:row['collection_error']=repr(e)
            raise
        finally:self.last=time.monotonic()
    def sleep(self,seconds):
        self.charge();start=time.monotonic()
        try:time.sleep(min(seconds,self.remaining(seconds)))
        finally:self.last+=min(seconds,time.monotonic()-start)
    def save(self,p,v):
        # Keep the operational observation even if accounting/hook/filesystem fails.
        record=dict(v,binding=self.b,actor=self.actor,actor_pid=os.getpid(),record_path=str(p),monotonic=time.monotonic())
        self.observations.append(record)
        try:
            self.charge();require(time.monotonic()<self.deadline,'fact deadline exhausted')
            hook=self.s.r['host_executables'].get('fact_hook')
            if hook:
                argv=[hook,str(p)];local_start=time.monotonic();proc=self.acquire(subprocess.Popen(argv),argv,'fact-hook')
                try:
                    code=self.wait(proc,self.s.t['L']);self.last=local_start;self.charge();require(code==0,'fact hook failed: '+str(p))
                except BaseException:
                    if proc.poll() is None:proc.kill()
                    try:self.wait(proc,self.s.t['S'])
                    except BaseException:pass
                    raise
            write(p,record);self.charge();return True
        except BaseException as e:
            self.publication_errors.append(dict(path=str(p),error=repr(e)));return False
    def snapshot(self):
        return dict(binding=self.b,actor=self.actor,actor_pid=os.getpid(),children=[dict(row) for _,row in self.children],responses=self.responses,observations=self.observations,publication_errors=self.publication_errors,accounting_errors=self.accounting_errors,os_wait_owner=os.getpid(),outer_adoption='unestablished')
    def deliver(self,p,v):
        # Each route is attempted independently. Delivery is only established by collector.
        try:self.charge()
        except BaseException:pass
        payload=dict(v,custody=self.snapshot(),within_actor_deadline=time.monotonic()<self.deadline,delivery_tail_bound='requires actual outer capture timing; no universal guarantee');failures=[]
        try:write(p,payload)
        except BaseException as e:failures.append(dict(route=str(p),error=repr(e)))
        payload['delivery_attempt_errors']=failures
        for stream in [sys.stdout,sys.stderr]:
            try:stream.write(json.dumps(payload)+'\n');stream.flush();return payload
            except BaseException as e:failures.append(dict(route=getattr(stream,'name','stream'),error=repr(e)))
        return payload
    def rpc(self,*args,timeout=None):
        proc=None;started=time.monotonic();argv=[self.s.r['host_executables']['docker'],*args]
        result=dict(returncode=None,stdout='',stderr='',process=None)
        try:
            self.charge();started=time.monotonic();end=min(self.deadline,started+(timeout or self.s.t['R']))
            proc=self.acquire(subprocess.Popen(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True),argv,'rpc')
            stdout,stderr=proc.communicate(timeout=self.remaining(end-time.monotonic()));self.collected(proc)
            result.update(returncode=proc.returncode,stdout=stdout,stderr=stderr)
        except BaseException as e:
            result.update(stderr=repr(e),exception=type(e).__name__,partial_stdout=getattr(e,'stdout',None),partial_stderr=getattr(e,'stderr',None))
            for key in ['partial_stdout','partial_stderr']:
                if isinstance(result[key],bytes):result[key]=result[key].decode(errors='replace')
            if proc:
                try:
                    if proc.poll() is None:proc.kill()
                    stdout,stderr=proc.communicate(timeout=self.remaining(self.s.t['S']));self.collected(proc)
                    result.update(stdout=stdout,stderr=stderr)
                except BaseException as error:
                    for p,row in self.children:
                        if p is proc:row['collection_error']=repr(error)
        finally:
            self.last=time.monotonic()
            result['process']=next((dict(row) for p,row in self.children if p is proc),dict(pid=None,argv=argv,collected=False))
            self.responses.append(result)
        return result
