#!/usr/bin/env python3
"""One bounded real-root observation regression; synthetic thresholds, real usage."""
from pathlib import Path
import json,hashlib,subprocess,uuid,os,signal,shutil,time

ROOT=Path('/home/bline/.local/state/work-engine/experiment-control-live-20261023/context-enrollment-01')
def dump(p,x):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(x,indent=2)+'\n')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    ws=ROOT/'workspace';ws.mkdir(exist_ok=False);(ROOT/'records').mkdir()
    base=Path('/home/bline/.local/state/work-engine/experiment-control-live-20261023/calibration-01/pinned')
    for name in ['SKILL.md','references/coordinator.md','references/context-boundary.md','references/context-control.md','references/call-cost.md','references/anthropic-host.md']:
        relative=Path('skills/manual-orchestration')/name
        source=ROOT/'source'/relative
        if not source.exists():source=base/relative
        out=ws/relative;out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,out)
    (ws/'docs').mkdir();shutil.copy2('docs/codex-chatboard.md',ws/'docs/codex-chatboard.md')
    (ws/'docs/model-selection.md').write_text('Frozen regression configuration: claude-sonnet-5-5, medium. No model changes or child launches.\n')
    (ws/'tools').mkdir();shutil.copy2(ROOT/'source/skills/durable-state/scripts/codex_chatboard.py',ws/'tools/board.py');shutil.copy2(ROOT/'source/skills/durable-state/scripts/durable_state.py',ws/'tools/durable_state.py')
    shutil.copy2('/tmp/experiment-control-context-target/debug/work-engine-context-status',ws/'tools/context-status')
    sid=str(uuid.uuid4());dump(ROOT/'records/identity.json',dict(requested_session=sid))
    project=''.join(c if c.isalnum() else '-' for c in str(ws));native=Path.home()/'.claude/projects'/project/(sid+'.jsonl')
    identity_script=f'''import json,hashlib
from pathlib import Path
stream=Path({str(ROOT/'records/stream.jsonl')!r})
rows=[]
for line in stream.read_text().splitlines():
 try:rows.append(json.loads(line))
 except json.JSONDecodeError:pass
init=next((r for r in rows if r.get('type')=='system' and r.get('subtype')=='init'),None)
if not init or init.get('session_id')!={sid!r}:raise SystemExit('actual identity unavailable')
p=Path({str(native)!r})
print(json.dumps({{'session_id':init['session_id'],'host':'claude','rollout':str(p),'source_exists':p.exists(),'identity_evidence':'native init record verified by host helper','parent_collection_owner':'test root','control_binary':str(Path('tools/context-status').resolve()),'binary_sha256':{sha(ws/'tools/context-status')!r}}}))
'''
    (ws/'tools/identity.py').write_text(identity_script)
    board=ws/'board';subprocess.run(['git','init','-q',str(board)],check=True)
    boardcmd=['python3',str(ws/'tools/board.py'),'--repository',str(board),'wait','--since','0','--topic','all','--follow','--timeout-seconds','180']
    prompt='''Act as a root coordinator for this small local assignment. Read skills/manual-orchestration/SKILL.md and the references it requires for your role. Normal board coordination already has a host-owned board-only listener; its exact launch is in equipment.json. Complete a continuation note describing the actual owned observation route, any unavailable capabilities and the next work boundary, then compute 12*7 and report it with your status. No new agents or production actions.
Available equipment: python3 tools/identity.py returns your actual native identity/source binding; tools/context-status is the pinned telemetry executable; tools/board.py is the current board adapter; board/ is a disposable local board. Read equipment.json for their provenance and test parameters. The host owns collection and will terminate its preexisting board listener after your result. You may run bounded local commands and maintain continuation.md. Do not read unrelated paths or change the tools.
This is a short synthetic regression with a real native transcript. Planning threshold1000 and band floor2000 are deliberately lowered test policy, NOT provider reset limits or actual exhaustion. A pressure event should inform your decision, not force an unsafe or pointless handoff. Any listener you start must have a bounded lifetime; model-visible collection is through your own tool results. Record truthfully whether anything will still be listening after the tool returns.'''
    equipment=dict(board_listener_argv=boardcmd,board_listener_owner='test host',telemetry_binary='tools/context-status',telemetry_sha256=sha(ws/'tools/context-status'),identity_command='python3 tools/identity.py',planning_tokens=1000,band_floor_tokens=2000,policy='deliberately lowered regression thresholds, not reset prediction')
    dump(ws/'equipment.json',equipment);(ROOT/'records/input.txt').write_text(prompt)
    args=['claude','-p','--session-id',sid,'--model','claude-sonnet-5-5','--effort','medium','--safe-mode','--setting-sources','','--strict-mcp-config','--mcp-config','{"mcpServers":{}}','--tools','Bash,Read','--allowedTools','Bash,Read','--permission-mode','dontAsk','--no-chrome','--max-budget-usd','.75','--max-turns','16','--output-format','stream-json','--verbose','--system-prompt','You are a root coordinator in a disposable synthetic regression. Follow the supplied role contract. Use only the assigned workspace and your verified own transcript. No network, production board actions, subagent launches, or unrelated file access.']
    dump(ROOT/'records/manifest.json',dict(kind='separate_context_enrollment_regression',requested_model='claude-sonnet-5-5',effort='medium',max_budget_usd=.75,max_turns=16,deadline_seconds=180,argv=args,pins={str(p.relative_to(ws)):sha(p) for p in ws.rglob('*') if p.is_file() and '.git' not in p.parts},claim='real usage under artificial test levels; cooperative role observation only'))
    start=time.monotonic()
    with (ROOT/'records/board-stream.jsonl').open('w') as bout,(ROOT/'records/board-stderr.txt').open('w') as berr:
        bp=subprocess.Popen(boardcmd,stdout=bout,stderr=berr,start_new_session=True)
        try:
            with (ROOT/'records/stream.jsonl').open('w') as out,(ROOT/'records/stderr.txt').open('w') as err:
                p=subprocess.Popen(args,cwd=ws,stdin=subprocess.PIPE,stdout=out,stderr=err,text=True,start_new_session=True)
                try:p.communicate(prompt,timeout=180)
                except subprocess.TimeoutExpired:
                    os.killpg(p.pid,signal.SIGTERM)
                    try:p.wait(timeout=3)
                    except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
        finally:
            if bp.poll() is None:
                os.killpg(bp.pid,signal.SIGTERM)
                try:bp.wait(timeout=3)
                except subprocess.TimeoutExpired:os.killpg(bp.pid,signal.SIGKILL);bp.wait()
    if native.exists():shutil.copy2(native,ROOT/'records/native-transcript.jsonl')
    rows=[]
    for line in (ROOT/'records/stream.jsonl').read_text().splitlines():
        try:rows.append(json.loads(line))
        except json.JSONDecodeError:pass
    result=next((r for r in reversed(rows) if r.get('type')=='result'),{})
    dump(ROOT/'records/result.json',result)
    dump(ROOT/'records/collection.json',dict(model_exit=p.returncode,board_exit=bp.returncode,elapsed_seconds=time.monotonic()-start,processes_collected=True))
    print('Context regression collected',p.returncode)
if __name__=='__main__':main()
