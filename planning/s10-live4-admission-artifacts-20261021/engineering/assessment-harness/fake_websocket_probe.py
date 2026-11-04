#!/usr/bin/python3
"""No-provider-response WebSocket capture for built-in OpenAI Codex route.

Run only in unshared-network bwrap with fake key, synthetic /in, private cache.
"""
import hashlib,json,os,pathlib,shutil,subprocess,threading,time
from websockets.sync.server import serve
ROOT=pathlib.Path('/probe')
if pathlib.Path('/model-cache-source').is_file(): shutil.copyfile('/model-cache-source',ROOT/'models_cache.json')
records=[]
def handler(ws):
    try:
        msg=ws.recv(timeout=5)
        body=json.loads(msg)
        records.append({'type':body.get('type'),'model':body.get('model'),'keys':list(body),
                        'tool_names':[t.get('name') for t in body.get('tools',[])],
                        'sha256':hashlib.sha256(msg.encode()).hexdigest()})
        # No response generation. A deliberate close ends this local probe.
    except Exception as e:
        records.append({'error':type(e).__name__})
with serve(handler,'127.0.0.1',0) as server:
    port=server.socket.getsockname()[1]
    t=threading.Thread(target=server.serve_forever,daemon=True);t.start()
    sock=ROOT/'mcp.sock'
    mcp=subprocess.Popen(['/usr/bin/socat','UNIX-LISTEN:'+str(sock)+',fork','EXEC:/usr/bin/python3 /app/corpus_mcp.py'],stdout=subprocess.DEVNULL,stderr=(ROOT/'mcp.stderr').open('wb'))
    try:
        for _ in range(100):
            if sock.exists():break
            time.sleep(.01)
        assert sock.exists()
        codex='/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex'
        cmd=[codex,'--disable','shell_tool','--disable','unified_exec','--disable','view_image','--disable','image_generation','--disable','computer_use','--disable','browser_use','--disable','apps','--disable','plugins','--disable','multi_agent','-m','gpt-6-sol','-s','read-only','-a','never','--no-daemon','exec','--ignore-user-config','--ignore-rules','--strict-config','--skip-git-repo-check','-c','web_search="disabled"','-c','model_reasoning_effort=medium','-c','openai_base_url="http://127.0.0.1:'+str(port)+'/v1"','-c','mcp_servers.corpus.command="/usr/bin/socat"','-c','mcp_servers.corpus.args=["STDIO","UNIX-CONNECT:/probe/mcp.sock"]','--json','-o','/probe/final.md','-']
        if os.environ.get('S10_PROBE_NO_DISABLE')=='1':
            j=cmd.index('-m');cmd=cmd[:1]+cmd[j:]
        env={'HOME':'/probe','CODEX_HOME':'/probe','PATH':'/usr/bin:/bin','OPENAI_API_KEY':'synthetic-no-secret'}
        try:r=subprocess.run(cmd,input='Inspect only the synthetic input.\n',text=True,capture_output=True,timeout=8,cwd='/in',env=env)
        except subprocess.TimeoutExpired as e:
            r=None
            (ROOT/'codex.stdout').write_bytes(e.stdout or b'')
            (ROOT/'codex.stderr').write_bytes(e.stderr or b'')
        if r:
            (ROOT/'codex.stdout').write_text(r.stdout);(ROOT/'codex.stderr').write_text(r.stderr)
        print(json.dumps({'codex_exit':r.returncode if r else 'timeout','websocket_requests':records,
                          'temporary_rollouts':[str(p) for p in ROOT.glob('sessions/**/rollout-*.jsonl')]}))
    finally:
        mcp.terminate()
        try:mcp.wait(timeout=2)
        except subprocess.TimeoutExpired:mcp.kill();mcp.wait()
        server.shutdown()
