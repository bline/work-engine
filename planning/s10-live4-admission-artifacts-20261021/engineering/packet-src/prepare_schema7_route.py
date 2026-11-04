#!/usr/bin/python3
"""Use the exact selected host path to leave one admitted D0 unentered, then select epoch 1."""
import hashlib, json, os, pathlib, signal, socket, sqlite3, subprocess, sys, time

root=pathlib.Path(sys.argv[1]); host=pathlib.Path(sys.argv[2]); payload_path=pathlib.Path(sys.argv[3])
assert root.is_absolute() and host.is_absolute() and payload_path.is_absolute()
assert os.environ.get('OPENAI_API_KEY') and len(os.environ['OPENAI_API_KEY']) <= 512
logs=root/'schema7-prep';logs.mkdir(mode=0o700,exist_ok=False)
def sha(b): return hashlib.sha256(b).hexdigest()
def fsha(p): return sha(pathlib.Path(p).read_bytes())
def save(p,obj):
    p=pathlib.Path(p);p.write_text(json.dumps(obj,sort_keys=True,separators=(',',':'))+'\n');p.chmod(0o600)
def query(db,sql,args=()):
    db=pathlib.Path(db); con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); out=con.execute(sql,args).fetchall(); con.close(); return out
def until(predicate,seconds,label):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        if predicate(): return
        time.sleep(.02)
    raise TimeoutError(label)
def exchange(sock_path,obj):
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as sock:
        sock.settimeout(5);sock.connect(str(sock_path));sock.sendall(json.dumps(obj,separators=(',',':')).encode()+b'\n')
        data=b''
        while not data.endswith(b'\n'):
            part=sock.recv(131072)
            if not part: raise RuntimeError('host socket EOF')
            data+=part
    return json.loads(data)
config_path=root/'host-startup.json';config=json.loads(config_path.read_text())
assert config['features']==['controlled-proof'] and config['provider_identity'] in ('openai-responses-live-v1','openai-responses-fake-v1')
assert fsha(host)==config['executable_sha256']
assert ((config['provider_identity']=='openai-responses-live-v1' and config['lifecycle']['native']['simulator_base_url']=='') or (config['provider_identity']=='openai-responses-fake-v1' and config['lifecycle']['native']['simulator_base_url'].startswith('http://127.0.0.1:')))
selection_path=root/'selection-v1.json'
selection={'version':1,'selection_id':'selected-s10-semantic-predecessor','executable_path':str(host),'executable_sha256':fsha(host),'config_path':str(config_path),'config_sha256':fsha(config_path),'task_store_path':config['task_store_path'],'lifecycle_store_root':config['lifecycle']['store_root'],'host_socket_path':config['socket_path'],'lifecycle_socket_path':config['lifecycle']['socket_path'],'build_id':config['lifecycle']['build_id'],'features':config['features'],'provider_identity':config['provider_identity']}
save(selection_path,selection)
barrier=pathlib.Path(config['lifecycle']['proof_barrier_dir']);arm=barrier/'native_before_spawn.arm';arm.write_text('arm\n')
host_socket=pathlib.Path(config['socket_path']);life_root=pathlib.Path(config['lifecycle']['store_root']);task_db=pathlib.Path(config['task_store_path'])
private_home=root/'private-home';private_codex=root/'private-codex';private_tmp=root/'private-tmp';private_target=root/'private-target'
for p in (private_home,private_codex,private_tmp,private_target):p.mkdir(mode=0o700,exist_ok=True)
env={'HOME':str(private_home),'CODEX_HOME':str(private_codex),'TMPDIR':str(private_tmp),'CARGO_TARGET_DIR':str(private_target),'PATH':'/usr/bin:/bin','RUST_LOG':'error','OPENAI_API_KEY':os.environ['OPENAI_API_KEY']}
proc=None
try:
    err=(logs/'predecessor.stderr.log').open('wb')
    proc=subprocess.Popen([str(host),'serve','--selected',str(selection_path)],env=env,cwd=root,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=err,start_new_session=True)
    def ready():
        if proc.poll() is not None: raise RuntimeError(f'predecessor exited {proc.returncode}')
        try:
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:s.settimeout(.1);s.connect(str(host_socket));return True
        except OSError:return False
    until(ready,20,'selected predecessor startup')
    until(lambda:(barrier/'native_before_spawn.reached').exists(),10,'pre-spawn hold')
    payload=json.loads(payload_path.read_text())
    admitted=exchange(host_socket,{'op':'task.submit.v1','payload':payload})
    assert admitted['status']=='admitted',admitted
    save(logs/'predecessor-admitted.json',admitted)
    until(lambda:len(query(life_root/'lifecycle.sqlite','SELECT delivery_id FROM inputs'))==1,10,'queued D0')
    assert not query(life_root/'lifecycle.sqlite','SELECT invocation_id FROM native_turns')
    predecessor_pid=proc.pid
    predecessor_starttime=(pathlib.Path(f'/proc/{proc.pid}/stat').read_text().rsplit(')',1)[1].split()[19])
    receipt_path=root/'startup-receipt.json';receipt_bytes=receipt_path.read_bytes();receipt=json.loads(receipt_bytes)
    save(logs/'predecessor-before.json',{'task':json.loads(query(task_db,'SELECT record_json FROM tasks')[0][0]),'sessions':query(life_root/'lifecycle.sqlite','SELECT session_id,allocation_state FROM native_sessions'),'inputs':query(life_root/'lifecycle.sqlite','SELECT delivery_id,state FROM inputs'),'turns':query(life_root/'lifecycle.sqlite','SELECT invocation_id FROM native_turns')})
    os.killpg(proc.pid,signal.SIGTERM);exit_code=proc.wait(timeout=20);err.close();proc=None
    assert exit_code==0,exit_code
    arm.unlink()
    owner=json.loads(query(task_db,'SELECT record_json FROM tasks')[0][0])
    subject=query(life_root/'lifecycle.sqlite','SELECT revision,semantic_revision,queue_revision FROM subjects WHERE subject_id=?',(config['lifecycle']['subject_id'],))[0]
    cursor=query(life_root/'lifecycle.sqlite','SELECT COALESCE(MAX(sequence),0) FROM journal')[0][0]
    session=query(life_root/'lifecycle.sqlite','SELECT profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256 FROM native_sessions WHERE session_id=?',(config['lifecycle']['native']['session_id'],))[0]
    next_root=root/'native-restart';next_root.mkdir(mode=0o700)
    next_config=json.loads(config_path.read_text())
    next_config['lifecycle']['native'].update({'root':str(next_root),'session_id':'session:s8:g0:restart','invocation_id':'invocation:s8:domain:g0:restart','attempt_id':'attempt:s8:domain:g0:restart'})
    next_config_path=root/'host-restart.json';save(next_config_path,next_config)
    declaration_path=root/'restart-declaration.json';next_selection_path=root/'selection-v3.json'
    next_selection=dict(selection,version=3,selection_id='selected-s10-semantic-epoch1',predecessor_selection_path=str(selection_path),restart_declaration_path=str(declaration_path),config_path=str(next_config_path),config_sha256=fsha(next_config_path))
    save(next_selection_path,next_selection)
    declaration={'version':1,'selection_sha256':fsha(next_selection_path),'predecessor_selection_sha256':fsha(selection_path),'predecessor_receipt_sha256':sha(receipt_bytes),'task':receipt['task'],'lifecycle':receipt['lifecycle'],'subject_id':config['lifecycle']['subject_id'],'context_generation':config['lifecycle']['context_generation'],'task_id':owner['task_id'],'task_revision':owner['revision'],'task_cursor_revision':owner['cursor_revision'],'predecessor_session_id':config['lifecycle']['native']['session_id'],'predecessor_epoch':0,'target_session_id':next_config['lifecycle']['native']['session_id'],'expected_subject_revision':subject[0],'expected_semantic_revision':subject[1],'expected_queue_revision':subject[2],'expected_journal_cursor':cursor,'profile_digest':session[0],'binary_sha256':session[1],'protocol_digest':session[2],'snapshot_id':session[3],'snapshot_manifest_sha256':session[4],'operator_ref':config['answer_authority_ref'],'cooperative_stop_asserted':True,'predecessor_host_pid':predecessor_pid,'predecessor_host_starttime_ticks':predecessor_starttime}
    save(declaration_path,declaration)
    outcome={'status':'prepared','selection_v3_path':str(next_selection_path),'selection_v3_sha256':fsha(next_selection_path),'declaration_sha256':fsha(declaration_path),'predecessor_receipt_sha256':sha(receipt_bytes),'predecessor_pid':predecessor_pid,'predecessor_exit_code':exit_code,'admitted':admitted,'queued_input_count':len(query(life_root/'lifecycle.sqlite','SELECT delivery_id FROM inputs')),'native_turn_count':len(query(life_root/'lifecycle.sqlite','SELECT invocation_id FROM native_turns'))}
    save(logs/'outcome.json',outcome)
    print(json.dumps(outcome,sort_keys=True))
except Exception as e:
    save(logs/'failure.json',{'error':repr(e),'pid':proc.pid if proc else None})
    if proc and proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:proc.wait(timeout=15)
        except subprocess.TimeoutExpired:proc.kill();proc.wait()
    sys.exit(1)
