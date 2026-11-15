"""One invocation, independent watchdog, physical custody distinct from process exit."""
import argparse, json, math, os, pathlib, re, shutil, subprocess, sys, time, traceback
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
from realization import Subject, Actor, limits, read, write, sha, require, exact
import cleanup

def watchdog(root,context_path):
    s=Subject(root); require(pathlib.Path(__file__).resolve()==s.root/'controller.py','executing watchdog subject mismatch'); x=read(context_path); b=x['binding']; require(s.invocation(b['mode'],b['invocation_id'])==b,'watchdog binding')
    require(pathlib.Path(context_path)==pathlib.Path(b['evidence'])/'watchdog-context.json','watchdog context path')
    t0=x['T0'];tw=x['Tw']; lim=limits(s.t,b['mode']); deadline=min(t0+lim['watch'],x['outer_deadline'])
    require(t0<=tw<=time.monotonic() and x['trigger']==tw+lim['W'],'watchdog anchors')
    a=Actor(s,b,'watchdog',deadline,start=tw,budget_fd=x['budget_fd']); out=pathlib.Path(b['evidence']);cid=x['container'];closed=False
    try:
        a.save(out/'watchdog-start.json',dict(container=cid,T0=t0,Tw=tw,trigger=x['trigger'],deadline=deadline))
        while time.monotonic()<min(x['trigger'],deadline):
            if cleanup.confirmation(out,cid,b):
                a.save(out/'watchdog-disarmed.json',dict(container=cid,validated_confirmation=True)); return 0 if not a.publication_errors and not a.accounting_errors else 2
            a.sleep(min(s.t['P'],max(0,min(x['trigger'],deadline)-time.monotonic())))
        closed=cleanup.cleanup(cid,out,'watchdog',s.r['controller_session_id'],a.rpc,a)
        a.save(out/'watchdog-result.json',dict(container=cid,closed=closed,disarmed=False));return 0 if closed and not a.publication_errors and not a.accounting_errors else 2
    except BaseException as e:
        a.publication_errors.append(dict(stage='watchdog',error=repr(e)));return 3
    finally:
        delivered=a.deliver(out/'watchdog-custody.json',dict(container=cid,closed=closed,custody_owner=s.r['root_session_id'],deadline=deadline,finished=time.monotonic()))
        if delivered['delivery_attempt_errors']:return 3

def main(root,mode,invocation,outer_wall):
    T0=time.monotonic(); wall0=time.time(); s=None; b=None; out=None; watch=None; attach=None;cid=None;closed=False;ambiguous=False;errors=[];watch_exit=None;attach_exit=None
    # A refusal can occur before writable evidence exists. stderr is outer custody.
    s=Subject(root); require(pathlib.Path(__file__).resolve()==s.root/'controller.py','executing controller subject mismatch'); b=s.invocation(mode,invocation); lim=limits(s.t,mode)
    require(type(outer_wall) is float and math.isfinite(outer_wall) and outer_wall-wall0>=lim['parent']+30*s.r['timing_scale'],'outer collection deadline insufficient')
    outer_deadline=T0+min(s.t['O'],outer_wall-wall0-30*s.r['timing_scale'])
    deadline=min(T0+lim['parent'],outer_deadline); a=Actor(s,b,'controller',deadline,start=T0)
    require(deadline-time.monotonic()>=lim['parent']-(s.t['L']), 'validation overhead exhausted')
    if mode=='fixture':s.release_check(b,lim['parent'])
    runtime=pathlib.Path(b['runtime']);out=pathlib.Path(b['evidence'])
    # Exclusive whole invocation directories make duplicate allocation a refusal.
    runtime.parent.mkdir(parents=True,exist_ok=True);runtime.mkdir();out.parent.mkdir(parents=True,exist_ok=True);out.mkdir()
    for k in ['output','tmp']:
        p=pathlib.Path(b[k]);p.mkdir(mode=0o777);p.chmod(0o777);require(not list(p.iterdir()) and os.access(p,os.W_OK),'unusable run directory')
    a.establish_budget()
    sid=s.r['controller_session_id'];label=s.r['owner_label_key'];name='u2-q-'+sid[:8]+'-'+mode+'-'+invocation
    argv=[s.r['host_executables']['docker'],'create','--pull=never','--name',name,'--label',label+'='+sid,'--network=none','--cap-drop=ALL','--security-opt=no-new-privileges','--read-only','--user','1000:1000','--pids-limit','128','--memory','2g','--cpus','2','--tmpfs','/tmp:rw,nosuid,nodev,size=67108864']
    for key,val in a.labels().items():argv+=['--label',key+'='+val]
    for k in ['output','tmp']:argv+=['--mount',f'type=bind,source={b[k]},target={b[k]}']
    for m in s.c['mounts']:
        if mode=='missing' and m['target']=='/bin/kill':continue
        argv+=['--mount',f'type=bind,source={m["source"]},target={m["target"]},readonly']
    argv+=['--mount',f'type=bind,source={s.closure},target={s.r["entrypoints"]["closure"]},readonly']
    env=dict(s.c['environment'],TMPDIR=b['tmp'],UI_TEST_EVIDENCE_DIR=b['output'],U2_OUTER_OUTPUT=b['output'],U2_CLOSURE=s.r['entrypoints']['closure'],U2_ABSOLUTE_DEADLINE=str(deadline))
    for k,v in env.items():argv+=['--env',k+'='+v]
    argv+=['--entrypoint',s.r['entrypoints']['container_python'],s.c['image'],'-I',s.r['entrypoints']['pid1'],s.r['modes'][mode]]
    try:
        require(a.save(out/'create-argv.json',dict(argv=argv,attempted_name=name,owner_label={label:sid},T0=T0,parent_deadline=deadline,outer_deadline=outer_deadline,limits=lim)), 'pre-effect publication failed')
        require(time.monotonic()<T0+s.t['C']+s.t['L'],'precreate time exhausted')
        if mode=='fixture':s.consume(b)
        create=a.rpc(*argv[1:],timeout=s.t['C'])
        if create['returncode']==0 and re.fullmatch('[0-9a-f]{64}\n?',create['stdout']) and create['stderr']=='':cid=create['stdout'].strip()
        else:
            ambiguous=True
            o,r=cleanup.inspect(name,sid,a.rpc,label,a.labels())
            if o['kind']=='Present':cid=o['object']['Id']
            a.save(out/'ambiguous-create-custody.json',dict(attempted_name=name,owner_label={label:sid},observation=o,response=r,closed=False,custody_owner=s.r['root_session_id']))
            raise RuntimeError('ambiguous create; no create retry')
        a.save(out/'create.json',dict(response=create,attempted_name=name,owner_label={label:sid},container=cid))
        a.save(out/'ownership.json',dict(container=cid,attempted_name=name,owner_label={label:sid},controller_pid=os.getpid()))
        Tw=time.monotonic();context=out/'watchdog-context.json'
        require(a.save(context,dict(container=cid,T0=T0,Tw=Tw,trigger=Tw+lim['W'],outer_deadline=outer_deadline,budget_fd=a.budget_fd)), 'watchdog context publication failed')
        wf=(out/'watchdog.stdout').open('w');we=(out/'watchdog.stderr').open('w')
        try:watch=a.acquire(subprocess.Popen([s.r['host_executables']['python'],'-I',str(s.root/'controller.py'),'_watchdog','--subject',str(s.root),'--context',str(context)],start_new_session=True,stdout=wf,stderr=we,pass_fds=(a.budget_fd,)),[s.r['host_executables']['python'],'-I',str(s.root/'controller.py'),'_watchdog','--subject',str(s.root),'--context',str(context)],'watchdog')
        finally:wf.close();we.close()
        a.save(out/'watchdog-ownership.json',dict(container=cid,pid=watch.pid,T0=T0,Tw=Tw,trigger=Tw+lim['W'],cleanup_collection_deadline=min(T0+lim['watch'],outer_deadline),parent_deadline=deadline))
        o,r=cleanup.inspect(cid,sid,a.rpc,label,a.labels());a.save(out/'prestart-inspect.json',dict(observation=o,response=r));require(o['kind']=='Present','prestart Unknown/absent')
        info=o['object'];h=info['HostConfig']
        require(h['NetworkMode']=='none' and h['CapDrop']==['ALL'] and h['ReadonlyRootfs'] is True and info['Config']['User']=='1000:1000','isolation mismatch')
        require('no-new-privileges' in h['SecurityOpt'] and h['PidMode']!='host' and h['IpcMode']!='host' and not h['Privileged'] and not h.get('CapAdd'),'privilege mismatch')
        af=(out/'attach.stdout').open('w');ae=(out/'attach.stderr').open('w')
        try:attach=a.acquire(subprocess.Popen([s.r['host_executables']['docker'],'start','--attach',cid],stdout=af,stderr=ae),[s.r['host_executables']['docker'],'start','--attach',cid],'attach')
        finally:af.close();ae.close()
        a.save(out/'attach-ownership.json',dict(pid=attach.pid,container=cid))
        if mode=='watchdog':
            watch_exit=a.wait(watch,min(T0+lim['watch'],outer_deadline)-time.monotonic())
            require(watch_exit==0,'watchdog unsuccessful')
            a.wait(attach,s.t['S'])
        elif mode=='attachment':
            preeffect(a,attach,pathlib.Path(b['output'])/'pre-effect.json')
            attach.kill();a.wait(attach,s.t['S'])
        else:
            if mode=='timeout':preeffect(a,attach,pathlib.Path(b['output'])/'pre-effect.json')
            try:a.wait(attach,s.t['A_fixture'] if mode=='fixture' else s.t['A_timeout'] if mode=='timeout' else s.t['A'])
            except subprocess.TimeoutExpired:attach.kill();a.wait(attach,s.t['S'])
        attach_exit=attach.returncode
        o,r=cleanup.inspect(cid,sid,a.rpc,label,a.labels());a.save(out/'after-attach-inspect.json',dict(observation=o,response=r))
        for n in ['pre-effect.json','pre-effect-refusal.json','before-boundary-exit.json']:
            p=pathlib.Path(b['output'])/n
            if p.exists():a.save(out/n,dict(runtime_path=str(p),sha256=sha(p),result=p.read_text()))
        if mode in ['success','observer-sentinel']:require(attach_exit==0,'attach success expected')
        if mode=='failure':require(attach_exit==37,'attach failure37 expected')
        if mode in ['timeout','attachment']:require(o['kind']=='Present' and o['object']['State']['Running'],'attach loss must retain running object')
        if mode=='missing':require(attach_exit!=0 and (out/'pre-effect-refusal.json').exists() and not (out/'pre-effect.json').exists(),'missing preflight refusal expected')
        else:require((out/'pre-effect.json').exists(),'pre-effect missing')
    except BaseException as e:errors.append(dict(stage='controller',error=repr(e)))
    finally:
        # Each collection is independently guarded: a failed write/cleanup cannot skip it.
        try:
            if attach:
                if attach.poll() is None:attach.kill()
                attach_exit=a.wait(attach,s.t['S'])
        except BaseException as e:errors.append(dict(stage='attach collection',error=repr(e)))
        try:
            if cid:
                closed=cleanup.cleanup(cid,out,'controller',sid,a.rpc,a)
                if closed:
                    cp=out/'controller-closure.json';a.save(out/'cleanup-confirmed.json',dict(container=cid,terminal_removed=True,closure=dict(path=str(cp),sha256=sha(cp))))
        except BaseException as e:errors.append(dict(stage='cleanup/confirmation',error=repr(e)))
        try:
            if watch:
                watch_exit=a.wait(watch,s.t['O'])
                if watch_exit!=0:errors.append(dict(stage='watchdog exit',exit=watch_exit))
        except BaseException as e:errors.append(dict(stage='watchdog collection',error=repr(e)))
        errors.extend(dict(stage='local budget during collection',error=e) for e in a.accounting_errors)
        errors.extend(dict(stage='publication',**e) for e in a.publication_errors)
        errors.extend(dict(stage='unresolved child',**row) for _,row in a.children if not row['collected'])
        custody=dict(binding=b,actor='controller',actor_pid=os.getpid(),container=cid,attempted_name=name,owner_label={label:sid},closed=closed,ambiguous_create=ambiguous,attach_pid=attach.pid if attach else None,attach_exit=attach_exit,watchdog_pid=watch.pid if watch else None,watchdog_exit=watch_exit,watchdog_collected=(watch is None or watch_exit is not None),errors=errors,custody_owner=s.r['root_session_id'],T0=T0,finished=time.monotonic(),deadline=deadline)
        # No implicit transfer of OS wait ownership to root.
        custody=a.deliver(out/'custody.json',custody)
        errors.extend(dict(stage='final delivery attempt',**e) for e in custody['delivery_attempt_errors'])
    return 0 if not errors and closed else 2

def preeffect(a,attach,path):
    end=min(time.monotonic()+a.s.t['E'],a.deadline)
    while not path.exists():
        require(time.monotonic()<end and attach.poll() is None,'pre-effect timeout/attach exit')
        a.sleep(min(a.s.t['P'],max(0,end-time.monotonic())))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('mode');p.add_argument('--subject',required=True);p.add_argument('--invocation');p.add_argument('--outer-deadline',type=float);p.add_argument('--context');args=p.parse_args()
    try:code=watchdog(args.subject,args.context) if args.mode=='_watchdog' else main(args.subject,args.mode,args.invocation,args.outer_deadline)
    except BaseException as e:print(json.dumps(dict(refused=True,error=repr(e),actor_pid=os.getpid())),file=sys.stderr);code=3
    raise SystemExit(code)
