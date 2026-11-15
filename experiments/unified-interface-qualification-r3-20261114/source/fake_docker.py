#!/usr/bin/python3
"""Isolated executable daemon model. Never dispatches Docker or native binaries."""
import fcntl,json,os,pathlib,sys,time
root=pathlib.Path(__file__).resolve().parent; cfg=json.loads((root/'scenario.json').read_text()); state=root/'daemon.json'; lock=(root/'daemon.lock').open('a')
a=sys.argv[1:]; actor='watchdog' if '_watchdog' in pathlib.Path('/proc/'+str(os.getppid())+'/cmdline').read_bytes().decode(errors='replace') else 'controller'
# All operations and actual process IDs retained independently of controller facts.
with (root/'rpc.log').open('a') as f:f.write(json.dumps(dict(argv=a,pid=os.getpid(),ppid=os.getppid(),actor=actor,time=time.monotonic()))+'\n')
fcntl.flock(lock,fcntl.LOCK_EX)
d=json.loads(state.read_text()) if state.exists() else {};cid='a'*64
if a[0]=='create':
    name=a[a.index('--name')+1];key,val=a[a.index('--label')+1].split('=',1)
    env=dict(x.split('=',1) for i,x in enumerate(a) if i and a[i-1]=='--env')
    labels=dict(x.split('=',1) for i,x in enumerate(a) if i and a[i-1]=='--label')
    mounts=[x for i,x in enumerate(a) if i and a[i-1]=='--mount']
    d=dict(id=cid,name=name,label=labels,missing=not any('target=/bin/kill,' in x or x.endswith('target=/bin/kill') for x in mounts),env=env,running=False,present=True,mode=a[-1],counts={})
    if cfg.get('wronglabels'):d['label']={key:'wrong-owner'}
    state.write_text(json.dumps(d));print(cid if not cfg.get('ambiguous') else 'ambiguous-id');raise SystemExit(0)
if a[0]=='start':
    d['running']=True;state.write_text(json.dumps(d));fcntl.flock(lock,fcntl.LOCK_UN)
    out=pathlib.Path(d['env']['U2_OUTER_OUTPUT']);mode=d['mode']
    if d['missing']:
        (out/'pre-effect-refusal.json').write_text('missing /bin/kill');code=1
    else:
        if not cfg.get('no_pre_effect'):(out/'pre-effect.json').write_text('{"controlled":true}')
        if mode in ['timeout','attachment'] or cfg.get('hold_running'):
            if cfg.get('hold_running') and mode not in ['timeout','attachment']:raise SystemExit(0)
            while True:
                fcntl.flock(lock,fcntl.LOCK_EX);d=json.loads(state.read_text());fcntl.flock(lock,fcntl.LOCK_UN)
                if not d['present'] or not d['running']:raise SystemExit(137)
                time.sleep(.01)
        time.sleep(cfg.get('attach_delay',0))
        code=37 if mode=='failure' else 0
    fcntl.flock(lock,fcntl.LOCK_EX);d=json.loads(state.read_text());d['running']=False;state.write_text(json.dumps(d));raise SystemExit(code)
key=actor+':'+a[0];d.setdefault('counts',{})[key]=d.get('counts',{}).get(key,0)+1;count=d['counts'][key];state.write_text(json.dumps(d));fcntl.flock(lock,fcntl.LOCK_UN)
if actor=='watchdog':
    # Delay includes interpreter startup, leaving the selected RPC's total below R.
    proc_start=int(pathlib.Path('/proc/self/stat').read_text().split()[21])/os.sysconf('SC_CLK_TCK')
    time.sleep(max(0,cfg.get('watch_rpc_delay',0)-(time.monotonic()-proc_start)))
else:time.sleep(cfg.get('controller_rpc_delay',0))
if a[:2]==['container','inspect']:
    if count in cfg.get(actor+'_unknown',[]):print('daemon unknown',file=sys.stderr);raise SystemExit(1)
    if not d.get('present'):
        print('[]');print('Error response from daemon: No such container: '+a[-1],file=sys.stderr);raise SystemExit(1)
    print(json.dumps([dict(Id=cid,Name='/'+d['name'],Config=dict(Labels=d['label'],User='1000:1000'),State=dict(Running=d['running'],Pid=8 if d['running'] else 0),HostConfig=dict(NetworkMode='none',CapDrop=['ALL'],ReadonlyRootfs=True,SecurityOpt=['no-new-privileges'],PidMode='',IpcMode='private',Privileged=False))]));raise SystemExit(0)
fcntl.flock(lock,fcntl.LOCK_EX);d=json.loads(state.read_text())
if a[0]=='kill':d['running']=False
if a[0]=='rm':
    if not d['present']:
        print('Error response from daemon: No such container: '+cid,file=sys.stderr);raise SystemExit(1)
    d['present']=False
state.write_text(json.dumps(d));fcntl.flock(lock,fcntl.LOCK_UN)
if a[0]=='rm' and cfg.get('lost_remove'):print('lost race',file=sys.stderr);raise SystemExit(1)
print(cid)
