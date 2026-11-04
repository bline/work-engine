#!/usr/bin/python3
"""Synthetic no-provider tool-schema probe. Run only in no-external-network bwrap."""
import hashlib, http.server, json, os, pathlib, re, shutil, socket, subprocess, threading, time
ROOT=pathlib.Path('/probe')
CAPTURE=ROOT/'requests.jsonl'
if pathlib.Path('/model-cache-source').is_file():
    shutil.copyfile('/model-cache-source', ROOT/'models_cache.json')
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n=int(self.headers.get('Content-Length','0'))
        body=self.rfile.read(min(n, 2_000_000))
        record={'path':self.path,'headers':{k:v for k,v in self.headers.items() if k.lower() not in ('authorization','api-key')},'body_sha256':hashlib.sha256(body).hexdigest()}
        try:
            x=json.loads(body);record['body_keys']=list(x);record['model']=x.get('model');record['tools']=x.get('tools');record['tool_choice']=x.get('tool_choice')
            context=json.dumps(x.get('input',[]))
            record['developer_advertises_functions_exec']='functions.exec' in context
            record['developer_mentions_corpus']='mcp__corpus' in context or 'corpus' in context
            record['nested_tool_names']=sorted(set(re.findall(r'tools\.([A-Za-z][A-Za-z0-9_]*)',context)))
            record['input_sha256']=hashlib.sha256(context.encode()).hexdigest()
            record['additional_tool_names']=[t.get('name') for item in x.get('input',[]) if isinstance(item,dict) and item.get('type')=='additional_tools' for t in item.get('tools',[]) if isinstance(t,dict)]
            record['is_followup']=any(isinstance(item,dict) and item.get('type')=='custom_tool_call_output' for item in x.get('input',[]))
            record['tool_output_excerpt']=[str(item.get('output',''))[:4000] for item in x.get('input',[]) if isinstance(item,dict) and item.get('type')=='custom_tool_call_output']
        except Exception as e:record['parse_error']=type(e).__name__
        with CAPTURE.open('a') as f:f.write(json.dumps(record)+'\n')
        if os.environ.get('S10_PROBE_FORCE_CALL') == '1' and not record.get('is_followup'):
            code='text(ALL_TOOLS.map(t => t.name)); text(await tools.mcp__corpus__list_artifacts({})); text(await tools.mcp__corpus__read_artifact({path: "prompt.md", offset: 0, length: 5}));'
            if os.environ.get('S10_PROBE_PATCH') == '1':
                code='text(await tools.apply_patch("*** Begin Patch\\n*** Add File: /probe/probe-write.txt\\n+synthetic\\n*** End Patch"));'
            events=[{'type':'response.created','response':{'id':'synthetic-r1'}},{'type':'response.output_item.done','item':{'type':'custom_tool_call','call_id':'synthetic-code-mode-1','namespace':'functions','name':'exec','input':code}},{'type':'response.completed','response':{'id':'synthetic-r1','usage':{'input_tokens':0,'output_tokens':0,'total_tokens':0}}}]
            reply=''.join('event: '+e['type']+'\ndata: '+json.dumps(e)+'\n\n' for e in events).encode()
            self.send_response(200);self.send_header('Content-Type','text/event-stream')
        else:
            reply=b'{"error":{"message":"synthetic endpoint deliberately stops before model response","type":"invalid_request_error"}}'
            self.send_response(400);self.send_header('Content-Type','application/json')
        self.send_header('Content-Length',str(len(reply)));self.end_headers();self.wfile.write(reply)
    def log_message(self,*args):pass
server=http.server.HTTPServer(('127.0.0.1',0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
sock=ROOT/'mcp.sock'
mcp=subprocess.Popen(['/usr/bin/socat','UNIX-LISTEN:'+str(sock)+',fork','EXEC:/usr/bin/python3 /app/corpus_mcp.py'],stdout=subprocess.DEVNULL,stderr=(ROOT/'mcp.stderr').open('wb'))
try:
    for _ in range(100):
        if sock.exists():break
        time.sleep(.01)
    assert sock.exists()
    codex='/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex'
    cmd=[codex,'--disable','shell_tool','--disable','unified_exec','--disable','view_image','--disable','image_generation','--disable','computer_use','--disable','browser_use','--disable','apps','--disable','plugins','--disable','multi_agent','--disable','goals','--disable','apply_patch_streaming_events','--disable','code_mode_host','-m',os.environ.get('S10_PROBE_MODEL','gpt-6-sol'),'-s','read-only','-a','never','--no-daemon','exec','--ignore-user-config','--ignore-rules','--strict-config','--skip-git-repo-check','-c','web_search="disabled"','-c','model_reasoning_effort=medium','-c','model_provider="synthetic"','-c','model_providers.synthetic.name="Synthetic"','-c','model_providers.synthetic.base_url="http://127.0.0.1:'+str(server.server_port)+'/v1"','-c','model_providers.synthetic.env_key="S10_FAKE_KEY"','-c','model_providers.synthetic.wire_api="responses"','-c','model_providers.synthetic.requires_openai_auth=false','-c','model_providers.synthetic.supports_websockets=false','-c','mcp_servers.corpus.default_tools_approval_mode="approve"','-c','mcp_servers.corpus.command="/usr/bin/socat"','-c','mcp_servers.corpus.args=["STDIO","UNIX-CONNECT:/probe/mcp.sock"]','--json','-o','/probe/final.md','-']
    if os.environ.get('S10_PROBE_CODE_MODE') == '1':
        i=cmd.index('code_mode_host'); del cmd[i-1:i+1]
        cmd[1:1]=['--enable','code_mode_only']
    if os.environ.get('S10_PROBE_UNIFIED') == '1':
        i=cmd.index('unified_exec'); del cmd[i-1:i+1]
    env={'HOME':'/probe','CODEX_HOME':'/probe','PATH':'/usr/bin:/bin','S10_FAKE_KEY':'synthetic-no-secret'}
    r=subprocess.run(cmd,input='Inspect the available synthetic corpus reader.\n',text=True,capture_output=True,timeout=12,cwd='/in',env=env)
    (ROOT/'codex.stdout').write_text(r.stdout);(ROOT/'codex.stderr').write_text(r.stderr)
    events=[json.loads(x) for x in r.stdout.splitlines() if x.startswith('{')]
    thread_ids=[e.get('thread_id') for e in events if e.get('type')=='thread.started']
    rollouts=list(ROOT.glob('sessions/**/rollout-*.jsonl'))
    print(json.dumps({'codex_exit':r.returncode,'request_count':len(CAPTURE.read_text().splitlines()) if CAPTURE.exists() else 0,
                      'synthetic_session_ids':thread_ids,'temporary_rollout_paths':[str(p) for p in rollouts],
                      'codex_stderr_tail':r.stderr[-600:]}))
finally:
    mcp.terminate()
    try:mcp.wait(timeout=2)
    except subprocess.TimeoutExpired:mcp.kill();mcp.wait()
    server.shutdown()
