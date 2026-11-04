#!/usr/bin/python3
"""DRAFT live4 one-shot selected schema-7/schema-2 semantic continuation; execute only after owner admission."""
import hashlib, json, os, pathlib, select, shutil, signal, socket, sqlite3, subprocess, sys, threading, time

HERE=pathlib.Path(__file__).resolve().parent
TRIAL=HERE.parent
MANIFEST=TRIAL/'candidate-manifest-v1.json'  # created and frozen after authorized exact build
HOST=TRIAL/'target/controlled/debug/work-engine-task-host'
NATIVE=pathlib.Path('/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex')
FAKE_PROVIDER=TRIAL/'target/controlled/debug/offline_rehearsal_provider'
AUTHORITY=TRIAL/'live-execution-authority-v1.json'  # absent until fresh explicit owner approval
MAP=TRIAL/'experimental/correspondence-map.json'
RECORDER_START=TRIAL/'experimental/recorder-start.json'
LIVE_STATE=TRIAL/'live4-state'
FAKE_STATE=TRIAL/'rehearsal-state'
CASE='s10-live4-selected-schema7-owner2-two-cycle-semantic'
TASK='task:s8:analysis'
KEY='OPENAI_API_KEY'
MAX_TURNS=13
WALL_SECONDS=1200

def sha(p): return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def save(p,obj):
    p=pathlib.Path(p);p.write_text(json.dumps(obj,sort_keys=True,separators=(',',':'))+'\n');p.chmod(0o600)
def query(db,sql,args=()):
    con=sqlite3.connect('file:'+str(db)+'?mode=ro',uri=True)
    try: return con.execute(sql,args).fetchall()
    finally: con.close()
def exchange(sock_path,obj):
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
        s.settimeout(8);s.connect(str(sock_path));s.sendall(json.dumps(obj,separators=(',',':')).encode()+b'\n')
        data=b''
        while not data.endswith(b'\n'):
            piece=s.recv(131072)
            if not piece: raise RuntimeError('host socket EOF')
            data+=piece
    return json.loads(data)
def stop(proc,seconds=15):
    if proc is None:return {'exit_code':None,'forced':False,'group_quiescent':True,
                            'detached_descendants_verified':False}
    pgid=proc.pid  # start_new_session=True for every launched group
    forced=False
    try:os.killpg(pgid,signal.SIGTERM)
    except ProcessLookupError:pass
    try:exit_code=proc.wait(timeout=seconds)
    except subprocess.TimeoutExpired:
        forced=True
        try:os.killpg(pgid,signal.SIGKILL)
        except ProcessLookupError:pass
        exit_code=proc.wait(timeout=5)
    # A remaining group is uncertain, including a detached native descendant.
    try:os.killpg(pgid,0);quiescent=False
    except ProcessLookupError:quiescent=True
    return {'exit_code':exit_code,'forced':forced,'group_quiescent':quiescent,
            'detached_descendants_verified':False}

mode=sys.argv[1:] if len(sys.argv)>1 else []
assert mode in (['--offline-rehearsal'],['--live-armed']), 'explicit one-shot mode required'
assert os.environ.get('S10_LIVE4_ADMISSION')=='authorized', 'frozen obligations/key and owner admission required even for rehearsal'
scope_gate=pathlib.Path(os.environ['S10_LIVE4_SCOPE_GATE'])
scope_deadline=time.monotonic()+20
while not scope_gate.is_file() and time.monotonic()<scope_deadline:time.sleep(.05)
assert scope_gate.is_file(), 'external user-scope controller did not establish containment before work'
scope_identity=json.loads(scope_gate.read_text())
assert scope_identity['runner_pid']==os.getpid() and scope_identity['control_group'].startswith('/user.slice/')
live=mode==['--live-armed']
state=LIVE_STATE if live else FAKE_STATE
route='openai-responses-live-v1' if live else 'openai-responses-fake-v1'
assert MANIFEST.is_file(), 'exact candidate manifest must be frozen after build and admission'
manifest=json.loads(MANIFEST.read_text())
assert manifest['schema']=='s10-live4-execution-candidate-v1'
assert manifest['source_manifest_sha256']==sha(TRIAL/'source-manifest-v1.json')
assert manifest['selected_source_manifest_sha256']==sha(TRIAL/'selected-source-manifest-v1.json')
assert manifest['host_binary_sha256']==sha(HOST)
assert manifest['native_binary_sha256']==sha(NATIVE)
assert manifest['public_obligations_sha256']==sha(TRIAL/'public-obligations-v1.json')
assert MAP.is_file() and RECORDER_START.is_file(), 'prospective recorder must start before execution'
assert json.loads(RECORDER_START.read_text())['session_id'], 'actual recorder session ID required'
assert sha(TRIAL/'selected-source-manifest-v1.json')==manifest['selected_source_manifest_sha256']
assert 'ORION-7' not in (HERE/'initial-submit-payload.json').read_text()
for item in json.loads((TRIAL/'source-manifest-v1.json').read_text())['entries']:
    assert sha(TRIAL/'source-snapshot'/item['path'])==item['sha256'],item['path']
for item in json.loads((TRIAL/'selected-source-manifest-v1.json').read_text()):
    assert sha(TRIAL/'source-snapshot'/item['path'])==item['sha256'],item['path']
if live:
    assert AUTHORITY.is_file() and sha(AUTHORITY)==manifest['live_authority_sha256']
    decision=json.loads(AUTHORITY.read_text())
    assert decision['root_only_provider_launch'] is True and decision['paid_live_authorized'] is True
    assert decision['live_api_planning_allowance_usd']==10 and decision['attempts']==1
    assert os.environ.get('S10_LIVE4_ROOT_LAUNCH')=='authorized', 'root-only live marker required'
    key=os.environ.get(KEY)
else:
    assert KEY not in os.environ, 'rehearsal parent must have no real provider credential'
    key='offline-rehearsal-only-token'
assert key and len(key)<=512 and all(0x21<=ord(ch)<=0x7e for ch in key)
assert not state.exists(), 'one-shot state path already exists; no replay'
state.mkdir(mode=0o700);os.chmod(state,0o700)
log=state/'logs';log.mkdir(mode=0o700)
event_snapshots=log/'event-snapshots';event_snapshots.mkdir(mode=0o700)
ack_dir=state/'recorder-acks';ack_dir.mkdir(mode=0o700)
for name in ['native','barrier','private-home','private-codex','private-tmp','private-target']:
    (state/name).mkdir(mode=0o700)
barrier=state/'barrier'
for point in ['native_before_turn_entry','native_after_turn_send_intent','native_after_result_commit','semantic_before_turn_entry']:
    (barrier/(point+'.release')).write_text('release\n')
shutil.copy2(TRIAL/'selected-source-manifest-v1.json',state/'selected-source-manifest.json')
shutil.copy2(HERE/'initial-submit-payload.json',state/'initial-submit-payload.json')
cfg=json.loads((HERE/'startup-template.json').read_text().replace('@STATE@',str(state)).replace('@HOST_SHA256_AFTER_BUILD@',sha(HOST)))
now_ms=int(time.time()*1000)
for grant in cfg['lifecycle']['grants']: grant['expires_wall_ms']=now_ms+1_800_000
cfg['provider_identity']=route
cfg['lifecycle']['native']['provider_route']=route
provider=None;host=None;host_err=None
provider_url=''
if not live:
    provider_err=(log/'offline-provider.stderr.log').open('wb')
    try:
        provider=subprocess.Popen([str(FAKE_PROVIDER),str(log/'offline-provider-ledger.jsonl'),str(state)],
            cwd=state,env={'PATH':'/usr/bin:/bin'},stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=provider_err,text=True,start_new_session=True)
        address=b'';ready_deadline=time.monotonic()+10
        while b'\n' not in address and time.monotonic()<ready_deadline:
            ready,_,_=select.select([provider.stdout],[],[],max(0,ready_deadline-time.monotonic()))
            if not ready:break
            chunk=os.read(provider.stdout.fileno(),1024)
            if not chunk:break
            address+=chunk
            assert len(address)<2048, 'offline provider readiness too large'
        assert b'\n' in address, 'offline provider readiness timeout'
        provider_url=address.split(b'\n',1)[0].decode().strip()
        assert provider_url.startswith('http://127.0.0.1:') and provider_url.endswith('/v1')
        cfg['lifecycle']['native']['simulator_base_url']=provider_url
    except BaseException:
        stop(provider);provider_err.close();raise
else:
    assert cfg['lifecycle']['native']['simulator_base_url']==''
save(state/'host-startup.json',cfg)
env={'HOME':str(state/'private-home'),'CODEX_HOME':str(state/'private-codex'),
     'TMPDIR':str(state/'private-tmp'),'CARGO_TARGET_DIR':str(state/'private-target'),
     'PATH':'/usr/bin:/bin','RUST_LOG':'error',KEY:key}
life_db=pathlib.Path(cfg['lifecycle']['store_root'])/'lifecycle.sqlite'
task_db=pathlib.Path(cfg['task_store_path'])
host_sock=pathlib.Path(cfg['socket_path'])
deadline=time.monotonic()+WALL_SECONDS
deadline_hit=threading.Event()
def terminate_at_deadline():
    deadline_hit.set()
    if host is not None and host.poll() is None:host.send_signal(signal.SIGTERM)
deadline_timer=threading.Timer(WALL_SECONDS,terminate_at_deadline)
deadline_timer.daemon=True;deadline_timer.start()
events=[]
def record(label,value):
    events.append({'label':label,'wall_ms':int(time.time()*1000),'value':value})
    save(log/'events.json',events)
def recorder_boundary(label):
    # Control-plane boundary and ack stay outside product/raw evidence.
    immutable_event=event_snapshots/(label+'.json')
    assert not immutable_event.exists(), 'one-shot recorder boundary'
    shutil.copy2(log/'events.json',immutable_event)
    event_sha=sha(immutable_event)
    save(ack_dir/(label+'.boundary.json'),{'label':label,'event_sha256':event_sha})
    ack=ack_dir/(label+'.json')
    def acknowledged():
        if not ack.is_file():return False
        value=json.loads(ack.read_text())
        return (value.get('event_sha256')==event_sha
                and value.get('map_sha256')==sha(MAP)
                and value.get('recorder_session_id')==json.loads(RECORDER_START.read_text())['session_id'])
    wait_for('recorder acknowledgement '+label,acknowledged,90)
def turns():
    return query(life_db,'SELECT purpose,state,send_intent FROM native_turns ORDER BY rowid') if life_db.exists() else []
def wait_for(label,predicate,seconds):
    local=min(deadline,time.monotonic()+seconds)
    while time.monotonic()<local:
        if deadline_hit.is_set():raise TimeoutError('global wall deadline')
        if host is not None and host.poll() is not None: raise RuntimeError(f'host exited during {label}: {host.returncode}')
        if len(turns())>MAX_TURNS: raise RuntimeError(f'native turn bound exceeded during {label}')
        if life_db.exists() and query(life_db,'SELECT COUNT(*) FROM native_operations')[0][0]>1:
            raise RuntimeError(f'snapshot operation bound exceeded during {label}')
        if life_db.exists() and query(life_db,'SELECT COUNT(*) FROM native_task_operations')[0][0]>0:
            raise RuntimeError(f'unexpected task operation during {label}')
        value=predicate()
        if value:return value
        time.sleep(.05)
    raise TimeoutError(label)
def observe(delivery=None):
    req={'op':'task.observe.v2','task_id':TASK}
    if delivery:req['delivery_id']=delivery
    value=exchange(host_sock,req)
    assert value['status']=='observation',value
    return value
def owner_revision(value): return value['components']['owner']['value']['revision']
def replacement(command,revision,reason):
    value=exchange(host_sock,{'op':'task.request_replacement.v1','payload':{
        'task_id':TASK,'command_id':command,'expected_task_revision':revision,
        'authority_ref':'operator:s8:authorized','reason':reason}})
    assert value['status']=='replacement_requested',value
    record(command,value)
    return value
def submit(command,question):
    payload=json.loads((state/'initial-submit-payload.json').read_text())
    payload.update(command_id=command,engineering_question=question,facts=[])
    value=exchange(host_sock,{'op':'task.submit.v1','payload':payload})
    assert value['status']=='admitted',value
    record(command,value)
    return value
def barrier_file(point,kind):return barrier/(point+'.'+kind)
def fake_entries():
    path=log/'offline-provider-ledger.jsonl'
    return len(path.read_text().splitlines()) if path.exists() else 0

status='failed'
try:
    prep=subprocess.run(['/usr/bin/python3',str(HERE/'prepare_schema7_route.py'),
        str(state),str(HOST),str(state/'initial-submit-payload.json')],env=env,cwd=state,
        stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=45,check=False)
    save(log/'preparation-status.json',{'exit_code':prep.returncode,'stdout_sha256':hashlib.sha256(prep.stdout).hexdigest(),
        'stderr_sha256':hashlib.sha256(prep.stderr).hexdigest()})
    assert prep.returncode==0, 'schema7 predecessor preparation failed; inspect retained logs'
    prepared=json.loads(prep.stdout)
    assert prepared['status']=='prepared' and prepared['native_turn_count']==0 and prepared['queued_input_count']==1
    record('prepared',prepared)
    selection=pathlib.Path(prepared['selection_v3_path'])
    host_err=(log/'selected-host.stderr.log').open('wb')
    host=subprocess.Popen([str(HOST),'serve','--selected',str(selection)],env=env,cwd=state,start_new_session=True,
        stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=host_err)
    wait_for('selected host ready',lambda:host_sock.exists(),20)
    admitted=prepared['admitted'];delivery=admitted['lifecycle']['outcome_ref']
    first=observe(delivery);record('selected-host-start',first)
    def first_window():
        read=query(life_db,"SELECT COUNT(*) FROM native_operations WHERE state='completed_settled'")[0][0]
        domain=query(life_db,"SELECT state,final_sha256 FROM native_turns WHERE purpose='domain_work' ORDER BY rowid")
        return read==1 and len(domain)==1 and domain[0][0]=='entered_uncertain' and domain[0][1] is None and (live or fake_entries()>=2)
    wait_for('D0 first entered provider after committed snapshot read',first_window,150)
    before_a=observe(delivery);record('before-replacement-a',before_a)
    replacement('command:s8:replace-a',owner_revision(before_a),'Context replacement A after approved snapshot read')
    if not live:(state/'release-d0-final').write_text('release\n')
    def first_reconciled():
        transition=query(life_db,'SELECT stage FROM transitions ORDER BY created_wall_ms')
        return transition==[('reconciled',)]
    wait_for('first semantic transition reconciled',first_reconciled,300)
    sem=query(life_db,'SELECT phase,verdict,eligibility FROM semantic_results ORDER BY rowid')
    assert sem==[('candidate_verify','accepted','current'),('successor_adequacy','accepted','current')],sem
    record('after-commit-a',sem)
    recorder_boundary('after-first-reconciliation')
    def second_window():
        domain=query(life_db,"SELECT state,final_sha256 FROM native_turns WHERE purpose='domain_work' ORDER BY rowid")
        return len(domain)==2 and domain[1][0]=='entered_uncertain' and domain[1][1] is None and (live or fake_entries()>=7)
    wait_for('D0 g1 successor entered provider',second_window,90)
    after_a=observe(delivery);record('d0-g1-entered',after_a)
    q1=submit('command:s8:q1','Q1: Which approved snapshot constraint remains active?')
    q2=submit('command:s8:q2','Q2: What attributed result follows without repository mutation?')
    q1_id=q1['lifecycle']['outcome_ref'];q2_id=q2['lifecycle']['outcome_ref']
    queued=observe(q2_id);record('q1-q2-queued',queued)
    replacement('command:s8:replace-b',owner_revision(queued),'Context replacement B while D0 remains entered and Q1/Q2 remain queued')
    if not live:(state/'release-d0-g1-final').write_text('release\n')
    def second_reconciled():
        transition=query(life_db,'SELECT stage FROM transitions ORDER BY created_wall_ms')
        return transition==[('reconciled',),('reconciled',)]
    wait_for('second semantic transition reconciled',second_reconciled,300)
    sem=query(life_db,'SELECT phase,verdict,eligibility FROM semantic_results ORDER BY rowid')
    assert sem==[('candidate_verify','accepted','current'),('successor_adequacy','accepted','current')]*2,sem
    record('after-commit-b',sem)
    recorder_boundary('after-second-reconciliation')
    def settled():
        epoch=query(life_db,'SELECT predecessor_context,successor_context,stage FROM transitions ORDER BY created_wall_ms')
        if len(epoch)!=2 or any(row[2]!='reconciled' for row in epoch):return False
        rows=query(life_db,'SELECT i.sequence,a.outcome,a.settlement_kind FROM inputs i LEFT JOIN effects e ON e.input_id=i.input_id LEFT JOIN attempts a ON a.effect_id=e.effect_id ORDER BY i.sequence')
        return len(rows)==3 and all(row[1:] == ('completed','established') for row in rows)
    wait_for('two transitions and D0/Q1/Q2 settlement',settled,300)
    final=observe(q2_id);record('final-observation',final)
    assert len(turns())==MAX_TURNS,turns()
    assert all(row[1]=='completed_settled' for row in turns()),turns()
    owner=json.loads(query(task_db,'SELECT record_json FROM tasks WHERE task_id=?',(TASK,))[0][0])
    assert len(owner['completed_effects'])==1 and owner['result'] is None
    attribution=owner['completed_effects'][0].get('attribution')
    assert attribution and attribution['schema']=='completed-snapshot-read-attribution-v2'
    assert attribution['requested_length']==attribution['returned_length']==7
    assert attribution['result']['availability']=='inline_utf8' and attribution['result']['text']=='ORION-7'
    assert [x['input_id'] for x in owner['inputs']]==['input:command:s8:d0','input:command:s8:q1','input:command:s8:q2']
    assert owner['fact_kinds']=={'approval':'human_interaction','commitment':'commitment','decision':'decision','reference':'canonical_reference','rule':'governing','uncertainty':'uncertainty'}
    segments=query(life_db,'SELECT queue_attempt_id,segment_no,state FROM native_delivery_segments ORDER BY queue_attempt_id,segment_no')
    assert segments==[('attempt:1',0,'yielded'),('attempt:1',1,'yielded'),('attempt:1',2,'completed'),('attempt:2',0,'completed'),('attempt:3',0,'completed')],segments
    record('settled-readback',{'semantic_results':sem,'segments':segments,'turns':turns(),'owner_result':owner['result']})
    assert not deadline_hit.is_set()
    if not live:
        provider.wait(timeout=8)
        assert provider.returncode==0
    status='passed'
except Exception as error:
    record('failure',{'type':type(error).__name__,'message':str(error)[:2000]})
finally:
    deadline_timer.cancel()
    host_stop=stop(host)
    if host_err:host_err.close()
    provider_stop=stop(provider)
    if not live:provider_err.close()
    # Process-group quiescence does not cover descendants that created a new session.
    # A separate supervised-descendant gate must establish this before positive live admission.
    if (host_stop['forced'] or not host_stop['group_quiescent']
            or not host_stop['detached_descendants_verified']
            or host_stop['exit_code'] not in (None,0)):
        status='uncertain_teardown'
    if provider_stop['forced'] or not provider_stop['group_quiescent'] or provider_stop['exit_code'] not in (None,0):
        status='uncertain_teardown'
    count_error=None
    try:
        final_turn_count=len(turns()) if life_db.exists() else 0
        final_snapshot_count=query(life_db,'SELECT COUNT(*) FROM native_operations')[0][0] if life_db.exists() else 0
        final_task_operation_count=query(life_db,'SELECT COUNT(*) FROM native_task_operations')[0][0] if life_db.exists() else 0
        if (final_turn_count>MAX_TURNS or final_snapshot_count>1
                or final_task_operation_count>0):
            status='failed'
    except Exception as count_exception:
        final_turn_count=final_snapshot_count=final_task_operation_count=None
        count_error=repr(count_exception);status='uncertain_teardown'
    files=[task_db,task_db.with_name(task_db.name+'-wal'),task_db.with_name(task_db.name+'-shm'),life_db,life_db.with_name(life_db.name+'-wal'),life_db.with_name(life_db.name+'-shm')]
    # Provisional only: external scope controller makes the authoritative copy
    # after the entire cgroup is empty/inactive, including detached descendants.
    copy=state/'stopped-copy-provisional';copy.mkdir(exist_ok=True)
    hashes=[];copy_errors=[]
    for p in files:
        if p.exists():
            try:
                a=sha(p);dst=copy/p.name;shutil.copy2(p,dst);b=sha(p)
                assert a==b==sha(dst)
                hashes.append({'original':str(p),'copy':str(dst),'sha256':a})
            except Exception as copy_error:
                copy_errors.append({'path':str(p),'error':repr(copy_error)})
    if copy_errors:status='uncertain_teardown'
    summary={'schema':'s10-semantic-live-counterpart-run-v1','case':CASE,'mode':'live' if live else 'offline-rehearsal',
        'status':status,'candidate_manifest_sha256':sha(MANIFEST),'host_binary_sha256':sha(HOST),
        'native_binary_sha256':sha(NATIVE),'provider_route':route,'model':'gpt-6-sol',
        'wall_seconds_limit':WALL_SECONDS,'native_turn_limit':MAX_TURNS,'host_exit':host_stop['exit_code'],
        'deadline_terminated':deadline_hit.is_set(),
        'host_stop':host_stop,'provider_stop':provider_stop,
        'provider_process_exit':provider_stop['exit_code'],'stopped_copies':hashes,'copy_errors':copy_errors,'event_count':len(events),
        'final_turn_count':final_turn_count,'final_snapshot_count':final_snapshot_count,
        'final_task_operation_count':final_task_operation_count,'final_count_error':count_error,
        'provider_requests_observed_locally':len((log/'offline-provider-ledger.jsonl').read_text().splitlines()) if not live and (log/'offline-provider-ledger.jsonl').exists() else None,
        'exact_http_requests':None if live else len((log/'offline-provider-ledger.jsonl').read_text().splitlines()) if (log/'offline-provider-ledger.jsonl').exists() else None,
        'exact_billing_usd':None,'no_retry_or_replay':True,'key_persisted_by_runner':False,
        'limits':['Runtime turn count is bounded by this driver; provider HTTP count and dollar cost are not hard-capped.',
                  'A passing offline rehearsal proves orchestration mechanics only, not live semantic adequacy.',
                  'Live result requires separate stopped-state semantic quality and evidence review.']}
    save(log/'result.json',summary)
    print(json.dumps({'status':status,'mode':summary['mode'],'state_dir':str(state),'result':str(log/'result.json')},sort_keys=True))
    if status!='passed':sys.exit(2)
