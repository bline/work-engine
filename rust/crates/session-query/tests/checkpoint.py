"""Offline CLI comparison against the real session-events MCP adapter."""
import importlib.util, json, os, pathlib, subprocess, sys, tempfile
root = pathlib.Path(__file__).resolve().parents[4]
owner = root / 'skills/session-events/scripts/session_events.py'
helper = root / 'rust/crates/session-query/owner.py'
exe = root / 'rust/target/debug/work-engine-query'
spec = importlib.util.spec_from_file_location('session_events_test', owner)
m = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = m
spec.loader.exec_module(m)
with tempfile.TemporaryDirectory() as tmp:
    t = pathlib.Path(tmp)
    c, cl, state = (t / n for n in ('codex', 'claude', 'state'))
    path = c / 'sessions/2026/01/01/rollout-fixture-11111111-1111-4111-8111-111111111111.jsonl'
    path.parent.mkdir(parents=True)
    records = [{'type':'session_meta','timestamp':'2026-01-01T00:00:00Z','payload':{'cwd':'/fixture','timestamp':'2026-01-01T00:00:00Z'}},
               {'type':'response_item','timestamp':'2026-01-01T00:00:01Z','payload':{'type':'message','role':'user','content':[{'type':'input_text','text':'fixture message'}]}}]
    path.write_text(''.join(json.dumps(r)+'\n' for r in records))
    os.utime(path, (1767225600, 1767225600))
    profile = {'helper':str(helper),'owner':str(owner),'owner_sha256':'e4ceb6d7f5f7685f8c9c0ad84ce7ddd54fd1dc6bae8cc71f35be86373e53c716','codex_home':str(c),'claude_home':str(cl),'state_dir':str(state)}
    prof = t/'profile.json'; prof.write_text(json.dumps(profile))
    server = m.Server(m.Homes(c,cl,state), 'local-query', m.CallbackPolicy())
    for name, args in [('list_sessions', {}), ('read_session', {'session_id':'11111111-1111-4111-8111-111111111111'})]:
        expected = server.call_tool({'name':name,'arguments':args})['structuredContent']
        p = subprocess.run([str(exe),str(prof),name,'--json','-'],input=json.dumps(args),capture_output=True,text=True,check=True,timeout=40)
        actual = json.loads(p.stdout)
        assert actual == {'schema':'work-engine.session-query.v1','result':expected}, (actual, expected)
        print(json.dumps({'query':name,'result':actual},sort_keys=True))
    p = subprocess.run([str(exe),str(prof),'submit_session_request','--json','-'],input='{}',capture_output=True,text=True)
    assert p.returncode != 0 and 'invalid_query' in p.stderr
    p = subprocess.run([str(exe),str(prof),'list_sessions','--json','-'],input='{"mutation":true}',capture_output=True,text=True)
    assert p.returncode != 0 and 'invalid_arguments' in p.stderr
    print('rejected mutation name and field')
