from pathlib import Path
import json, subprocess, signal, sys
r=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/control')
a=json.loads((r/'listener7-argv.json').read_text())
c=json.loads((r/'listener7-consumed.json').read_text())
a[a.index('--since')+1]=str(c['next_since'])
a.extend(['--budget-cursor',c['next_budget_cursor']])
a[a.index('--timeout-seconds')+1]='3600'
(r/'listener8-argv.json').write_text(json.dumps(a,indent=2)+'\n')
p=subprocess.Popen(a,cwd='/home/bline/code/work-engine',stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1)
(r/'listener8-process.json').write_text(json.dumps({'pid':p.pid,'owner':'01a1d6e2-32ba-7701-9115-740dcb41af11','collector':'root foreground tool; natural boundary collection','binding':'fresh185k/220k context baseline after own native new_context; retained budget/board cursors','argv_file':str(r/'listener8-argv.json')},indent=2)+'\n')
try:
 with (r/'listener8.jsonl').open('w') as log:
  for line in p.stdout:
   log.write(line);log.flush()
   try:
    d=json.loads(line)
    print(json.dumps({'status':d.get('status'),'messages':[{'sequence':x.get('sequence'),'body':x.get('body')} for x in d.get('messages',[])],'control_events':d.get('control_events',[]),'budget_events':[{k:v for k,v in x.items() if k not in ['topic']} for x in d.get('budget_events',[])],'next_since':d.get('next_since')}),flush=True)
   except Exception: print(line,end='',flush=True)
 code=p.wait()
except KeyboardInterrupt:
 p.send_signal(signal.SIGINT);code=p.wait()
(r/'listener8-terminal.json').write_text(json.dumps({'pid':p.pid,'exit_code':code})+'\n')
sys.exit(code if code>=0 else 128-code)
