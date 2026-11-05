"""AH3 qualification assertions against real published server and consumers."""
import hashlib, json, os, pathlib, select, subprocess, urllib.request, urllib.error

SID = '11111111-1111-4111-8111-111111111111'

def save(out, name, value):
    (out / (name + '.json')).write_text(json.dumps(value, indent=2))

def prepare(fixture):
    source = pathlib.Path(os.environ['A42_OWNER_SOURCE'])
    c = fixture / 'codex'; cl = fixture / 'claude'; state = fixture / 'query-state'
    for p in [c, cl, state]: p.mkdir()
    path = c / ('sessions/2026/01/01/rollout-fixture-' + SID + '.jsonl')
    path.parent.mkdir(parents=True)
    records = [
        {'type':'session_meta','timestamp':'2026-01-01T00:00:00Z','payload':{'cwd':str(fixture),'timestamp':'2026-01-01T00:00:00Z'}},
        {'type':'response_item','timestamp':'2026-01-01T00:00:01Z','payload':{'type':'message','role':'user','content':[{'type':'input_text','text':'hello fresh AH3 webridge_'+'A'*43}]}}
    ]
    path.write_text(''.join(json.dumps(r)+'\n' for r in records))
    (c/'session_index.jsonl').write_text(json.dumps({'id':SID,'thread_name':'AH3 synthetic'})+'\n')
    owner = fixture / 'query-owner.py'
    owner.write_bytes((source/'skills/session-events/scripts/session_events.py').read_bytes())
    profile = fixture/'query-profile.json'
    profile.write_text(json.dumps({'version':1,'owner':str(owner),'owner_sha256':hashlib.sha256(owner.read_bytes()).hexdigest(),'codex_home':str(c),'claude_home':str(cl),'state_dir':str(state),'bridge':None,'context':None}))
    profile.chmod(0o600)
    return profile

def capture(fixture, out, stage):
    values = []
    for p in sorted(fixture.rglob('*')):
        if p.is_file() and not p.is_symlink():
            data=p.read_bytes(); values.append({'path':str(p),'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
    save(out, stage+'-inputs', values)

def rpc(proc, request, out, label):
    proc.stdin.write((json.dumps(request)+'\n').encode());proc.stdin.flush()
    assert select.select([proc.stdout], [], [], 45)[0], ('MCP timeout',label)
    result=json.loads(proc.stdout.readline())
    save(out,label,{'request':request,'response':result})
    return result

def http(config, op, args, request_id):
    data=json.dumps({'schema_version':1,'operation':op,'arguments':args,'request_id':request_id}).encode()
    req=urllib.request.Request('http://'+config['listen']+'/v1/session-queries',data=data,headers={'Content-Type':'application/json','Authorization':'Bearer '+pathlib.Path(config['credential_file']).read_text()})
    with urllib.request.urlopen(req, timeout=45) as response: return json.load(response)

def client(config, family, op, args, out, label, expect_success=True):
    argv=[os.environ['AH3_CLIENT_BINARY'],config['listen'],config['credential_file'],family,op,json.dumps(args)]
    result=subprocess.run(argv,capture_output=True,text=True,timeout=45)
    value=json.loads(result.stdout)
    save(out,label,{'argv':argv,'returncode':result.returncode,'response':value,'stderr':result.stderr})
    assert (result.returncode==0)==expect_success,(label,value)
    return value

def query_suite(proc, fixture, config, out):
    expected_identity='sha256:'+hashlib.sha256((fixture/'server.json').read_bytes()+(fixture/'query-profile.json').read_bytes()+(fixture/'task-profile.json').read_bytes()).hexdigest()
    profile=fixture/'cli-profile.json'
    profile.write_text(json.dumps({'version':1,'kind':'local-service','endpoint':config['listen'],'credential_file':config['credential_file'],'timeout_seconds':30}));profile.chmod(0o600)
    capture(fixture,out,'pre-consumers')
    if config['mcp']:
        init=rpc(proc,{'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'AH3','version':'1'}}},out,'mcp-initialize')
        assert init['result']['protocolVersion']=='2025-06-18'
        proc.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');proc.stdin.flush()
        discovery=rpc(proc,{'jsonrpc':'2.0','id':2,'method':'tools/list','params':{}},out,'mcp-discovery')
        assert {t['name'] for t in discovery['result']['tools']}=={'list_sessions','read_session','project_owner_task','consume_owner_answer','read_owner_answer'}
    else:
        proc.stdin.write(b'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n');proc.stdin.flush()
        assert not select.select([proc.stdout],[],[],.2)[0], 'disabled MCP responded'
        save(out,'mcp-disabled',{'config_mcp':False,'no_stdio_response':True})
    for index,(op,args) in enumerate([('list_sessions',{}),('read_session',{'session_id':SID})]):
        h=http(config,op,args,'joined-query')
        assert h['server_identity']==expected_identity
        assert h['owner_result'] is not None,h
        assert 'webridge_'+'A'*43 not in json.dumps(h)
        assert h['owner_result']['matched']==1 if op=='list_sessions' else h['owner_result']['entries'][0]['text']=='hello fresh AH3 [REDACTED:bridge_credential]'
        save(out,op+'-http',{'arguments':args,'response':h})
        r=client(config,'query',op,args,out,op+'-rust')
        assert r['owner_result']==h['owner_result'] and r['server_identity']==expected_identity and r['server_build']==h['server_build']
        argfile=fixture/(op+'-args.json');argfile.write_text(json.dumps(args));argfile.chmod(0o600)
        argv=[os.environ['AH3_QUERY_BINARY'],str(profile),op,'--json',str(argfile)]
        run=subprocess.run(argv,text=True,capture_output=True,timeout=45);cli=json.loads(run.stdout)
        save(out,op+'-cli',{'argv':argv,'returncode':run.returncode,'output':cli,'stderr':run.stderr})
        assert run.returncode==0 and cli['ok'] and cli['envelope']['owner_result']==h['owner_result'] and cli['envelope']['server_identity']==expected_identity
        if config['mcp']:
            m=rpc(proc,{'jsonrpc':'2.0','id':10+index,'method':'tools/call','params':{'name':op,'arguments':args}},out,op+'-mcp')['result']['structuredContent']
            assert m['owner_result']==h['owner_result'] and m['server_identity']==expected_identity
    # Command reads stay disabled in this common task+query profile.
    denied=client(config,'command','read_session_operation',{},out,'command-disabled')
    assert denied.get('owner_result') is None and (denied.get('error') or denied.get('owner_error'))
    wrong=fixture/'wrong-token';wrong.write_text('B'*40);wrong.chmod(0o600)
    bad=dict(config,credential_file=str(wrong))
    assert client(bad,'query','list_sessions',{},out,'rust-denied',False)['transport_error']=='transport_rejected'
    save(out,'common-profile',{'listen':config['listen'],'mcp':config['mcp'],'server_identity':expected_identity,'task_profile':str(fixture/'task-profile.json'),'query_profile':str(fixture/'query-profile.json'),'actual_query_cli':os.environ['AH3_QUERY_BINARY'],'actual_public_Rust_client':os.environ['AH3_CLIENT_BINARY']})

def signed_read(proc,config,record,out):
    args=record['input']
    h=http(config,'read_owner_answer',args,'ah3-exact-read')
    r=client(config,'task','read_owner_answer',args,out,'signed-read-rust')
    assert r['owner_result']==h['owner_result'],(r,h)
    assert r['owner_result']['signed']==args['signed']==record['decision']['signed']
    save(out,'signed-read-http',h)
    if config['mcp']:
        # Restarted MCP session must initialize again.
        rpc(proc,{'jsonrpc':'2.0','id':20,'method':'initialize','params':{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'AH3-restart','version':'1'}}},out,'restart-mcp-initialize')
        m=rpc(proc,{'jsonrpc':'2.0','id':21,'method':'tools/call','params':{'name':'read_owner_answer','arguments':args}},out,'signed-read-mcp')['result']['structuredContent']
        assert m['owner_result']==h['owner_result']
    # Change only answer identity: read never consumes a previously unknown answer.
    unknown=json.loads(json.dumps(args));answer=json.loads(unknown['signed']['answer_json']);answer['answer_id']='22222222-2222-4222-8222-222222222222';unknown['signed']['answer_json']=json.dumps(answer)
    u=client(config,'task','read_owner_answer',unknown,out,'unknown-exact-read')
    assert u.get('owner_result') is None and u.get('owner_error'),u

def protocol_consume(proc, fixture, config, browser, out):
    """A separate fresh answer for each real protocol's first consume."""
    publications=json.loads((fixture/'sync-state/ah-selections.json').read_text())
    assert len(publications)==1
    binding=next(iter(publications.values()))
    value=browser['submitted'];response=value['credential']['response']
    args={'selection':binding['selection'],'signed':{
        'answer_json':value['answer_json'],'credential_id':value['credential']['id'],
        'authenticator_data':response['authenticatorData'],'client_data_json':response['clientDataJSON'],'signature':response['signature']}}
    # Exact valid signature is unknown until this single first consume.
    unknown=http(config,'read_owner_answer',args,'before-consume')
    save(out,'before-consume-unknown',unknown)
    assert unknown.get('owner_result') is None and unknown.get('owner_error'),unknown
    if config['mcp']:
        result=rpc(proc,{'jsonrpc':'2.0','id':30,'method':'tools/call','params':{'name':'consume_owner_answer','arguments':args}},out,'first-consume-mcp')['result']['structuredContent']
    else:
        result=client(config,'task','consume_owner_answer',args,out,'first-consume-rust')
    decision=result['owner_result'];assert decision is not None,result
    assert decision['signed']==args['signed'] and decision['request_json']==binding['request_json'] and decision['selection']==args['selection']
    read=http(config,'read_owner_answer',args,'after-consume')
    save(out,'after-consume-read-http',read)
    for field in ['decision','signed','request_json','selection','current_standing','execution_admission']:
        assert read['owner_result'][field]==decision[field],(field,read,decision)
    rust=client(config,'task','read_owner_answer',args,out,'after-consume-read-rust')
    assert rust['owner_result']==read['owner_result']
    if config['mcp']:
        m=rpc(proc,{'jsonrpc':'2.0','id':31,'method':'tools/call','params':{'name':'read_owner_answer','arguments':args}},out,'after-consume-read-mcp')['result']['structuredContent']
        assert m['owner_result']==read['owner_result']
    import sqlite3
    with sqlite3.connect(fixture/'registry.sqlite') as db:
        effects={t:db.execute('SELECT count(*) FROM '+t).fetchone()[0] for t in ['admission','outbox','capacity_claim','scoped_claim','pi_claim']}
    assert all(v==0 for v in effects.values())
    save(out,'protocol-result',{'synthetic_only':True,'first_consume_transport':'MCP' if config['mcp'] else 'public Rust LocalClient','signed':args['signed'],'selection':args['selection'],'decision':decision,'server_identity':result['server_identity'],'server_build':result['server_build'],'effects':effects,'fresh_fixture':str(fixture),'consumer_config':config})
