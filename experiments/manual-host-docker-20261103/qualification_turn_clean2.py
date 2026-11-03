#!/usr/bin/env python3
"""One-off fixed-input retention qualification. Run inside the owned container."""
import json,pathlib,sys,time
sys.path.insert(0,'/opt/probe')
import codex_app,role_host
S=pathlib.Path('/state'); stage=sys.argv[1]
stages={
 'builder-1':('01a1a35a-712f-7ae0-9857-102fc749877d','gpt-6.1-sol','medium','This is a non-scored transport retention fixture. Do only this: write answer.txt containing 5 and verify its contents, using one shell command. Retain the private marker MAPLE-42 in your conversation. Reply with JSON {"stage":"initial","value":5}. No delegation or network. The value 5 is intentionally prescribed for a later correction.'),
 'reviewer-1':('01a1a35a-722c-7243-8153-7457db20f37e','gpt-6-astra','high','This is a non-scored transport retention fixture, not production review. Read answer.txt using one shell command. Its required value is 2+2. Do not write files. Retain the private finding marker CEDAR-17 in your conversation. Reply concise JSON with actual value, required value, and one finding. No delegation or network.'),
 'builder-2':('01a1a35a-712f-7ae0-9857-102fc749877d','gpt-6.1-sol','medium','Continue the same fixture. Reviewer found required 2+2 is 4. Correct answer.txt to contain 4 and run a Python assertion that its integer value equals 2+2, using one shell command. Reply concise JSON with corrected value and the private marker from your prior turn. No delegation or network.'),
 'reviewer-2':('01a1a35a-722c-7243-8153-7457db20f37e','gpt-6-astra','high','Rereview the same fixture and your retained finding. Read answer.txt using one shell command. Reply concise JSON with current value, whether your prior finding is resolved, and the private finding marker from your prior turn. Do not write files. No delegation or network.')}
sid,model,effort,prompt=stages[stage]
f=(S/(stage+'-events.jsonl')).open('x')
def record(kind,data):
 f.write(json.dumps({'at':time.time(),'kind':kind,'data':data})+'\n');f.flush()
c=codex_app.Connection(pathlib.Path('/state/codex/control.sock'),client_name='docker-retention-fixture',timeout=120)
def call(method,params):
 record('request',{'method':method,'params':params});r=c.call(method,params);record('response',{'method':method,'result':r})
 for n in c.notifications:record('notification',n)
 c.notifications.clear();return r
r=call('thread/resume',{'threadId':sid,'cwd':'/state/workspace','model':model,'approvalPolicy':'never','sandbox':'danger-full-access','config':{'model_reasoning_effort':effort,'service_tier':'default','features.apps':False,'apps._default.enabled':False}})
assert r['thread']['id']==sid and r['model']==model and r['serviceTier']=='default'
(S/(stage+'-realized.json')).write_text(json.dumps(r,indent=2)+'\n')
turn=call('turn/start',{'threadId':sid,'input':[{'type':'text','text':prompt}],'effort':effort})['turn'];tid=turn['id']
role_host.Ledger(S/'role-host').append({'event':'docker-fixture-turn','host':'codex','session_id':sid,'turn_id':tid,'parent_session_id':'01a19d59-05c0-7861-b8d5-b227b13a9f1e','at_epoch':time.time(),'stage':stage})
(S/(stage+'-binding.json')).write_text(json.dumps({'sid':sid,'turn_id':tid,'model':model,'effort':effort,'service_tier':'default','container':'we-manual-host-docker-20261103-clean2-retained'})+'\n')
end=time.monotonic()+180
completed=None
while time.monotonic()<end:
 msg=json.loads(c._recv_message());record('notification',msg)
 if msg.get('method')=='turn/completed' and msg.get('params',{}).get('turn',{}).get('id')==tid:
  completed=msg['params']['turn'];break
assert completed is not None,'selected turn did not complete'
(S/(stage+'-result.json')).write_text(json.dumps(completed,indent=2)+'\n')
c.close();f.close()
assert completed['status']=='completed',completed
print(json.dumps({'stage':stage,'sid':sid,'turn_id':tid,'status':completed['status']}))
