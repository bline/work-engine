"""Fresh synthetic startup/profile refusal and task-disabled qualification."""
import json, os, pathlib, socket, subprocess, sys, time, urllib.request, urllib.error
import ah3_queries as joined
out=pathlib.Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
binary=sys.argv[2];producer=sys.argv[3]
fixture=pathlib.Path(subprocess.check_output([producer],text=True).strip())
query=joined.prepare(fixture)
with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
base={'version':1,'listen':f'127.0.0.1:{port}','mcp':False,'credential_file':str(fixture/'token'),'query_profile':str(query),'max_connections':8,'max_queries':2,'request_timeout_seconds':30}
task=dict(base,task_profile=str(fixture/'task-profile.json'),task_capability='synthetic-owner-v1')
cases={
 'task-and-command':dict(task,command_profile=str(fixture/'never-opened.json'),command_capability='offline-synthetic-v1'),
 'task-without-capability':{k:v for k,v in task.items() if k!='task_capability'},
 'capability-without-task':dict(base,task_capability='synthetic-owner-v1'),
 'unsupported-task-capability':dict(task,task_capability='live-provider-v1'),
 'no-owner-profile':{k:v for k,v in base.items() if k!='query_profile'},
 'unsupported-command-capability':dict(base,command_profile=str(fixture/'never-opened.json'),command_capability='live-provider-v1'),
}
for name,config in cases.items():
    path=fixture/(name+'.json');path.write_text(json.dumps(config));path.chmod(0o600)
    joined.capture(fixture,out,'before-'+name)
    run=subprocess.run([binary,str(path)],text=True,capture_output=True,timeout=10)
    joined.save(out,name,{'config':config,'returncode':run.returncode,'stdout':run.stdout,'stderr':run.stderr})
    assert run.returncode==1 and run.stderr.startswith('startup: '),(name,run)
# Actual query-only service explicitly refuses task and command capability.
path=fixture/'query-only.json';path.write_text(json.dumps(base));path.chmod(0o600)
joined.capture(fixture,out,'before-query-only')
proc=subprocess.Popen([binary,str(path)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
try:
    for _ in range(200):
        try:
            with socket.create_connection(('127.0.0.1',port),timeout=.1):break
        except OSError:
            assert proc.poll() is None;time.sleep(.05)
    else:raise AssertionError('query-only startup timeout')
    for op,args in [('project_owner_task',{'task_id':'fixture-task'}),('consume_owner_answer',{}),('read_owner_answer',{}),('read_session_operation',{})]:
        value=joined.http(base,op,args,'disabled')
        joined.save(out,'disabled-'+op,value)
        assert value.get('owner_result') is None and value.get('error'),value
    cli=fixture/'query-cli.json';cli.write_text(json.dumps({'version':1,'kind':'local-service','endpoint':base['listen'],'credential_file':base['credential_file'],'timeout_seconds':30}));cli.chmod(0o600)
    # Real CLI supported-profile boundary rejects task invocation.
    run=subprocess.run([os.environ['AH3_QUERY_BINARY'],str(cli),'project_owner_task','--json'],text=True,capture_output=True,timeout=40)
    joined.save(out,'cli-task-refusal',{'returncode':run.returncode,'output':json.loads(run.stdout)})
    assert run.returncode==1 and json.loads(run.stdout)['error']['code']=='invalid_query'
finally:
    proc.terminate()
    try:proc.wait(timeout=10)
    except subprocess.TimeoutExpired:proc.kill();proc.wait();raise
joined.save(out,'result',{'passed':True,'cases':list(cases),'query_only_task_and_command_disabled':True,'fixture':str(fixture)})
print('PASS startup profile rejection and query-only capability refusal')
