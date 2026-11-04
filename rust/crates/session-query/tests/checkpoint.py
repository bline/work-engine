"""Offline integration with actual session-events MCP and actual Rust bridge.

The SQLite rows are test-only preexisting operations; no submit or native server is run.
"""
import hashlib, importlib.util, json, os, pathlib, shutil, sqlite3, subprocess, sys, tempfile
root = pathlib.Path(__file__).resolve().parents[4]
owner = root / 'skills/session-events/scripts/session_events.py'
exe = pathlib.Path(os.environ['SESSION_QUERY_BINARY'])
bridge_exe = pathlib.Path(os.environ['SESSION_BRIDGE_BINARY'])
fixture_root = pathlib.Path(os.environ['SESSION_QUERY_FIXTURE_ROOT'])
spec = importlib.util.spec_from_file_location('session_events_test', owner)
m = importlib.util.module_from_spec(spec); sys.modules[spec.name] = m; spec.loader.exec_module(m)
sha = lambda b: hashlib.sha256(b).hexdigest()
compact = lambda v: json.dumps(v, separators=(',', ':'), ensure_ascii=False).encode()
sid='11111111-1111-4111-8111-111111111111'
req='22222222-2222-4222-8222-222222222222'
gid='33333333-3333-4333-8333-333333333333'
opid='44444444-4444-4444-8444-444444444444'
turn='55555555-5555-4555-8555-555555555555'
token='webridge_'+'A'*43
with tempfile.TemporaryDirectory(prefix='query-',dir=fixture_root) as tmp:
    t = pathlib.Path(tmp); t.chmod(0o700)
    c, cl, state = (t / n for n in ('codex', 'claude', 'state'))
    workspace=t/'workspace'; workspace.mkdir(mode=0o700)
    locks=t/'locks'; locks.mkdir(mode=0o700)
    path = c / ('sessions/2026/01/01/rollout-fixture-'+sid+'.jsonl'); path.parent.mkdir(parents=True)
    records = [{'type':'session_meta','timestamp':'2026-01-01T00:00:00Z','payload':{'cwd':str(workspace),'timestamp':'2026-01-01T00:00:00Z'}},
      {'type':'response_item','timestamp':'2026-01-01T00:00:01Z','payload':{'type':'message','role':'user','content':[{'type':'input_text','text':'fixture '+token}]}},
      {'type':'event_msg','timestamp':'2026-01-01T00:00:02Z','payload':{'type':'task_complete','turn_id':turn,'last_agent_message':'complete '+token}}]
    path.write_text(''.join(json.dumps(r)+'\n' for r in records)); os.utime(path,(1767225600,1767225600))
    config=t/'config.json'; db=t/'state.sqlite'; binary=t/'bridge'; shutil.copyfile(bridge_exe,binary); binary.chmod(0o700)
    init=subprocess.run([str(binary),'--config',str(config),'store','init','--database',str(db),'--lock-dir',str(locks),'--native-socket',str(t/'unused.sock')],capture_output=True,check=True,timeout=10)
    assert json.loads(init.stdout)['ok'], init.stdout
    wire='preexisting wire'; wire_hash=sha(wire.encode()); payload_hash=sha(b'preexisting payload')
    identity={'protocol':'session-bridge-rpc-v1','envelope':'runtime-envelope-v1','profile':'dedicated-loaded-session-v1','request_id':req,'session_id':sid,'workspace':str(workspace),'payload_sha256':payload_hash,'wire_sha256':wire_hash}
    operation_hash=sha(compact(identity)); token_hash=sha(('work-engine/session-bridge/token/v1\0'+token).encode())
    wirefile=locks/(opid+'.input'); wirefile.write_text(wire); wirefile.chmod(0o600)
    with sqlite3.connect(db) as sql:
        sql.execute('INSERT INTO grants VALUES (?,?,?,?,?,?,?,?,?,?,?,?)',(gid,token_hash,req,sid,str(workspace),payload_hash,wire_hash,operation_hash,4102444800,os.geteuid(),1767225600,0))
        sql.execute('INSERT INTO operations VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)',(req,opid,gid,sid,str(workspace),payload_hash,wire_hash,operation_hash,opid,1767225600,'terminal','[]',turn,turn,'completed',1767225600))
    profile={'version':1,'owner':str(owner),'owner_sha256':sha(owner.read_bytes()),'codex_home':str(c),'claude_home':str(cl),'state_dir':str(state), 'bridge':{'binary':str(binary),'binary_sha256':sha(binary.read_bytes()),'config':str(config),'config_sha256':sha(config.read_bytes())}}
    prof=t/'profile.json'; prof.write_text(json.dumps(profile)); prof.chmod(0o600)
    owner_bridge=m.Bridge(str(binary),sha(binary.read_bytes()),str(config),sha(config.read_bytes()))
    server=m.Server(m.Homes(c,cl,state),'local-query',m.CallbackPolicy(),bridge=owner_bridge)
    def cli(name,args, expected=0, fmt='--json'):
        p=subprocess.run([str(exe),str(prof),name,fmt,'-'],input=json.dumps(args),capture_output=True,text=True,timeout=40)
        assert p.returncode==expected,(name,p.stdout,p.stderr)
        assert token not in p.stdout+p.stderr
        return json.loads(p.stdout) if fmt=='--json' else p.stdout
    for name,args in [('list_sessions',{}),('read_session',{'session_id':sid}),('read_session_operation',{'request_id':req,'grant_credential':token}),('read_session_result',{'request_id':req,'grant_credential':token})]:
        expected=server.call_tool({'name':name,'arguments':args})['structuredContent']
        actual=cli(name,args)
        assert actual == {'schema':'work-engine.session-query.v1','ok':True,'result':expected},name
        v=actual['result']
        if name=='list_sessions': assert v['matched']==1 and v['sessions'][0]['sessionId']==sid
        if name=='read_session': assert v['entries'][0]['redactions']==1 and '[REDACTED:bridge_credential]' in v['entries'][0]['text']
        if name=='read_session_operation': assert v['entry_state']=='terminal' and v['native_status']=='completed' and v['workers']['available'] is False and v['process_closure']['available'] is False and v['usage']['available'] is False
        if name=='read_session_result': assert v['operation']['operation_id']==opid and v['completion']['status']=='completed' and v['result']['available'] and '[REDACTED:bridge_credential]' in v['result']['text'] and not v['usage']['available'] and not v['process_closure']['available']
        print(name,'matches actual MCP and assertions')
    # An explicitly configured real context reaches the same owner projection.
    role=t/'role-ledger.jsonl'; role.write_text('')
    profile['context']={'budget_root':None,'role_ledger':str(role),'ledger_path':None,'ledger_sha256':None}
    prof.write_text(json.dumps(profile))
    context=m.Context.build(m.Homes(c,cl,state),role_ledger=role)
    observed=m.Server(m.Homes(c,cl,state),'local-query',m.CallbackPolicy(),context=context,bridge=owner_bridge)
    args={'request_id':req,'grant_credential':token}
    direct=observed.call_tool({'name':'read_session_operation','arguments':args})['structuredContent']
    via=cli('read_session_operation',args)['result']
    assert via==direct and via['workers']['availability']=='partial' and via['workers']['sources']['role_host']=='available'
    profile.pop('context'); prof.write_text(json.dumps(profile))
    print('configured owner context preserved partial worker observation')
    # Existing ambiguous state remains ambiguous on repeated reads, with no native entry.
    with sqlite3.connect(db) as sql:
        sql.execute("UPDATE operations SET state='identity_conflict', native_status=NULL, turn_id=NULL WHERE request_id=?",(req,))
        sql.execute('INSERT INTO occupancy VALUES (?,?)',(sid,req))
    for _ in range(2):
        v=cli('read_session_operation',{'request_id':req,'grant_credential':token})['result']
        assert v['entry_state']=='identity_conflict' and v['native_status'] is None
    with sqlite3.connect(db) as sql:
        assert sql.execute('SELECT state FROM operations').fetchone()[0]=='identity_conflict'
        assert sql.execute('SELECT count(*) FROM transitions').fetchone()[0]==0
    # Unknown entry remains unknown when native history is unavailable. No retry/submission occurs.
    with sqlite3.connect(db) as sql:
        sql.execute("UPDATE operations SET state='entry_uncertain' WHERE request_id=?",(req,))
    for _ in range(2):
        assert cli('read_session_operation',{'request_id':req,'grant_credential':token},1)['error']['code']=='history_unavailable'
    with sqlite3.connect(db) as sql:
        assert sql.execute('SELECT state FROM operations').fetchone()[0]=='entry_uncertain'
        assert sql.execute('SELECT count(*) FROM transitions').fetchone()[0]==0
    for name,args,code in [('submit_session_request',{},'invalid_query'),('list_sessions',{'mutation':True},'invalid_arguments'),('read_session',{'session_id':req+'x'},'invalid_arguments'),('read_session_operation',{'request_id':req,'grant_credential':'wrong'},'unauthorized'),('read_session_operation',{'request_id':sid,'grant_credential':token},'unauthorized'),('read_session_result',{'request_id':req,'grant_credential':token,'input_text':token},'invalid_arguments')]:
        assert cli(name,args,1)['error']['code']==code
    assert 'session(s)' in cli('list_sessions',{},fmt='--human')
    # Bounded private input rejects oversized data, public modes, and final symlinks.
    huge=subprocess.run([str(exe),str(prof),'list_sessions','--json','-'],input=' ' * 262145,capture_output=True,text=True,timeout=10)
    assert huge.returncode and json.loads(huge.stdout)['error']['code']=='input_too_large'
    public=t/'public.json'; public.write_text('{}'); public.chmod(0o644)
    p=subprocess.run([str(exe),str(prof),'list_sessions','--json',str(public)],capture_output=True,text=True)
    assert p.returncode and json.loads(p.stdout)['error']['code']=='invalid_file'
    link=t/'link'; link.symlink_to(public)
    p=subprocess.run([str(exe),str(prof),'list_sessions','--json',str(link)],capture_output=True,text=True)
    assert p.returncode and json.loads(p.stdout)['error']['code']=='file_unavailable'
    profile['version']=2; prof.write_text(json.dumps(profile)); assert cli('list_sessions',{},1)['error']['code']=='invalid_profile'
    profile['version']=1; profile['owner_sha256']='0'*64; prof.write_text(json.dumps(profile)); assert cli('list_sessions',{},1)['error']['code']=='invalid_profile'
    print('negative, redaction, authorization and repeated ambiguous read assertions passed; no native service or entry')
