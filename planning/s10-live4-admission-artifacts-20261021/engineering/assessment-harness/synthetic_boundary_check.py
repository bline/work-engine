#!/usr/bin/python3
"""No-secret, no-model mechanical check of the corpus reader namespace."""
import hashlib, json, subprocess, tempfile, time
from pathlib import Path
APP=Path(__file__).resolve().parent
with tempfile.TemporaryDirectory() as d:
    root=Path(d)
    for name in ('in','audit','socket'): (root/name).mkdir()
    (root/'in/prompt.md').write_bytes(b'hello')
    code='''import os,socket,subprocess
assert not os.path.exists('/state/auth.json')
assert not os.path.exists('/home/bline/code/work-engine')
assert not os.path.exists('/proc/1/root/home/bline/code/work-engine')
s=socket.socket(); assert s.connect_ex(('1.1.1.1',443))==101
subprocess.run(['/usr/bin/python3','/app.py'],check=True)
'''
    base=['/usr/bin/bwrap','--unshare-all','--die-with-parent','--ro-bind','/usr','/usr',
          '--symlink','usr/bin','/bin','--symlink','usr/lib','/lib','--symlink','usr/lib64','/lib64',
          '--dev','/dev','--proc','/proc','--tmpfs','/tmp','--ro-bind',str(root/'in'),'/in',
          '--ro-bind',str(APP/'corpus_mcp.py'),'/app.py','--bind',str(root/'audit'),'/audit',
          '--clearenv','--setenv','PATH','/usr/bin:/bin']
    requests=[{'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-06-18'}},
              {'jsonrpc':'2.0','id':2,'method':'tools/list'},
              {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'read_artifact','arguments':{'path':'/state/auth.json'}}},
              {'jsonrpc':'2.0','id':4,'method':'tools/call','params':{'name':'read_artifact','arguments':{'path':'prompt.md','offset':0,'length':5}}},
              {'jsonrpc':'2.0','id':5,'method':'resources/list'},
              {'jsonrpc':'2.0','id':6,'method':'resources/templates/list'},
              {'jsonrpc':'2.0','id':7,'method':'resources/read','params':{'uri':'secret://forbidden'}}]
    proc=subprocess.run(base+['--','/usr/bin/python3','-c',code],input='\n'.join(map(json.dumps,requests))+'\n',text=True,capture_output=True,timeout=10)
    assert proc.returncode==0,(proc.returncode,proc.stderr)
    rows=[json.loads(x) for x in proc.stdout.splitlines()]
    assert [t['name'] for t in rows[1]['result']['tools']]==['list_artifacts','read_artifact']
    assert rows[2]['result']['isError'] and 'auth' not in rows[2]['result']['content'][0]['text']
    assert json.loads(rows[3]['result']['content'][0]['text'])['utf8']=='hello'
    assert rows[4]['result']=={'resources': []}
    assert rows[5]['result']=={'resourceTemplates': []}
    assert rows[6]['error']['message']=='Method not found'
    audit=(root/'audit/reads.jsonl').read_bytes()
    socket_cmd=base[:-4]+['--bind',str(root/'socket'),'/socket','--clearenv','--setenv','PATH','/usr/bin:/bin','--','/usr/bin/socat','UNIX-LISTEN:/socket/mcp.sock,fork','EXEC:/usr/bin/python3 /app.py']
    server=subprocess.Popen(socket_cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    try:
        for _ in range(100):
            if (root/'socket/mcp.sock').exists():break
            time.sleep(.01)
        assert (root/'socket/mcp.sock').exists()
        client=subprocess.run(['/usr/bin/socat','STDIO','UNIX-CONNECT:'+str(root/'socket/mcp.sock')],
                              input=json.dumps(requests[1])+'\n',text=True,capture_output=True,timeout=5)
        assert client.returncode==0 and json.loads(client.stdout)['result']['tools'][0]['name']=='list_artifacts'
    finally:
        server.terminate()
        try:server.communicate(timeout=3)
        except subprocess.TimeoutExpired:server.kill();server.communicate()
    result={'schema':'s10-live4-synthetic-boundary-v1','at':time.time(),'scope':'no secret, no provider, no model, synthetic input only',
            'reader_source_sha256':hashlib.sha256((APP/'corpus_mcp.py').read_bytes()).hexdigest(),
            'reader_namespace_exit':proc.returncode,'reader_response_sha256':hashlib.sha256(proc.stdout.encode()).hexdigest(),
            'reader_stderr_sha256':hashlib.sha256(proc.stderr.encode()).hexdigest(),'audit_sha256':hashlib.sha256(audit).hexdigest(),
            'tools':['list_artifacts','read_artifact'],'outside_auth_repo_proc_unavailable':True,
            'external_connect_errno':101,'unknown_path_denied':True,'allowed_text_read_exact':True,
            'mcp_resources_empty':True,'mcp_resource_templates_empty':True,'mcp_resource_read_denied':True,
            'unix_socket_bridge':True,'limits':'Does not prove effective Codex advertised tools, real credential boundary or live assessment.'}
    out=APP/'synthetic-boundary-check-v1.json';out.write_text(json.dumps(result,indent=2)+'\n')
    print(out)
